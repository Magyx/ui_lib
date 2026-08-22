// TODO: text height can exede its parents fixed size, increasing the parents size
use std::{
    cmp::{max, min},
    num::NonZeroU32,
    ops::{Deref, DerefMut, Index, IndexMut},
    range::Range,
};

use crate::{
    context::Id,
    model::{Position, Rect, Size},
};

mod models;
pub use models::{Align, Align2, Axis, Edges, Length, Main, Node, Placement};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeIdx(NonZeroU32);
impl NodeIdx {
    #[inline]
    pub fn new(index: usize) -> Self {
        let val = NonZeroU32::new((index + 1) as u32).expect("Node index overflow");
        NodeIdx(val)
    }

    #[inline]
    pub fn as_usize(self) -> usize {
        (self.0.get() - 1) as usize
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct __Node {
    pub desc: Node,

    pub(crate) id: Id,
    pub(crate) pos: Position<i32>,
    pub(crate) current_size: Size<i32>,
    pub(crate) natural_size: Size<i32>,

    pub(crate) parent: Option<NodeIdx>,
    pub(crate) first_child: Option<NodeIdx>,
    pub(crate) last_child: Option<NodeIdx>,
    pub(crate) next_sibling: Option<NodeIdx>,
}
impl Deref for __Node {
    type Target = Node;
    fn deref(&self) -> &Self::Target {
        &self.desc
    }
}
impl DerefMut for __Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.desc
    }
}

// TODO: Might want to look into making this a soa of topology, style and computed values
pub struct Tree {
    pub(crate) nodes: Vec<__Node>,
    pub(crate) node_count: usize,
}
impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}
impl Tree {
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(capacity),
            node_count: 0,
        }
    }

    pub(crate) fn create_node(&mut self, desc: Node, seed: Id) -> NodeIdx {
        let i = self.node_count;
        let node_idx = NodeIdx::new(i);
        let node = __Node {
            desc,
            id: seed,
            ..Default::default()
        };

        if i < self.nodes.len() {
            self.nodes[i] = node;
        } else {
            self.nodes.push(node);
        }
        self.node_count += 1;
        node_idx
    }

    pub(crate) fn add_child(&mut self, parent: NodeIdx, child: NodeIdx) {
        self[child].parent = Some(parent);

        if let Some(last) = self[parent].last_child {
            self[last].next_sibling = Some(child);
        } else {
            self[parent].first_child = Some(child);
        }

        self[parent].last_child = Some(child);
    }

    pub(crate) fn reset(&mut self) {
        self.node_count = 0;
    }

    pub fn find_by_id(&self, target_id: Id) -> Option<NodeIdx> {
        self.nodes[..self.node_count]
            .iter()
            .position(|n| n.id == target_id)
            .map(NodeIdx::new)
    }
}
impl Index<NodeIdx> for Tree {
    type Output = __Node;

    #[inline]
    fn index(&self, index: NodeIdx) -> &Self::Output {
        &self.nodes[index.as_usize()]
    }
}
impl IndexMut<NodeIdx> for Tree {
    #[inline]
    fn index_mut(&mut self, index: NodeIdx) -> &mut Self::Output {
        &mut self.nodes[index.as_usize()]
    }
}
impl Index<Range<NodeIdx>> for Tree {
    type Output = [__Node];

    fn index(&self, index: Range<NodeIdx>) -> &Self::Output {
        &self.nodes[index.start.as_usize()..index.end.as_usize()]
    }
}

pub struct LayoutEngine {
    pub nodes: Tree,

    pub(crate) fill_scratch: Vec<NodeIdx>,
    pub(crate) debug: bool,
}
impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}
impl LayoutEngine {
    pub fn new() -> Self {
        LayoutEngine {
            nodes: Tree::default(),
            fill_scratch: Vec::new(),
            debug: false,
        }
    }

    pub(crate) fn create_node(&mut self, desc: Node, seed: Id) -> NodeIdx {
        self.nodes.create_node(desc, seed)
    }

    pub(crate) fn add_child(&mut self, parent: NodeIdx, child: NodeIdx) {
        self.nodes.add_child(parent, child);
    }

