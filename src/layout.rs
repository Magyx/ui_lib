// TODO: text height can exede its parents fixed size, increasing the parents size
use std::cmp::{max, min};

use crate::{
    context::{Id, LayoutCtx, PaintCtx, PrepareCtx},
    event::{KeyState, LogicalKey, UiEventRef},
    focus::{Dir, ScopeId},
    model::{Color, Position, Size},
    primitive::Instance,
    theme::Env,
    widget::{Align, Axis, Length, Padding, Widget},
};

pub const ROOT_SEED: u64 = 0xCBF2_9CE4_8422_2325;

#[inline]
pub fn mix64(parent: Id, idx: usize) -> Id {
    let mut z =
        (parent ^ 0x9E37_79B9_7F4A_7C15u64) ^ (idx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15u64);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9u64);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EBu64);
    z ^ (z >> 31)
}

#[inline]
fn color_for_depth(depth: usize) -> (u8, u8, u8, u8) {
    const P: &[(u8, u8, u8, u8)] = &[
        (255, 66, 66, 220),  // red
        (255, 159, 28, 220), // orange
        (255, 214, 10, 220), // yellow
        (52, 199, 89, 220),  // green
        (48, 176, 199, 220), // teal
        (10, 132, 255, 220), // blue
        (94, 92, 230, 220),  // indigo
        (191, 90, 242, 220), // purple
    ];
    P[depth % P.len()]
}

pub fn run_layout<'a>(
    layout_engine: &mut LayoutEngine,
    ctx: &mut LayoutCtx<'a>,
    root: &mut dyn Widget,
    max_w: i32,
    max_h: i32,
) -> usize {
    crate::scope!("layout::run_layout");

    layout_engine.reset();
    // 1) Build engine graph from the widget tree
    let root_id = {
        crate::scope!("layout::build_tree");
        let env = ctx.theme.root_env();
        build_tree(layout_engine, ctx, root, ROOT_SEED, env)
    };

    // 2) Width phases
    {
        crate::scope!("layout::measure_width");
        layout_engine.measure_width(root_id);
    }
    {
        crate::scope!("layout::assign_width");
        layout_engine.assign_width(root_id, max_w);
    }

    // 3) Height phases
    let mut cursor = 0usize;
    {
        crate::scope!("layout::post_width_query");
        post_width_query(root, layout_engine, ctx, &mut cursor);
    }

    {
        crate::scope!("layout::measure_height");
        layout_engine.measure_height(root_id);
    }
    {
        crate::scope!("layout::assign_height");
        layout_engine.assign_height(root_id, max_h);
    }

    // 4) Place everything starting at origin
    {
        crate::scope!("layout::place");
        layout_engine.place(root_id, 0, 0);
    }

    root_id
}

fn build_tree<'a>(
    layout_engine: &mut LayoutEngine,
    ctx: &mut LayoutCtx<'a>,
    w: &mut dyn Widget,
    seed: u64,
    env: Env,
) -> usize {
    ctx.__set_id(seed);
    ctx.__set_env(env);
    let desc = w.layout(ctx);
    let i = layout_engine.create_node(desc.size, desc.layout_dir, desc.is_absolute);
    {
        let n = &mut layout_engine.nodes[i];
        n.id = seed;
        n.min = desc.min;
        n.max = desc.max;
        n.padding = desc.padding;
        n.spacing = desc.spacing;
        n.offset_pos = desc.offset_pos;
        n.clip_children = desc.clip_children;
        n.main_align = desc.main_align;
        n.cross_align = desc.cross_align;
    }

    let mut child_env = w.child_env(env, ctx.theme);
    if w.focus_trap() {
        child_env.focus_scope = seed;
    }
    let count = w.child_count();
    for y in 0..count {
        let child = w.child_mut(y);
        let child_seed = match child.key() {
            Some(k) => mix64(seed, k as usize),
            None => mix64(seed, y + 1),
        };
        let ci = build_tree(layout_engine, ctx, child, child_seed, child_env);
        layout_engine.add_child(i, ci);
    }

    i
}

