use super::{Env, Id, ViewState};
use crate::{
    focus::Focus,
    gpu::Globals,
    layout::LayoutEngine,
    model::{Color, Position, Rect, Size},
    primitive::{Instance, InstanceStore},
    text::TextBackend,
    theme::Theme,
};

pub struct PaintCtx<'a> {
    pub globals: &'a Globals,
    pub text: &'a dyn TextBackend,
    pub(crate) layout: &'a LayoutEngine,
    pub(crate) current_node: usize,
    pub(crate) offset: Position<i32>,
    pub view_state: &'a mut ViewState,
    pub theme: &'a Theme,
    pub env: Env,
    pub(crate) focus: &'a Focus,
}

impl<'a> PaintCtx<'a> {
    pub fn new(
        globals: &'a Globals,
        text: &'a dyn TextBackend,
        layout: &'a LayoutEngine,
        view_state: &'a mut ViewState,
        theme: &'a Theme,
        focus: &'a Focus,
    ) -> Self {
        Self {
            globals,
            text,
            layout,
            current_node: 0,
            offset: Position::splat(0),
            view_state,
            theme,
            env: theme.root_env(),
            focus,
        }
    }
    pub(crate) fn __set_data(&mut self, current_node: usize, acc_tx: i32, acc_ty: i32, env: Env) {
        self.current_node = current_node;
        self.offset = Position::new(acc_tx, acc_ty);
        self.env = env;
    }
    pub(crate) fn __set_env(&mut self, env: Env) {
        self.env = env;
    }
    pub fn current_node_id(&self) -> usize {
        self.current_node
    }
    pub fn rect(&self) -> Rect {
        let n = &self.layout.nodes[self.current_node];
        Rect::new(
            n.pos.x + self.offset.x,
            n.pos.y + self.offset.y,
            n.current_size.width,
            n.current_size.height,
        )
    }
    pub fn id(&self) -> Id {
        self.layout.nodes[self.current_node].id
    }
    pub fn state_or<T: 'static>(&mut self, default: impl FnOnce() -> T) -> &mut T {
        self.view_state.ensure(self.id(), default)
    }
    pub fn first_child_node(&self) -> Option<usize> {
        self.layout.nodes[self.current_node].first_child
    }
    pub fn child_content_height(&self) -> i32 {
        if let Some(cid) = self.first_child_node() {
            self.layout.nodes[cid].natural_size.height.max(0)
        } else {
            0
        }
    }
    pub fn physical_size(&self, logical: Size<u32>) -> Size<u32> {
        let sf = self.globals.scale;
        Size::new(
            (logical.width as f32 * sf).round() as u32,
            (logical.height as f32 * sf).round() as u32,
        )
    }

    /// True if this widget currently holds keyboard focus.
    #[inline]
    pub fn is_focused(&self) -> bool {
        self.focus.is_focused(self.id())
    }
    /// True if this widget is the current hover target.
    #[inline]
    pub fn is_hovered(&self) -> bool {
        self.focus.is_hovered(self.id())
    }
    /// True if this widget is currently pressed/active.
    #[inline]
    pub fn is_pressed(&self) -> bool {
        self.focus.is_pressed(self.id())
    }

    pub fn fill(&self, out: &mut InstanceStore, (x, y, w, h): (i32, i32, i32, i32), color: Color) {
        if color.a() == 0 {
            return;
        }
        out.push(Instance::ui(
            Position::new(x as f32, y as f32),
            Size::new(w as f32, h as f32),
            color,
        ));
    }
    pub fn surface(
        &self,
        out: &mut InstanceStore,
        (x, y, w, h): (i32, i32, i32, i32),
        fill: Color,
        border: Color,
    ) {
        out.push(Instance::ui_rounded(
            Position::new(x as f32, y as f32),
            Size::new(w as f32, h as f32),
            fill,
            self.theme.corner_radius,
            self.theme.border_width,
            border,
        ));
    }

    pub fn focus_ring(&self, out: &mut InstanceStore, (x, y, w, h): (i32, i32, i32, i32)) {
        use crate::theme::{GAP, RING_WIDTH};
        out.push(Instance::ui_rounded(
            Position::new((x - GAP) as f32, (y - GAP) as f32),
            Size::new((w + GAP * 2) as f32, (h + GAP * 2) as f32),
            Color::TRANSPARENT,
            self.theme.corner_radius + GAP as f32,
            RING_WIDTH,
            self.theme.focus_outline,
        ));
    }
}