    pub(crate) fn reset(&mut self) {
        self.nodes.reset();
    }

    pub fn toggle_debug(&mut self) {
        self.debug = !self.debug;
    }
}

macro_rules! impl_layout_pass {
    (
        measure_fn: $measure_fn:ident,
        assign_fn: $assign_fn:ident,
        fill_fn: $fill_fn:ident,
        dim: $dim:ident,
        pad_start: $pad_start:ident,
        pad_end: $pad_end:ident,
        main_axis: $main_axis:pat,
        cross_axis: $cross_axis:pat
    ) => {
        // Phase 1 / 3: measure minimal size
        pub(crate) fn $measure_fn(&mut self, id: NodeIdx) {
            let base_val = match self.nodes[id].size.$dim {
                Length::Fixed(v) => {
                    self.nodes[id].min.$dim = max(self.nodes[id].min.$dim, v);
                    self.nodes[id].max.$dim = min(self.nodes[id].max.$dim, v);
                    v
                }
                _ => 0,
            };

            let min_val = if let Some(first) = self.nodes[id].first_child {
                let layout_dir = self.nodes[id].layout_dir;
                let pad = self.nodes[id].padding;
                let spacing = self.nodes[id].spacing;

                let (mut total, mut count, mut max_cross) = (0, 0, 0);
                let mut idx = Some(first);
                while let Some(child) = idx {
                    self.$measure_fn(child);
                    if !self.nodes[child].placement.is_absolute() {
                        let cv = self.nodes[child].min.$dim;
                        total += cv;
                        max_cross = max(max_cross, cv);
                        count += 1;
                    }
                    idx = self.nodes[child].next_sibling;
                }

                match layout_dir {
                    $main_axis => {
                        let gaps = if count > 0 { (count - 1) * spacing } else { 0 };
                        total + pad.$pad_start + pad.$pad_end + gaps
                    }
                    $cross_axis => max_cross + pad.$pad_start + pad.$pad_end,
                }
            } else {
                self.nodes[id].min.$dim
            };

            let total_min = max(min_val, self.nodes[id].min.$dim);
            let natural_val = max(total_min, base_val);
            self.nodes[id].natural_size.$dim = natural_val;
            if self.nodes[id].clip_children {
                self.nodes[id].current_size.$dim = natural_val.min(self.nodes[id].max.$dim);
            } else {
                self.nodes[id].current_size.$dim = natural_val;
                self.nodes[id].min.$dim = total_min;
            }
        }

        /// Size the `Fill` children in `first`'s sibling chain from `pool`,
        /// in proportion to their weights.
        ///
        /// A child whose share would violate its own min or max is frozen at
        /// that bound and taken out of the pool; the remaining children then
        /// re-split what is left. Each pass freezes at least one child, so
        /// this settles in at most one pass per flexible child.
        ///
        /// Note a `Fill` child's min is its *content* min (set in the measure
        /// pass), unless it clips — so text still refuses to shrink past the
        /// point where it would be cut off, and the exact weight ratio gives
        /// way to that.
        pub(crate) fn $fill_fn(&mut self, first: NodeIdx, pool: i32) {
            let mut open = std::mem::take(&mut self.fill_scratch);
            open.clear();

            let mut idx = Some(first);
            while let Some(child) = idx {
                let n = &self.nodes[child];
                if !n.placement.is_absolute() && n.size.$dim.weight().is_some() {
                    open.push(child);
                }
                idx = n.next_sibling;
            }

            let mut pool = pool;
            while !open.is_empty() {
                let mut total_w = 0f32;
                for &c in open.iter() {
                    total_w += self.nodes[c].size.$dim.weight().unwrap_or(0.0);
                }
                if total_w <= 0.0 {
                    break;
                }

                // Freeze every child whose fair share breaks a bound, then
                // re-split the reduced pool among whoever is left.
                let mut froze = false;
                let mut i = 0;
                while i < open.len() {
                    let c = open[i];
                    let w = self.nodes[c].size.$dim.weight().unwrap_or(0.0);
                    let share = (pool as f32 * (w / total_w)).floor() as i32;
                    let lo = self.nodes[c].min.$dim;
                    let hi = self.nodes[c].max.$dim;
                    if share < lo || share > hi {
                        let bound = share.clamp(lo, hi);
                        self.nodes[c].current_size.$dim = bound;
                        pool -= bound;
                        open.remove(i);
                        froze = true;
                    } else {
                        i += 1;
                    }
                }
                if froze {
                    continue;
                }

                // Nobody is clamped: hand out the shares and spread the
                // rounding remainder one pixel at a time.
                let mut assigned = 0;
                for &c in open.iter() {
                    let w = self.nodes[c].size.$dim.weight().unwrap_or(0.0);
                    let share = (pool as f32 * (w / total_w)).floor() as i32;
                    self.nodes[c].current_size.$dim = share;
                    assigned += share;
                }
                let mut rem = pool - assigned;
                for &c in open.iter() {
                    if rem <= 0 {
                        break;
                    }
                    if self.nodes[c].current_size.$dim < self.nodes[c].max.$dim {
                        self.nodes[c].current_size.$dim += 1;
                        rem -= 1;
                    }
                }
                break;
            }

            open.clear();
            self.fill_scratch = open;
        }

        // Phase 2 / 4: assign sizes within available space
        pub(crate) fn $assign_fn(&mut self, id: NodeIdx, parent_size: i32) {
            let target_v = match self.nodes[id].size.$dim {
                // Both edges pinned: the size is derived from the parent,
                // whatever the `Length` says. This is what makes a `Fit` child
                // stretch under `Edges::horizontal(..)`.
                _ if matches!(
                    self.nodes[id].placement,
                    Placement::Absolute { edges, .. }
                        if edges.$pad_start().is_some() && edges.$pad_end().is_some()
                ) =>
                {
                    let Placement::Absolute { edges, .. } = self.nodes[id].placement else {
                        unreachable!()
                    };
                    let (a, b) = (edges.$pad_start().unwrap(), edges.$pad_end().unwrap());
                    (parent_size - a - b).max(0)
                }
                Length::Fill(_) => parent_size,
                Length::Fixed(v) => v,
                Length::Fit
                    if self.parent_fills_cross(id)
                        && matches!(self.parent_axis(id), Some($cross_axis)) =>
                {
                    parent_size
                }
                Length::Fit => self.nodes[id].current_size.$dim,
            };
            let target_v = target_v
                .min(self.nodes[id].max.$dim)
                .max(self.nodes[id].min.$dim);
            self.nodes[id].current_size.$dim = target_v;

            let Some(first) = self.nodes[id].first_child else {
                return;
            };
            let pad = self.nodes[id].padding;

            // Cross axis: every child gets the full inner size; no distribution.
            if let $cross_axis = self.nodes[id].layout_dir {
                let inner_v = (target_v - pad.$pad_start - pad.$pad_end).max(0);
                let mut idx = Some(first);
                while let Some(child) = idx {
                    self.$assign_fn(child, inner_v);
                    idx = self.nodes[child].next_sibling;
                }
                return;
            }

            // Main axis: one walk for count + base sum. `Fill` children are
            // deliberately excluded from the base — their size comes from the
            // pool below, in proportion to weight, rather than from growing
            // their content outward. That is what makes `Fill(2)` twice
            // `Fill(1)` regardless of what either one contains.
            let spacing = self.nodes[id].spacing;
            let (mut count, mut total_base, mut fills) = (0, 0, 0);
            let mut idx = Some(first);
            while let Some(child) = idx {
                if !self.nodes[child].placement.is_absolute() {
                    count += 1;
                    if self.nodes[child].size.$dim.weight().is_some() {
                        fills += 1;
                    } else {
                        total_base += self.nodes[child].current_size.$dim;
                    }
                }
                idx = self.nodes[child].next_sibling;
            }

            let gaps = if count > 0 { (count - 1) * spacing } else { 0 };
            let inner_v = (target_v - pad.$pad_start - pad.$pad_end - gaps).max(0);

            if fills > 0 {
                self.$fill_fn(first, (inner_v - total_base).max(0));
            }

            // Re-sum now that the `Fill` children have been sized; anything
            // still over budget is taken back by the shrink loop below.
            let (mut total_used, mut idx) = (0, Some(first));
            while let Some(child) = idx {
                if !self.nodes[child].placement.is_absolute() {
                    total_used += self.nodes[child].current_size.$dim;
                }
                idx = self.nodes[child].next_sibling;
            }
            let remaining = inner_v - total_used;

            if remaining < 0 {
                // shrink flexible children toward min
                let mut deficit = -remaining;
                while deficit > 0 {
                    let mut shrinkers = 0;
                    let mut idx = Some(first);
                    while let Some(child) = idx {
                        let n = &self.nodes[child];
                        if !n.placement.is_absolute()
                            && n.current_size.$dim > n.min.$dim
                            && !matches!(n.size.$dim, Length::Fixed(_))
                        {
                            shrinkers += 1;
                        }
                        idx = n.next_sibling;
                    }
                    if shrinkers == 0 {
                        break;
                    }
                    let reduce_each = (deficit / shrinkers).max(1);
                    let mut used = 0;
                    let mut idx = Some(first);
                    while let Some(child) = idx {
                        let next = self.nodes[child].next_sibling;
                        let n = &mut self.nodes[child];
                        if deficit > 0 && !n.placement.is_absolute() && !matches!(n.size.$dim, Length::Fixed(_))
                        {
                            let reducible = n.current_size.$dim - n.min.$dim;
                            let amt = min(reducible, reduce_each);
                            if amt > 0 {
                                n.current_size.$dim -= amt;
                                used += amt;
                                deficit -= amt;
                            }
                        }
                        idx = next;
                    }
                    if used == 0 {
                        break;
                    }
                }
            }

            // Recurse: flow children into their resolved size, absolute children
            // into the full inner size.
            let mut idx = Some(first);
            while let Some(child) = idx {
                let next = self.nodes[child].next_sibling;
                let v = if self.nodes[child].placement.is_absolute() {
                    inner_v
                } else {
                    self.nodes[child].current_size.$dim
                };
                self.$assign_fn(child, v);
                idx = next;
            }
        }
    };
}

