// TODO: text height can exede its parents fixed size, increasing the parents size
use std::cmp::{max, min};

use crate::{
    context::{Id, LayoutCtx, PaintCtx, PrepareCtx},
    event::{KeyState, LogicalKey, UiEventRef},
    focus::{Dir, ScopeId},
    model::{Color, Position, Size},
    primitive::Instance,
    theme::Env,
    widget::{Axis, Length, Padding, Widget},
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

pub fn run_layout<'a, M>(
    layout_engine: &mut LayoutEngine,
    ctx: &mut LayoutCtx<'a, M>,
    root: &mut dyn Widget<M>,
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

fn build_tree<'a, M>(
    layout_engine: &mut LayoutEngine,
    ctx: &mut LayoutCtx<'a, M>,
    w: &mut dyn Widget<M>,
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

fn post_width_query<'a, M>(
    w: &mut dyn Widget<M>,
    eng: &mut LayoutEngine,
    ctx: &mut LayoutCtx<'a, M>,
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

pub fn prepare_tree<M>(
    w: &mut dyn crate::widget::Widget<M>,
    ctx: &mut PrepareCtx,
    cursor: &mut usize,
) {
    crate::scope!("layout::prepare_tree");
    let env = ctx.theme.root_env();
    __prepare_tree(w, ctx, cursor, 0, 0, env);
}

fn __prepare_tree<M>(
    w: &mut dyn crate::widget::Widget<M>,
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

pub fn handle_tree<M>(
    w: &mut dyn crate::widget::Widget<M>,
    ctx: &mut crate::context::EventCtx<M>,
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

fn __handle_tree<M>(
    w: &mut dyn crate::widget::Widget<M>,
    ctx: &mut crate::context::EventCtx<M>,
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

pub fn paint_tree<M>(
    w: &mut dyn crate::widget::Widget<M>,
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
fn __paint_tree<M>(
    w: &mut dyn crate::widget::Widget<M>,
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

    // Phase 1: measure minimal widths
    fn measure_width(&mut self, id: usize) {
        if let Some(child_idx) = self.nodes[id].first_child {
            let layout_dir = self.nodes[id].layout_dir;
            let pad = self.nodes[id].padding;
            let spacing = self.nodes[id].spacing;
            let min_w: i32;
            if let Axis::Horizontal = layout_dir {
                // Sum childrens min widths (skip absolute children)
                let mut total = 0;
                let mut count = 0;
                let mut idx = Some(child_idx);
                while let Some(child) = idx {
                    self.measure_width(child);
                    if !self.nodes[child].is_absolute {
                        total += self.nodes[child].min.width;
                        count += 1;
                    }
                    idx = self.nodes[child].next_sibling;
                }
                let gaps = if count > 0 { (count - 1) * spacing } else { 0 };
                min_w = total + pad.left + pad.right + gaps;
            } else {
                // Vertical: take max child width
                let mut max_child_w = 0;
                let mut idx = Some(child_idx);
                while let Some(child) = idx {
                    self.measure_width(child);
                    if !self.nodes[child].is_absolute {
                        max_child_w = max(max_child_w, self.nodes[child].min.width);
                    }
                    idx = self.nodes[child].next_sibling;
                }
                min_w = max_child_w + pad.left + pad.right;
            }

            let total_min_w = max(min_w, self.nodes[id].min.width);
            let base_w = match self.nodes[id].size.width {
                Length::Fixed(w) => {
                    self.nodes[id].min.width = max(self.nodes[id].min.width, w);
                    self.nodes[id].max.width = min(self.nodes[id].max.width, w);
                    w
                }
                _ => 0,
            };

            let natural_w = max(total_min_w, base_w);
            self.nodes[id].content_size.width = natural_w;
            self.nodes[id].current_size.width = natural_w;
            if !self.nodes[id].clip_children {
                self.nodes[id].min.width = total_min_w;
            }
        } else {
            let base_w = match self.nodes[id].size.width {
                Length::Fixed(w) => {
                    self.nodes[id].min.width = max(self.nodes[id].min.width, w);
                    self.nodes[id].max.width = min(self.nodes[id].max.width, w);
                    w
                }
                _ => 0,
            };
            let natural_w = max(self.nodes[id].min.width, base_w);
            self.nodes[id].content_size.width = natural_w;
            self.nodes[id].current_size.width = natural_w.min(self.nodes[id].max.width);
        }
    }

    // Phase 2: assign widths with available space
    fn assign_width(&mut self, id: usize, parent_width: i32) {
        if let Some(child_idx) = self.nodes[id].first_child {
            let layout_dir = self.nodes[id].layout_dir;
            let pad = self.nodes[id].padding;
            let spacing = self.nodes[id].spacing;

            let target_w = match self.nodes[id].size.width {
                Length::Grow => parent_width,
                Length::Fixed(w) => w,
                Length::Fit => self.nodes[id].current_size.width,
            };
            let target_w = max(target_w, self.nodes[id].min.width).min(self.nodes[id].max.width);
            self.nodes[id].current_size.width = target_w;
            let inner_w = (target_w
                - pad.left
                - pad.right
                - if let Axis::Horizontal = layout_dir {
                    let mut count = 0;
                    let mut idx = Some(child_idx);
                    while let Some(child) = idx {
                        if !self.nodes[child].is_absolute {
                            count += 1;
                        }
                        idx = self.nodes[child].next_sibling;
                    }
                    if count > 0 { (count - 1) * spacing } else { 0 }
                } else {
                    0
                })
            .max(0);
            if let Axis::Horizontal = layout_dir {
                // **Horizontal container**: distribute inner width among children
                let mut children = Vec::new();
                let mut idx = Some(child_idx);
                while let Some(child) = idx {
                    if !self.nodes[child].is_absolute {
                        children.push(child);
                    }
                    idx = self.nodes[child].next_sibling;
                }

                // Calculate each childs base width (from measure)
                let count = children.len();
                let mut allocated: Vec<i32> = Vec::with_capacity(count);
                let mut total_base = 0;
                for &child in &children {
                    let base = self.nodes[child].current_size.width;
                    allocated.push(base);
                    total_base += base;
                }
                let mut remaining = inner_w - total_base;

                // If not enough space, shrink proportionally above min
                if remaining < 0 {
                    let mut deficit = -remaining;
                    while deficit > 0 {
                        let shrinkables: Vec<usize> = children
                            .iter()
                            .enumerate()
                            .filter(|(j, child)| {
                                allocated[*j] > self.nodes[**child].min.width
                                    && !matches!(self.nodes[**child].size.width, Length::Fixed(_))
                            })
                            .map(|(j, _)| j)
                            .collect();
                        if shrinkables.is_empty() {
                            break;
                        }
                        let reduce_each = (deficit / shrinkables.len() as i32).max(1);
                        let mut used = 0;
                        for &j in &shrinkables {
                            let child = children[j];
                            let reducible = allocated[j] - self.nodes[child].min.width;
                            let reduce_amt = min(reducible, reduce_each);
                            if reduce_amt > 0 {
                                allocated[j] -= reduce_amt;
                                used += reduce_amt;
                                deficit -= reduce_amt;
                                if deficit <= 0 {
                                    break;
                                }
                            }
                        }
                        if used == 0 {
                            break;
                        }
                    }
                    remaining = 0;
                }

                // If extra space, distribute to Grow children
                if remaining > 0 {
                    while remaining > 0 {
                        let growables: Vec<usize> = children
                            .iter()
                            .enumerate()
                            .filter(|(j, child)| {
                                matches!(self.nodes[**child].size.width, Length::Grow)
                                    && allocated[*j] < self.nodes[**child].max.width
                            })
                            .map(|(j, _)| j)
                            .collect();
                        if growables.is_empty() {
                            break;
                        }
                        let add_each = (remaining / growables.len() as i32).max(1);
                        let mut used = 0;
                        for &j in &growables {
                            let child = children[j];
                            let addable = self.nodes[child].max.width - allocated[j];
                            let add_amt = min(addable, add_each);
                            if add_amt > 0 {
                                allocated[j] += add_amt;
                                used += add_amt;
                                remaining -= add_amt;
                                if remaining <= 0 {
                                    break;
                                }
                            }
                        }
                        if used == 0 {
                            break;
                        }
                    }
                }

                for (j, &child) in children.iter().enumerate() {
                    self.assign_width(child, allocated[j]);
                }

                let mut idx = Some(child_idx);
                while let Some(child) = idx {
                    if self.nodes[child].is_absolute {
                        self.assign_width(child, inner_w);
                    }
                    idx = self.nodes[child].next_sibling;
                }
            } else {
                // **Vertical container**: give all children the same inner width
                let mut idx = Some(child_idx);
                while let Some(child) = idx {
                    self.assign_width(child, inner_w);
                    idx = self.nodes[child].next_sibling;
                }
            }
        } else {
            let target_w = match self.nodes[id].size.width {
                Length::Grow => parent_width,
                Length::Fixed(w) => w,
                Length::Fit => self.nodes[id].current_size.width,
            };
            let final_w = max(target_w, self.nodes[id].min.width).min(self.nodes[id].max.width);
            self.nodes[id].current_size.width = final_w;
        }
    }

    // Phase 3: measure minimal heights (after widths are set)
    fn measure_height(&mut self, id: usize) {
        if let Some(child_idx) = self.nodes[id].first_child {
            let layout_dir = self.nodes[id].layout_dir;
            let pad = self.nodes[id].padding;
            let spacing = self.nodes[id].spacing;
            let min_h = match layout_dir {
                Axis::Horizontal => {
                    let mut max_child_h = 0;
                    let mut idx = Some(child_idx);
                    while let Some(child) = idx {
                        self.measure_height(child);
                        if !self.nodes[child].is_absolute {
                            max_child_h = max(max_child_h, self.nodes[child].min.height);
                        }
                        idx = self.nodes[child].next_sibling;
                    }
                    max_child_h + pad.top + pad.bottom
                }
                Axis::Vertical => {
                    let mut total = 0;
                    let mut count = 0;
                    let mut idx = Some(child_idx);
                    while let Some(child) = idx {
                        self.measure_height(child);
                        if !self.nodes[child].is_absolute {
                            total += self.nodes[child].min.height;
                            count += 1;
                        }
                        idx = self.nodes[child].next_sibling;
                    }
                    let gaps = if count > 0 { (count - 1) * spacing } else { 0 };
                    total + pad.top + pad.bottom + gaps
                }
            };
            let total_min_h = max(min_h, self.nodes[id].min.height);
            let base_h = match self.nodes[id].size.height {
                Length::Fixed(h) => {
                    self.nodes[id].min.height = max(self.nodes[id].min.height, h);
                    self.nodes[id].max.height = min(self.nodes[id].max.height, h);
                    h
                }
                _ => 0,
            };
            let natural_h = max(total_min_h, base_h);
            self.nodes[id].content_size.height = natural_h;
            self.nodes[id].current_size.height = natural_h.min(self.nodes[id].max.height);
            if !self.nodes[id].clip_children {
                self.nodes[id].min.height = natural_h;
            }
        } else {
            let base_h = match self.nodes[id].size.height {
                Length::Fixed(h) => {
                    self.nodes[id].min.height = max(self.nodes[id].min.height, h);
                    self.nodes[id].max.height = min(self.nodes[id].max.height, h);
                    h
                }
                _ => 0,
            };
            let natural_h = max(self.nodes[id].min.height, base_h);
            self.nodes[id].content_size.height = natural_h;
            self.nodes[id].current_size.height = natural_h.min(self.nodes[id].max.height);
        }
    }

    // Phase 4: assign heights within available space
    fn assign_height(&mut self, id: usize, parent_height: i32) {
        if let Some(child_idx) = self.nodes[id].first_child {
            let layout_dir = self.nodes[id].layout_dir;
            let pad = self.nodes[id].padding;
            let spacing = self.nodes[id].spacing;
            let target_h = match self.nodes[id].size.height {
                Length::Grow => parent_height,
                Length::Fixed(h) => h,
                Length::Fit => self.nodes[id].current_size.height,
            };
            let target_h = max(target_h, self.nodes[id].min.height).min(self.nodes[id].max.height);
            self.nodes[id].current_size.height = target_h;

            let inner_h = (target_h
                - pad.top
                - pad.bottom
                - if let Axis::Vertical = layout_dir {
                    let mut count = 0;
                    let mut idx = Some(child_idx);
                    while let Some(child) = idx {
                        if !self.nodes[child].is_absolute {
                            count += 1;
                        }
                        idx = self.nodes[child].next_sibling;
                    }
                    if count > 0 { (count - 1) * spacing } else { 0 }
                } else {
                    0
                })
            .max(0);
            if let Axis::Vertical = layout_dir {
                // **Vertical container**: distribute inner height among children
                let mut children = Vec::new();
                let mut idx = Some(child_idx);
                while let Some(child) = idx {
                    if !self.nodes[child].is_absolute {
                        children.push(child);
                    }
                    idx = self.nodes[child].next_sibling;
                }
                let mut allocated = Vec::with_capacity(children.len());
                let mut total_base = 0;
                for &child in &children {
                    allocated.push(self.nodes[child].current_size.height);
                    total_base += self.nodes[child].current_size.height;
                }
                let mut remaining = inner_h - total_base;
                if remaining < 0 {
                    // shrink heights above min_height
                    let mut deficit = -remaining;
                    while deficit > 0 {
                        let shrinkables: Vec<usize> = children
                            .iter()
                            .enumerate()
                            .filter(|(j, child)| {
                                allocated[*j] > self.nodes[**child].min.height
                                    && !matches!(self.nodes[**child].size.height, Length::Fixed(_))
                            })
                            .map(|(j, _)| j)
                            .collect();
                        if shrinkables.is_empty() {
                            break;
                        }
                        let reduce_each = (deficit / shrinkables.len() as i32).max(1);
                        let mut used = 0;
                        for &j in &shrinkables {
                            let child = children[j];
                            let reducible = allocated[j] - self.nodes[child].min.height;
                            let reduce_amt = min(reducible, reduce_each);
                            if reduce_amt > 0 {
                                allocated[j] -= reduce_amt;
                                used += reduce_amt;
                                deficit -= reduce_amt;
                                if deficit <= 0 {
                                    break;
                                }
                            }
                        }
                        if used == 0 {
                            break;
                        }
                    }
                    remaining = 0;
                }
                if remaining > 0 {
                    // distribute extra height to any Grow children
                    while remaining > 0 {
                        let growables: Vec<usize> = children
                            .iter()
                            .enumerate()
                            .filter(|(j, child)| {
                                matches!(self.nodes[**child].size.height, Length::Grow)
                                    && allocated[*j] < self.nodes[**child].max.height
                            })
                            .map(|(j, _)| j)
                            .collect();
                        if growables.is_empty() {
                            break;
                        }
                        let add_each = (remaining / growables.len() as i32).max(1);
                        let mut used = 0;
                        for &j in &growables {
                            let child = children[j];
                            let addable = self.nodes[child].max.height - allocated[j];
                            let add_amt = min(addable, add_each);
                            if add_amt > 0 {
                                allocated[j] += add_amt;
                                used += add_amt;
                                remaining -= add_amt;
                                if remaining <= 0 {
                                    break;
                                }
                            }
                        }
                        if used == 0 {
                            break;
                        }
                    }
                }
                for (j, &child) in children.iter().enumerate() {
                    self.assign_height(child, allocated[j]);
                }

                // Absolute children get full inner height available
                let mut idx = Some(child_idx);
                while let Some(child) = idx {
                    if self.nodes[child].is_absolute {
                        self.assign_height(child, inner_h);
                    }
                    idx = self.nodes[child].next_sibling;
                }
            } else {
                // **Horizontal container**: all children get same inner height
                let mut idx = Some(child_idx);
                while let Some(child) = idx {
                    self.assign_height(child, inner_h);
                    idx = self.nodes[child].next_sibling;
                }
            }
        } else {
            let target_h = match self.nodes[id].size.height {
                Length::Grow => parent_height,
                Length::Fixed(h) => h,
                Length::Fit => self.nodes[id].current_size.height,
            };
            let final_h = max(target_h, self.nodes[id].min.height).min(self.nodes[id].max.height);
            self.nodes[id].current_size.height = final_h;
        }
    }

    // Phase 5: place all nodes (compute positions)
    fn place(&mut self, id: usize, x: i32, y: i32) -> (i32, i32) {
        // Set this nodes position
        self.nodes[id].pos = Position::new(x, y);
        if let Some(child_idx) = self.nodes[id].first_child {
            let layout_dir = self.nodes[id].layout_dir;
            let pad = self.nodes[id].padding;
            // Starting cursor at top-left content area
            let base_x = x + pad.left;
            let base_y = y + pad.top;
            let mut cursor_x = base_x;
            let mut cursor_y = base_y;
            let mut idx = Some(child_idx);
            while let Some(child) = idx {
                let is_abs = self.nodes[child].is_absolute;
                let child_w = self.nodes[child].current_size.width;
                let child_h = self.nodes[child].current_size.height;
                let offx = self.nodes[child].offset_pos.x;
                let offy = self.nodes[child].offset_pos.y;
                let next = self.nodes[child].next_sibling;
                if !is_abs {
                    match layout_dir {
                        Axis::Horizontal => {
                            self.place(child, cursor_x, base_y);
                            cursor_x += child_w + self.nodes[id].spacing;
                        }
                        Axis::Vertical => {
                            self.place(child, base_x, cursor_y);
                            cursor_y += child_h + self.nodes[id].spacing;
                        }
                    }
                } else {
                    let abs_x = base_x + offx;
                    let abs_y = base_y + offy;
                    self.place(child, abs_x, abs_y);
                }
                idx = next;
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
