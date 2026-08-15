// TODO: text height can exede its parents fixed size, increasing the parents size
use std::cmp::{max, min};

use crate::{
    context::Id,
    model::{Position, Size},
};

#[derive(Clone, Copy, Debug, Default)]
pub enum Length {
    #[default]
    Fit,
    Fixed(i32),
    Grow,
    Weighted(f32),
}
impl Length {
    pub(crate) fn weight(self) -> Option<f32> {
        match self {
            Length::Grow => Some(1.0),
            Length::Weighted(w) => Some(w),
            _ => None,
        }
    }
}
impl<T> From<T> for Length
where
    T: Into<i32>,
{
    fn from(value: T) -> Self {
        Length::Fixed(value.into())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    /// Cross-axis only: fill the container's cross size. Resolved in the
    /// assign pass (not `place`) so the stretched child's subtree reflows.
    Stretch,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Axis {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Padding {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct __Node {
    pub size: Size<Length>,
    pub min: Size<i32>,
    pub max: Size<i32>,
    pub layout_dir: Axis,
    pub padding: Padding,
    pub spacing: i32,
    pub clip_children: bool,
    pub is_absolute: bool,
    pub offset_pos: Position<i32>,
    pub main_align: Align,
    pub cross_align: Align,

    pub(crate) id: Id,

    pub(crate) pos: Position<i32>,
    pub(crate) current_size: Size<i32>,
    pub(crate) content_size: Size<i32>,

    pub(crate) parent: Option<usize>,
    pub(crate) first_child: Option<usize>,
    pub(crate) next_sibling: Option<usize>,
}

impl Default for __Node {
    fn default() -> Self {
        Self {
            size: Default::default(),
            min: Default::default(),
            max: Size::splat(i32::MAX),
            layout_dir: Default::default(),
            padding: Default::default(),
            spacing: Default::default(),
            clip_children: Default::default(),
            is_absolute: Default::default(),
            offset_pos: Default::default(),
            main_align: Align::Start,
            cross_align: Align::Start,

            id: 0,

            pos: Default::default(),
            current_size: Default::default(),
            content_size: Default::default(),

            parent: Default::default(),
            first_child: Default::default(),
            next_sibling: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub size: Size<Length>,
    pub min: Size<i32>,
    pub max: Size<i32>,
    pub layout_dir: Axis,
    pub padding: Padding,
    pub spacing: i32,
    pub clip_children: bool,
    pub is_absolute: bool,
    pub offset_pos: Position<i32>,
    pub main_align: Align,
    pub cross_align: Align,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            size: Default::default(),
            min: Default::default(),
            max: Size::splat(i32::MAX),
            layout_dir: Default::default(),
            padding: Default::default(),
            spacing: Default::default(),
            clip_children: Default::default(),
            is_absolute: Default::default(),
            offset_pos: Default::default(),
            main_align: Align::Start,
            cross_align: Align::Start,
        }
    }
}

pub struct LayoutEngine {
    pub(crate) nodes: Vec<__Node>,
    pub(crate) node_count: usize,

    pub(crate) debug: bool,
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! impl_layout_pass {
    (
        measure_fn: $measure_fn:ident,
        assign_fn: $assign_fn:ident,
        dim: $dim:ident,
        pad_start: $pad_start:ident,
        pad_end: $pad_end:ident,
        main_axis: $main_axis:pat,
        cross_axis: $cross_axis:pat,
        stretch_check: $stretch_check:ident
    ) => {
        // Phase 1 / 3: measure minimal size
        pub(crate) fn $measure_fn(&mut self, id: usize) {
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
                    if !self.nodes[child].is_absolute {
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
            self.nodes[id].content_size.$dim = natural_val;
            if self.nodes[id].clip_children {
                self.nodes[id].current_size.$dim = natural_val.min(self.nodes[id].max.$dim);
            } else {
                self.nodes[id].current_size.$dim = natural_val;
                self.nodes[id].min.$dim = total_min;
            }
        }

        // Phase 2 / 4: assign sizes within available space
        pub(crate) fn $assign_fn(&mut self, id: usize, parent_size: i32) {
            let target_v = match self.nodes[id].size.$dim {
                Length::Grow | Length::Weighted(_) => parent_size,
                Length::Fixed(v) => v,
                Length::Fit if self.$stretch_check(id) => parent_size,
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

            // Main axis: one walk for count + base sum.
            let spacing = self.nodes[id].spacing;
            let (mut count, mut total_base) = (0, 0);
            let mut idx = Some(first);
            while let Some(child) = idx {
                if !self.nodes[child].is_absolute {
                    count += 1;
                    total_base += self.nodes[child].current_size.$dim;
                }
                idx = self.nodes[child].next_sibling;
            }

            let gaps = if count > 0 { (count - 1) * spacing } else { 0 };
            let inner_v = (target_v - pad.$pad_start - pad.$pad_end - gaps).max(0);
            let mut remaining = inner_v - total_base;

            if remaining < 0 {
                // shrink flexible children toward min
                let mut deficit = -remaining;
                while deficit > 0 {
                    let mut shrinkers = 0;
                    let mut idx = Some(first);
                    while let Some(child) = idx {
                        let n = &self.nodes[child];
                        if !n.is_absolute
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
                        if deficit > 0 && !n.is_absolute && !matches!(n.size.$dim, Length::Fixed(_))
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
            } else if remaining > 0 {
                // grow Grow children toward their max.
                while remaining > 0 {
                    let mut total_w = 0f32;
                    let mut idx = Some(first);
                    while let Some(child) = idx {
                        let n = &self.nodes[child];
                        if let Some(w) = n.size.$dim.weight()
                            && n.current_size.$dim < n.max.$dim
                        {
                            total_w += w;
                        }
                        idx = n.next_sibling;
                    }
                    if total_w <= 0.0 {
                        break;
                    }
                    let budget = remaining;
                    let mut used = 0;
                    let mut idx = Some(first);
                    while let Some(child) = idx {
                        let next = self.nodes[child].next_sibling;
                        let n = &mut self.nodes[child];
                        if remaining > 0
                            && let Some(w) = n.size.$dim.weight()
                        {
                            let addable = n.max.$dim - n.current_size.$dim;
                            if addable > 0 {
                                let share = (budget as f32 * (w / total_w)).floor() as i32;
                                let amt = min(addable, share);
                                if amt > 0 {
                                    n.current_size.$dim += amt;
                                    used += amt;
                                    remaining -= amt;
                                }
                            }
                        }
                        idx = next;
                    }
                    if used == 0 {
                        let mut idx = Some(first);
                        while let Some(child) = idx {
                            let next = self.nodes[child].next_sibling;
                            let n = &mut self.nodes[child];
                            if n.size.$dim.weight().is_some() && n.current_size.$dim < n.max.$dim {
                                n.current_size.$dim += 1;
                                remaining -= 1; // You added this to height, but not width in your original snippet!
                                used = 1; // Keeping it here makes both passes safer and mathematically correct.
                                break;
                            }
                            idx = next;
                        }
                        if used == 0 {
                            break;
                        }
                    }
                }
            }

            // Recurse: flow children into their resolved size, absolute children
            // into the full inner size.
            let mut idx = Some(first);
            while let Some(child) = idx {
                let next = self.nodes[child].next_sibling;
                let v = if self.nodes[child].is_absolute {
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

impl LayoutEngine {
    pub fn new() -> Self {
        LayoutEngine {
            nodes: Vec::with_capacity(1024),
            node_count: 0,

            debug: false,
        }
    }
    pub(crate) fn create_node(
        &mut self,
        size: Size<Length>,
        layout_dir: Axis,
        is_absolute: bool,
    ) -> usize {
        let i = self.node_count;
        let node = __Node {
            size,
            layout_dir,
            is_absolute,
            ..Default::default()
        };
        if i < self.nodes.len() {
            self.nodes[i] = node;
        } else {
            self.nodes.push(node);
        }
        self.node_count += 1;
        i
    }
    pub(crate) fn add_child(&mut self, parent: usize, child: usize) {
        self.nodes[child].parent = Some(parent);
        if self.nodes[parent].first_child.is_none() {
            self.nodes[parent].first_child = Some(child);
        } else {
            let mut cur = self.nodes[parent].first_child.unwrap();
            while let Some(next) = self.nodes[cur].next_sibling {
                cur = next;
            }
            self.nodes[cur].next_sibling = Some(child);
        }
    }
    pub(crate) fn reset(&mut self) {
        self.node_count = 0;
    }

    pub fn toggle_debug(&mut self) {
        self.debug = !self.debug;
    }

    /// True when this node is a flow child of a horizontal parent whose
    /// `cross_align` is `Stretch` — i.e. its *height* should fill the parent.
    fn stretched_by_horizontal_parent(&self, id: usize) -> bool {
        if self.nodes[id].is_absolute {
            return false;
        }
        match self.nodes[id].parent {
            Some(p) => {
                matches!(self.nodes[p].layout_dir, Axis::Horizontal)
                    && self.nodes[p].cross_align == Align::Stretch
            }
            None => false,
        }
    }

    /// True when this node is a flow child of a vertical parent whose
    /// `cross_align` is `Stretch` — i.e. its *width* should fill the parent.
    fn stretched_by_vertical_parent(&self, id: usize) -> bool {
        if self.nodes[id].is_absolute {
            return false;
        }
        match self.nodes[id].parent {
            Some(p) => {
                matches!(self.nodes[p].layout_dir, Axis::Vertical)
                    && self.nodes[p].cross_align == Align::Stretch
            }
            None => false,
        }
    }

    impl_layout_pass!(
        measure_fn: measure_width,
        assign_fn: assign_width,
        dim: width,
        pad_start: left,
        pad_end: right,
        main_axis: Axis::Horizontal,
        cross_axis: Axis::Vertical,
        stretch_check: stretched_by_vertical_parent
    );

    impl_layout_pass!(
        measure_fn: measure_height,
        assign_fn: assign_height,
        dim: height,
        pad_start: top,
        pad_end: bottom,
        main_axis: Axis::Vertical,
        cross_axis: Axis::Horizontal,
        stretch_check: stretched_by_horizontal_parent
    );

    // Phase 5: place all nodes (compute positions)
    pub(crate) fn place(&mut self, id: usize, x: i32, y: i32) -> (i32, i32) {
        fn cross_offset(align: Align, inner_cross: i32, child_cross: i32) -> i32 {
            match align {
                Align::Center => ((inner_cross - child_cross) / 2).max(0),
                Align::End => (inner_cross - child_cross).max(0),
                _ => 0,
            }
        }

        // Set this nodes position
        self.nodes[id].pos = Position::new(x, y);
        if let Some(child_idx) = self.nodes[id].first_child {
            let layout_dir = self.nodes[id].layout_dir;
            let pad = self.nodes[id].padding;
            let spacing = self.nodes[id].spacing;
            let main_align = self.nodes[id].main_align;
            let cross_align = self.nodes[id].cross_align;

            // Content-box origin (inside padding).
            let base_x = x + pad.left;
            let base_y = y + pad.top;
            let inner_w = (self.nodes[id].current_size.width - pad.left - pad.right).max(0);
            let inner_h = (self.nodes[id].current_size.height - pad.top - pad.bottom).max(0);
            let (inner_main, inner_cross) = match layout_dir {
                Axis::Horizontal => (inner_w, inner_h),
                Axis::Vertical => (inner_h, inner_w),
            };

            // Flow (non-absolute) children participate in alignment.
            let mut content_main = 0;
            let mut n = 0;
            let mut idx = Some(child_idx);
            while let Some(child) = idx {
                if !self.nodes[child].is_absolute {
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
            let (lead, gap) = match main_align {
                // `Stretch` is cross-axis only; on the main axis it means Start.
                Align::Start | Align::Stretch => (0, spacing),
                Align::Center => (free / 2, spacing),
                Align::End => (free, spacing),
                Align::SpaceBetween => {
                    if n > 1 {
                        (0, spacing + free / (n - 1))
                    } else {
                        (0, spacing)
                    }
                }
                Align::SpaceAround => {
                    if n > 0 {
                        let unit = free / n;
                        (unit / 2, spacing + unit)
                    } else {
                        (0, spacing)
                    }
                }
                Align::SpaceEvenly => {
                    let unit = free / (n + 1);
                    (unit, spacing + unit)
                }
            };

            let mut cursor_main = lead;
            let mut idx = Some(child_idx);
            while let Some(child) = idx {
                idx = self.nodes[child].next_sibling;
                if self.nodes[child].is_absolute {
                    let offx = self.nodes[child].offset_pos.x;
                    let offy = self.nodes[child].offset_pos.y;
                    self.place(child, base_x + offx, base_y + offy);
                } else {
                    let child_w = self.nodes[child].current_size.width;
                    let child_h = self.nodes[child].current_size.height;
                    match layout_dir {
                        Axis::Horizontal => {
                            let cross = cross_offset(cross_align, inner_cross, child_h);
                            self.place(child, base_x + cursor_main, base_y + cross);
                            cursor_main += child_w + gap;
                        }
                        Axis::Vertical => {
                            let cross = cross_offset(cross_align, inner_cross, child_w);
                            self.place(child, base_x + cross, base_y + cursor_main);
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
mod tests {
    use super::*;

    #[test]
    fn node_vec_grows_beyond_initial_capacity() {
        let mut engine = LayoutEngine::new();
        let initial_cap = engine.nodes.capacity();

        // Force growth past the initial allocation
        for _ in 0..initial_cap + 1 {
            engine.create_node(Size::default(), Axis::default(), false);
        }

        assert!(
            engine.nodes.capacity() > initial_cap,
            "vec should have reallocated"
        );
        assert_eq!(
            engine.node_count,
            initial_cap + 1,
            "all nodes should be accounted for"
        );

        // Verify reset preserves capacity
        let grown_cap = engine.nodes.capacity();
        engine.reset();
        assert_eq!(engine.node_count, 0);
        assert_eq!(
            engine.nodes.capacity(),
            grown_cap,
            "reset should not shrink capacity"
        );
    }
}