fn post_width_query<'a>(
    w: &mut dyn Widget,
    eng: &mut LayoutEngine,
    ctx: &mut LayoutCtx<'a>,
    cursor: &mut usize,
) {
    let i = *cursor;

    let first = eng.nodes[i].first_child;
    if first.is_none() {
        ctx.__set_id(eng.nodes[i].id);
        if let Some(h) = w.min_height_for_width(ctx, eng.nodes[i].current_size.width) {
            let clamped = h.max(eng.nodes[i].min.height).min(eng.nodes[i].max.height);
            eng.nodes[i].min.height = clamped;
        }
    }

    *cursor += 1;
    let count = w.child_count();
    for i in 0..count {
        post_width_query(w.child_mut(i), eng, ctx, cursor);
    }
}

pub fn prepare_tree(w: &mut dyn crate::widget::Widget, ctx: &mut PrepareCtx, cursor: &mut usize) {
    crate::scope!("layout::prepare_tree");
    let env = ctx.theme.root_env();
    __prepare_tree(w, ctx, cursor, 0, 0, env);
}

fn __prepare_tree(
    w: &mut dyn crate::widget::Widget,
    ctx: &mut PrepareCtx,
    cursor: &mut usize,
    acc_tx: i32,
    acc_ty: i32,
    env: Env,
) {
    let id = *cursor;
    ctx.__set_data(id, acc_tx, acc_ty, env);

    w.prepare(ctx);
    *cursor += 1;

    let child_env = w.child_env(env, ctx.theme);
    let (dx, dy) = w.children_offset(ctx.view_state, ctx.id());
    let child_count = w.child_count();
    for i in 0..child_count {
        let child = w.child_mut(i);
        __prepare_tree(child, ctx, cursor, acc_tx + dx, acc_ty + dy, child_env);
    }

    ctx.__set_data(id, acc_tx, acc_ty, env);
    w.prepare_overlay(ctx);
}

pub fn handle_tree(
    w: &mut dyn crate::widget::Widget,
    ctx: &mut crate::context::EventCtx,
    cursor: &mut usize,
) {
    crate::scope!("layout::handle_tree");

    ctx.ui.focus.begin_walk();
    // Discrete keyboard traversal is captured centrally: a Tab press is not
    // owned by any single widget. It becomes a Move request resolved at the end
    // of the walk, once the ring is fully built.
    if let Some(UiEventRef::Key(k)) = ctx.event
        && k.state == KeyState::Pressed
        && k.logical_key == LogicalKey::Tab
    {
        let dir = if ctx.ui.modifiers.shift {
            Dir::Prev
        } else {
            Dir::Next
        };
        ctx.ui.focus.request_move(dir);
        ctx.ui.request_redraw();
    }

    __handle_tree(w, ctx, cursor, 0, 0, ROOT_SEED);

    ctx.ui.focus.end_walk();
}

fn __handle_tree(
    w: &mut dyn crate::widget::Widget,
    ctx: &mut crate::context::EventCtx,
    cursor: &mut usize,
    acc_tx: i32,
    acc_ty: i32,
    scope: ScopeId,
) {
    let id = *cursor;
    ctx.__set_data(id, acc_tx, acc_ty, scope);

    let node_id = ctx.id();

    if w.focusable() {
        ctx.ui.focus.register(node_id, scope);
    }
    let child_scope = if w.focus_trap() {
        ctx.ui.focus.note_trap(node_id);
        node_id
    } else {
        scope
    };

    w.handle(ctx);
    *cursor += 1;

    let (dx, dy) = w.children_offset(&mut ctx.ui.view_state, node_id);
    let child_count = w.child_count();
    for i in 0..child_count {
        let child = w.child_mut(i);
        __handle_tree(child, ctx, cursor, acc_tx + dx, acc_ty + dy, child_scope);
    }

    ctx.__set_data(id, acc_tx, acc_ty, scope);
    w.handle_after(ctx);
}