#[inline]
fn cross_offset(align: Align, inner_cross: i32, child_cross: i32) -> i32 {
    (((inner_cross - child_cross) as f32 * align.get()).floor()).max(0.0) as i32
}

impl LayoutEngine {
    /// True when this node is a flow child of a parent with `fill_cross` set.
    /// The caller pairs this with an axis check, since filling only applies
    /// across the parent's layout direction.
    fn parent_fills_cross(&self, id: NodeIdx) -> bool {
        if self.nodes[id].placement.is_absolute() {
            return false;
        }
        match self.nodes[id].parent {
            Some(p) => self.nodes[p].fill_cross,
            None => false,
        }
    }

    #[inline]
    fn parent_axis(&self, id: NodeIdx) -> Option<Axis> {
        self.nodes[id].parent.map(|p| self.nodes[p].layout_dir)
    }

    impl_layout_pass!(
        measure_fn: measure_width,
        assign_fn: assign_width,
        fill_fn: resolve_fill_width,
        dim: width,
        pad_start: left,
        pad_end: right,
        main_axis: Axis::Horizontal,
        cross_axis: Axis::Vertical
    );

    impl_layout_pass!(
        measure_fn: measure_height,
        assign_fn: assign_height,
        fill_fn: resolve_fill_height,
        dim: height,
        pad_start: top,
        pad_end: bottom,
        main_axis: Axis::Vertical,
        cross_axis: Axis::Horizontal
    );

