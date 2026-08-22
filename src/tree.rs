use crate::{
    context::{Env, Id, LayoutCtx, PaintCtx, PrepareCtx},
    event::{KeyState, LogicalKey, UiEventRef},
    focus::{Dir, ScopeId},
    layout::{__Node, LayoutEngine, NodeIdx},
    model::{Color, Position, Rect, Size},
    primitive::{Instance, InstanceStore},
    widget::Widget,
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
fn color_for_depth(depth: usize) -> Color {
    const P: &[Color] = &[
        Color::rgba(255, 66, 66, 220),  // red
        Color::rgba(255, 159, 28, 220), // orange
        Color::rgba(255, 214, 10, 220), // yellow
        Color::rgba(52, 199, 89, 220),  // green
        Color::rgba(48, 176, 199, 220), // teal
        Color::rgba(10, 132, 255, 220), // blue
        Color::rgba(94, 92, 230, 220),  // indigo
        Color::rgba(191, 90, 242, 220), // purple
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

    root_id.as_usize()
}

fn build_tree<'a>(
    layout_engine: &mut LayoutEngine,
    ctx: &mut LayoutCtx<'a>,
    w: &mut dyn Widget,
    seed: u64,
    env: Env,
) -> NodeIdx {
    ctx.__set_id(seed);
    ctx.__set_env(env);
    let desc = w.layout(ctx);
    let i = layout_engine.create_node(desc, seed);

    let child_env = w.child_env(env, ctx.theme);
    for y in 0..w.child_count() {
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
    let id = NodeIdx::new(*cursor);

    let first = eng.nodes[id].first_child;
    if first.is_none() {
        ctx.__set_id(eng.nodes[id].id);
        if let Some(h) = w.min_height_for_width(ctx, eng.nodes[id].current_size.width) {
            let clamped = h
                .max(eng.nodes[id].min.height)
                .min(eng.nodes[id].max.height);
            eng.nodes[id].min.height = clamped;
        }
    }

    *cursor += 1;
    for i in 0..w.child_count() {
        post_width_query(w.child_mut(i), eng, ctx, cursor);
    }
}

pub fn prepare_tree(w: &mut dyn crate::widget::Widget, ctx: &mut PrepareCtx, cursor: &mut usize) {
    crate::scope!("layout::prepare_tree");
    let env = ctx.theme.root_env();
    __prepare_tree(w, ctx, cursor, env);
}

fn __prepare_tree(
    w: &mut dyn crate::widget::Widget,
    ctx: &mut PrepareCtx,
    cursor: &mut usize,
    env: Env,
) {
    let id = NodeIdx::new(*cursor);
    ctx.__set_data(id, env);

    w.prepare(ctx);
    *cursor += 1;

    let child_env = w.child_env(env, ctx.theme);
    for i in 0..w.child_count() {
        let child = w.child_mut(i);
        __prepare_tree(child, ctx, cursor, child_env);
    }
}

#[inline]
fn calculate_clip(n: &__Node, parent_clip: Option<Rect>) -> Option<Rect> {
    if !n.clip_children {
        return parent_clip;
    }
    let node_rect = Rect {
        x: n.pos.x,
        y: n.pos.y,
        w: n.current_size.width,
        h: n.current_size.height,
    };
    Some(match parent_clip {
        Some(parent) => parent.intersect(&node_rect),
        None => node_rect,
    })
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

    __handle_tree(w, ctx, cursor, ROOT_SEED, None);

    ctx.ui.focus.end_walk();
}

fn __handle_tree(
    w: &mut dyn crate::widget::Widget,
    ctx: &mut crate::context::EventCtx,
    cursor: &mut usize,
    scope: ScopeId,
    parent_clip: Option<Rect>,
) {
    let id = NodeIdx::new(*cursor);

    let n = ctx.layout.nodes[id];
    let clip = calculate_clip(&n, parent_clip);
    ctx.__set_data(id, scope, clip);

    let node_id = ctx.id();
    if w.focusable() {
        ctx.ui.focus.register(node_id, scope);
        ctx.ui.view_state.touch(node_id);
    }
    let child_scope = if w.focus_trap() {
        ctx.ui.focus.note_trap(node_id);
        node_id
    } else {
        scope
    };

    w.handle(ctx);
    *cursor += 1;

    let child_count = w.child_count();
    for i in 0..child_count {
        let child = w.child_mut(i);
        __handle_tree(child, ctx, cursor, child_scope, clip);
    }

    ctx.__set_data(id, scope, clip);
    w.handle_after(ctx);
}

pub fn paint_tree(
    w: &mut dyn crate::widget::Widget,
    ctx: &mut PaintCtx,
    eng: &crate::layout::LayoutEngine,
    cursor: &mut usize,
    out: &mut InstanceStore,
    parent_clip: Option<Rect>,
) {
    crate::scope!("layout::paint_tree");
    let env = ctx.theme.root_env();
    __paint_tree(w, ctx, eng, cursor, out, parent_clip, 0, 0, env);
}
#[allow(clippy::too_many_arguments)]
fn __paint_tree(
    w: &mut dyn crate::widget::Widget,
    ctx: &mut PaintCtx,
    eng: &crate::layout::LayoutEngine,
    cursor: &mut usize,
    out: &mut InstanceStore,
    parent_clip: Option<Rect>,
    parent_layer: u16,
    depth: usize,
    env: Env,
) {
    let id = NodeIdx::new(*cursor);
    ctx.__set_data(id, env);
    let n = eng.nodes[id];

    let clip = calculate_clip(&n, parent_clip);
    let prev_clip = out.set_clip(clip);
    let layer = w.layer_shift().resolve(parent_layer);
    let prev_layer = out.set_layer(layer);
    w.paint(ctx, out);
    if w.focusable() && ctx.is_focused() {
        w.paint_focus_ring(ctx, out);
    }

    *cursor += 1;

    let child_env = w.child_env(env, ctx.theme);
    for i in 0..w.child_count() {
        let child = w.child_mut(i);
        __paint_tree(
            child,
            ctx,
            eng,
            cursor,
            out,
            clip,
            layer,
            depth + 1,
            child_env,
        );
    }

    if eng.debug {
        ctx.__set_data(id, env);
        out.set_layer(layer);
        let r = ctx.rect();
        let col = color_for_depth(depth);
        let thickness = 1;

        out.push(Instance::ui_rounded(
            Position::new(r.x as f32, r.y as f32),
            Size::new(r.w as f32, r.h as f32),
            Color::TRANSPARENT,
            1.0,
            thickness,
            col,
        ));
    }
    out.set_clip(prev_clip);
    out.set_layer(prev_layer);
}