pub fn paint_tree(
    w: &mut dyn crate::widget::Widget,
    ctx: &mut PaintCtx,
    eng: &crate::layout::LayoutEngine,
    cursor: &mut usize,
    out: &mut Vec<Instance>,
    parent_clip: Option<[i32; 4]>,
) {
    crate::scope!("layout::paint_tree");
    let env = ctx.theme.root_env();
    __paint_tree(w, ctx, eng, cursor, out, parent_clip, 0, 0, 0, env);
}
#[allow(clippy::too_many_arguments)]
fn __paint_tree(
    w: &mut dyn crate::widget::Widget,
    ctx: &mut PaintCtx,
    eng: &crate::layout::LayoutEngine,
    cursor: &mut usize,
    out: &mut Vec<Instance>,
    parent_clip: Option<[i32; 4]>,
    acc_tx: i32,
    acc_ty: i32,
    depth: usize,
    env: Env,
) {
    let id = *cursor;
    ctx.__set_data(id, acc_tx, acc_ty, env);
    let n = eng.nodes[id];

    let mut clip = parent_clip;
    if n.clip_children {
        let mut r = [
            n.pos.x + acc_tx,
            n.pos.y + acc_ty,
            n.current_size.width,
            n.current_size.height,
        ];
        if let Some([px, py, pw, ph]) = clip {
            // intersect
            let x0 = px.max(r[0]);
            let y0 = py.max(r[1]);
            let x1 = (px + pw).min(r[0] + r[2]);
            let y1 = (py + ph).min(r[1] + r[3]);
            let w = (x1 - x0).max(0);
            let h = (y1 - y0).max(0);
            r = [x0, y0, w, h];
        }
        clip = Some(r);
    }

    let self_begin = out.len();
    w.paint(ctx, out);
    if let Some([cx, cy, cw, ch]) = clip {
        for inst in &mut out[self_begin..] {
            inst.add_clip(cx, cy, cw, ch);
        }
    }

    *cursor += 1;

    let node_id = eng.nodes[id].id;
    let mut child_env = w.child_env(env, ctx.theme);
    if w.focus_trap() {
        child_env.focus_scope = node_id;
    }
    let (dx, dy) = w.children_offset(ctx.view_state, node_id);
    let child_count = w.child_count();
    for i in 0..child_count {
        let child = w.child_mut(i);
        __paint_tree(
            child,
            ctx,
            eng,
            cursor,
            out,
            clip,
            acc_tx + dx,
            acc_ty + dy,
            depth + 1,
            child_env,
        );
    }

    let overlay_begin = out.len();
    ctx.__set_data(id, acc_tx, acc_ty, env);
    w.paint_overlay(ctx, out);

    if eng.debug {
        let (r, g, b, a) = color_for_depth(depth);
        let col = Color::rgba(r, g, b, a);
        let w = n.current_size.width.max(1) as f32;
        let h = n.current_size.height.max(1) as f32;
        let x = n.pos.x as f32;
        let y = n.pos.y as f32;
        let thickness = 1.0;

        // top
        out.push(Instance::ui(
            Position::new(x, y),
            Size::new(w, thickness),
            col,
        ));
        // bottom
        out.push(Instance::ui(
            Position::new(x, y + h - thickness),
            Size::new(w, thickness),
            col,
        ));
        // left
        out.push(Instance::ui(
            Position::new(x, y),
            Size::new(thickness, h),
            col,
        ));
        // right
        out.push(Instance::ui(
            Position::new(x + w - thickness, y),
            Size::new(thickness, h),
            col,
        ));
    }

    if let Some([cx, cy, cw, ch]) = clip {
        for inst in &mut out[overlay_begin..] {
            inst.add_clip(cx, cy, cw, ch);
        }
    }
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

    debug: bool,
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
        fn $measure_fn(&mut self, id: usize) {
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
        fn $assign_fn(&mut self, id: usize, parent_size: i32) {
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
    fn create_node(&mut self, size: Size<Length>, layout_dir: Axis, is_absolute: bool) -> usize {
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
    fn add_child(&mut self, parent: usize, child: usize) {
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
    fn reset(&mut self) {
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
    fn place(&mut self, id: usize, x: i32, y: i32) -> (i32, i32) {
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