    // Phase 5: place all nodes (compute positions)
    pub(crate) fn place(&mut self, id: NodeIdx, x: i32, y: i32) -> (i32, i32) {
        // Set this nodes position
        self.nodes[id].pos = Position::new(x, y);
        if let Some(child_idx) = self.nodes[id].first_child {
            let layout_dir = self.nodes[id].layout_dir;
            let pad = self.nodes[id].padding;
            let spacing = self.nodes[id].spacing;
            let main = self.nodes[id].main;
            let cross = self.nodes[id].cross;

            // Content box: the node's rect inside its padding, then shifted by
            // `children_offset` (scroll, and later any subtree translation).
            let offset = self.nodes[id].children_offset;
            let content = Rect::from_parts(Position::new(x, y), self.nodes[id].current_size)
                .inset(pad)
                .translate(offset);
            let (base_x, base_y) = (content.x, content.y);
            let (inner_w, inner_h) = (content.w, content.h);
            let (inner_main, inner_cross) = match layout_dir {
                Axis::Horizontal => (inner_w, inner_h),
                Axis::Vertical => (inner_h, inner_w),
            };

            // Flow (non-absolute) children participate in alignment.
            let mut content_main = 0;
            let mut n = 0;
            let mut idx = Some(child_idx);
            while let Some(child) = idx {
                if !self.nodes[child].placement.is_absolute() {
                    content_main += match layout_dir {
                        Axis::Horizontal => self.nodes[child].current_size.width,
                        Axis::Vertical => self.nodes[child].current_size.height,
                    };
                    n += 1;
                }
                idx = self.nodes[child].next_sibling;
            }

            let base_spacing = if n > 1 { (n - 1) * spacing } else { 0 };
            let free = (inner_main - content_main - base_spacing).max(0);
            let (lead, gap) = match main {
                Main::At(a) => ((free as f32 * a.get()).floor() as i32, spacing),
                Main::Between => {
                    if n > 1 {
                        (0, spacing + free / (n - 1))
                    } else {
                        (0, spacing)
                    }
                }
                Main::Around => {
                    if n > 0 {
                        let unit = free / n;
                        (unit / 2, spacing + unit)
                    } else {
                        (0, spacing)
                    }
                }
                Main::Evenly => {
                    let unit = free / (n + 1);
                    (unit, spacing + unit)
                }
            };

            let mut cursor_main = lead;
            let mut idx = Some(child_idx);
            while let Some(child) = idx {
                idx = self.nodes[child].next_sibling;
                if let Placement::Absolute {
                    anchor,
                    origin,
                    offset,
                    edges,
                } = self.nodes[child].placement
                {
                    let size = self.nodes[child].current_size;
                    let mut p = content.place(size, anchor, origin);

                    // A pinned edge overrides the anchor on that axis. When
                    // both are pinned the size was already derived in assign,
                    // so the leading edge alone gives the right position.
                    if let Some(l) = edges.left() {
                        p.x = content.x + l;
                    } else if let Some(r) = edges.right() {
                        p.x = content.right() - r - size.width;
                    }
                    if let Some(t) = edges.top() {
                        p.y = content.y + t;
                    } else if let Some(b) = edges.bottom() {
                        p.y = content.bottom() - b - size.height;
                    }

                    self.place(child, p.x + offset.x, p.y + offset.y);
                } else {
                    let child_w = self.nodes[child].current_size.width;
                    let child_h = self.nodes[child].current_size.height;
                    let align = self.nodes[child].cross_self.unwrap_or(cross);
                    match layout_dir {
                        Axis::Horizontal => {
                            let off = cross_offset(align, inner_cross, child_h);
                            self.place(child, base_x + cursor_main, base_y + off);
                            cursor_main += child_w + gap;
                        }
                        Axis::Vertical => {
                            let off = cross_offset(align, inner_cross, child_w);
                            self.place(child, base_x + off, base_y + cursor_main);
                            cursor_main += child_h + gap;
                        }
                    }
                }
            }
        }
        (
            self.nodes[id].current_size.width,
            self.nodes[id].current_size.height,
        )
    }
}

#[cfg(test)]
mod placement_tests {
    use super::*;
    use crate::model::Inset;

    const PARENT: i32 = 200;

    /// Lay out one absolutely placed child of `size` inside a 200x200 parent
    /// with `pad` padding, and return where it landed.
    fn place_one(pad: Inset, child: Size<Length>, placement: Placement) -> Rect {
        let mut e = LayoutEngine::new();
        let root = e.create_node(
            Node {
                size: Size::splat(Length::Fixed(PARENT)),
                layout_dir: Axis::Horizontal,
                padding: pad,
                ..Default::default()
            },
            0,
        );
        let c = e.create_node(
            Node {
                size: child,
                placement,
                ..Default::default()
            },
            1,
        );
        e.add_child(root, c);
        e.measure_width(root);
        e.assign_width(root, PARENT);
        e.measure_height(root);
        e.assign_height(root, PARENT);
        e.place(root, 0, 0);
        Rect::from_parts(e.nodes[c].pos, e.nodes[c].current_size)
    }

    fn at(anchor: Align2, origin: Align2) -> Placement {
        Placement::Absolute {
            anchor,
            origin,
            offset: Position::new(0, 0),
            edges: Edges::NONE,
        }
    }

    fn fixed(w: i32, h: i32) -> Size<Length> {
        Size::new(Length::Fixed(w), Length::Fixed(h))
    }

    #[test]
    fn top_left_on_top_left_is_the_old_behaviour() {
        let r = place_one(Inset::ZERO, fixed(40, 20), Placement::ABSOLUTE);
        assert_eq!(r, Rect::new(0, 0, 40, 20));
    }

    /// The headline case: the child's centre on the parent's centre.
    #[test]
    fn centre_on_centre() {
        let r = place_one(
            Inset::ZERO,
            fixed(40, 20),
            at(Align2::CENTER, Align2::CENTER),
        );
        assert_eq!(r, Rect::new(80, 90, 40, 20));
    }

    /// Centring must not depend on the sizes sharing parity.
    #[test]
    fn centre_is_parity_independent() {
        for w in [1, 2, 3, 39, 40, 41] {
            let r = place_one(
                Inset::ZERO,
                fixed(w, 10),
                at(Align2::CENTER, Align2::CENTER),
            );
            assert_eq!(r.x, (PARENT - w) / 2, "w={w}");
        }
    }

    /// A badge whose own centre sits on the parent's corner hangs outside it.
    #[test]
    fn corner_anchor_with_centre_origin_goes_negative() {
        let r = place_one(
            Inset::ZERO,
            fixed(20, 20),
            at(Align2::TOP_RIGHT, Align2::CENTER),
        );
        assert_eq!(r, Rect::new(190, -10, 20, 20));
    }

    #[test]
    fn every_anchor_with_top_left_origin() {
        let cases = [
            (Align2::TOP_LEFT, (0, 0)),
            (Align2::TOP_CENTER, (100, 0)),
            (Align2::TOP_RIGHT, (200, 0)),
            (Align2::CENTER_LEFT, (0, 100)),
            (Align2::CENTER, (100, 100)),
            (Align2::BOTTOM_RIGHT, (200, 200)),
        ];
        for (a, (x, y)) in cases {
            let r = place_one(Inset::ZERO, fixed(10, 10), at(a, Align2::TOP_LEFT));
            assert_eq!((r.x, r.y), (x, y), "anchor {a:?}");
        }
    }

    /// Anchoring is against the content box, so padding insets it.
    /// This locks in the choice recorded in the write-up.
    #[test]
    fn anchors_against_the_content_box() {
        let pad = Inset::new(10, 20, 30, 40);
        let r = place_one(pad, fixed(10, 10), at(Align2::TOP_LEFT, Align2::TOP_LEFT));
        assert_eq!((r.x, r.y), (10, 20));
        let r = place_one(
            pad,
            fixed(10, 10),
            at(Align2::BOTTOM_RIGHT, Align2::BOTTOM_RIGHT),
        );
        assert_eq!((r.x, r.y), (PARENT - 30 - 10, PARENT - 40 - 10));
    }

    #[test]
    fn offset_applies_after_anchoring() {
        let p = Placement::Absolute {
            anchor: Align2::TOP_RIGHT,
            origin: Align2::TOP_RIGHT,
            offset: Position::new(-8, 8),
            edges: Edges::NONE,
        };
        let r = place_one(Inset::ZERO, fixed(20, 20), p);
        assert_eq!((r.x, r.y), (PARENT - 20 - 8, 8));
    }

    fn with_edges(child: Size<Length>, edges: Edges) -> Rect {
        place_one(
            Inset::ZERO,
            child,
            Placement::Absolute {
                anchor: Align2::CENTER,
                origin: Align2::CENTER,
                offset: Position::new(0, 0),
                edges,
            },
        )
    }

    /// One edge fixes the position on that axis and beats the anchor; the
    /// other axis still anchors.
    #[test]
    fn single_edge_overrides_the_anchor() {
        let r = with_edges(fixed(20, 20), Edges::NONE.with_left(12));
        assert_eq!(r.x, 12);
        assert_eq!(r.y, 90, "y still centred");

        let r = with_edges(fixed(20, 20), Edges::NONE.with_right(12));
        assert_eq!(r.x, PARENT - 12 - 20);
    }

    /// Both edges derive the size, so even a `Fit` child stretches.
    #[test]
    fn both_edges_stretch_a_fit_child() {
        let r = with_edges(Size::splat(Length::Fit), Edges::horizontal(16));
        assert_eq!(r.x, 16);
        assert_eq!(r.w, PARENT - 32);
    }

    #[test]
    fn all_edges_fill_minus_the_inset() {
        let r = with_edges(Size::splat(Length::Fit), Edges::all(24));
        assert_eq!(r, Rect::new(24, 24, PARENT - 48, PARENT - 48));
    }

    /// A pinned `0` must not read as "unset".
    #[test]
    fn zero_pin_is_flush_not_unset() {
        let r = with_edges(fixed(20, 20), Edges::NONE.with_left(0));
        assert_eq!(
            r.x, 0,
            "left(0) pins flush, it does not fall through to the anchor"
        );
    }

    /// Absolute children contribute nothing to a `Fit` parent's size and
    /// nothing to the fill pool.
    #[test]
    fn absolute_children_do_not_size_the_parent() {
        let mut e = LayoutEngine::new();
        let root = e.create_node(
            Node {
                size: Size::splat(Length::Fit),
                layout_dir: Axis::Horizontal,
                ..Default::default()
            },
            0,
        );
        let flow = e.create_node(
            Node {
                size: fixed(30, 10),
                ..Default::default()
            },
            1,
        );
        let abs = e.create_node(
            Node {
                size: fixed(500, 500),
                placement: Placement::ABSOLUTE,
                ..Default::default()
            },
            2,
        );
        e.add_child(root, flow);
        e.add_child(root, abs);
        e.measure_width(root);
        assert_eq!(
            e.nodes[root].current_size.width, 30,
            "the 500px absolute child must not widen a Fit parent"
        );
    }
}

#[cfg(test)]
mod fill_tests {
    use super::*;

    /// A flexible child with `content` px of irreducible content.
    fn fill(w: f32, content: i32) -> Node {
        Node {
            size: Size::new(Length::Fill(w), Length::Fit),
            min: Size::new(content, 0),
            ..Default::default()
        }
    }

    fn fixed(v: i32) -> Node {
        Node {
            size: Size::new(Length::Fixed(v), Length::Fit),
            ..Default::default()
        }
    }

    /// Lay out a horizontal row of `width` px and return the children's
    /// resolved widths.
    fn widths(width: i32, kids: Vec<Node>) -> Vec<i32> {
        let mut e = LayoutEngine::new();
        let root = e.create_node(
            Node {
                size: Size::new(Length::Fixed(width), Length::Fit),
                layout_dir: Axis::Horizontal,
                ..Default::default()
            },
            0,
        );
        let ids: Vec<NodeIdx> = kids
            .into_iter()
            .enumerate()
            .map(|(i, k)| {
                let c = e.create_node(k, i as u64 + 1);
                e.add_child(root, c);
                c
            })
            .collect();
        e.measure_width(root);
        e.assign_width(root, width);
        ids.into_iter()
            .map(|c| e.nodes[c].current_size.width)
            .collect()
    }

    #[test]
    fn weight_sets_the_ratio_when_empty() {
        assert_eq!(
            widths(300, vec![fill(2.0, 0), fill(1.0, 0)]),
            vec![200, 100]
        );
        assert_eq!(
            widths(400, vec![fill(3.0, 0), fill(1.0, 0)]),
            vec![300, 100]
        );
    }

    /// The regression this change exists for: content used to be reserved
    /// before the split, so a `Fill(2)` holding 60px came out 220:80.
    #[test]
    fn content_does_not_skew_the_ratio() {
        assert_eq!(
            widths(300, vec![fill(2.0, 60), fill(1.0, 0)]),
            vec![200, 100]
        );
    }

    /// Two equal weights used to diverge whenever one held content.
    #[test]
    fn equal_weights_stay_equal() {
        assert_eq!(
            widths(300, vec![fill(1.0, 100), fill(1.0, 0)]),
            vec![150, 150]
        );
    }

    #[test]
    fn fixed_siblings_are_reserved_first() {
        assert_eq!(
            widths(300, vec![fixed(100), fill(1.0, 0), fill(1.0, 0)]),
            vec![100, 100, 100]
        );
    }

    /// A child that cannot fit its share freezes at its minimum, and the
    /// others re-split what is left rather than overflowing the row.
    #[test]
    fn content_min_wins_over_the_ratio() {
        assert_eq!(
            widths(300, vec![fill(1.0, 200), fill(1.0, 0)]),
            vec![200, 100]
        );
    }

    #[test]
    fn max_freezes_and_redistributes() {
        let capped = Node {
            size: Size::new(Length::Fill(1.0), Length::Fit),
            max: Size::new(50, i32::MAX),
            ..Default::default()
        };
        assert_eq!(widths(300, vec![capped, fill(1.0, 0)]), vec![50, 250]);
    }

    /// Flooring loses up to one pixel per child; the remainder is handed out
    /// so the row is filled exactly.
    #[test]
    fn rounding_remainder_is_distributed() {
        let out = widths(100, vec![fill(1.0, 0), fill(1.0, 0), fill(1.0, 0)]);
        assert_eq!(out.iter().sum::<i32>(), 100);
        assert_eq!(out, vec![34, 33, 33]);
    }

    /// Whatever the weights, the children must never exceed the container.
    #[test]
    fn never_overflows_the_container() {
        for width in 1..200 {
            for w in 1..6 {
                let out = widths(width, vec![fill(w as f32, 0), fill(1.0, 0)]);
                assert!(
                    out.iter().sum::<i32>() <= width,
                    "width={width} w={w} -> {out:?}"
                );
            }
        }
    }

    #[test]
    fn no_fill_children_leaves_slack_for_alignment() {
        assert_eq!(widths(300, vec![fixed(40), fixed(60)]), vec![40, 60]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cross_offset` must reproduce the arms the old `Align` enum had, for
    /// every input including the overflow cases the `.max(0)` clamps.
    #[test]
    fn cross_offset_matches_legacy_arms() {
        for inner in 0..48 {
            for child in 0..48 {
                assert_eq!(cross_offset(Align::START, inner, child), 0);
                assert_eq!(
                    cross_offset(Align::CENTER, inner, child),
                    ((inner - child) / 2).max(0),
                    "center inner={inner} child={child}"
                );
                assert_eq!(
                    cross_offset(Align::END, inner, child),
                    (inner - child).max(0),
                    "end inner={inner} child={child}"
                );
            }
        }
    }

    /// An oversized child pins to the leading edge rather than centring, so
    /// the start of its content stays reachable.
    #[test]
    fn cross_offset_clamps_overflow() {
        assert_eq!(cross_offset(Align::CENTER, 50, 200), 0);
        assert_eq!(cross_offset(Align::END, 50, 200), 0);
    }

    #[test]
    fn node_vec_grows_beyond_initial_capacity() {
        let mut tree = Tree::default();
        let initial_cap = tree.nodes.capacity();

        // Force growth past the initial allocation
        for _ in 0..initial_cap + 1 {
            tree.create_node(Node::default(), 0);
        }

        assert!(
            tree.nodes.capacity() > initial_cap,
            "vec should have reallocated"
        );
        assert_eq!(
            tree.node_count,
            initial_cap + 1,
            "all nodes should be accounted for"
        );

        // Verify reset preserves capacity
        let grown_cap = tree.nodes.capacity();
        tree.reset();
        assert_eq!(tree.node_count, 0);
        assert_eq!(
            tree.nodes.capacity(),
            grown_cap,
            "reset should not shrink capacity"
        );
    }
}
