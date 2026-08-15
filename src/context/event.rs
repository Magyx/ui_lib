use super::{Context, Id, MessageSink};
use crate::{
    event::{KeyState, MouseButton, UiEventRef},
    focus::{Dir, ScopeId},
    gpu::Globals,
    layout::LayoutEngine,
    model::{Position, Rect},
    text::TextBackend,
    tree::ROOT_SEED,
};

pub struct EventCtx<'a> {
    pub globals: &'a Globals,
    pub text: &'a mut dyn TextBackend,
    pub ui: &'a mut Context,
    pub event: Option<UiEventRef<'a>>,
    #[doc(hidden)]
    pub layout: &'a LayoutEngine,
    pub(crate) current_node: usize,
    pub(crate) offset: Position<i32>,
    pub(crate) focus_scope: ScopeId,
    pub(crate) clip: Option<Rect>,

    sink: &'a mut dyn MessageSink,
}

impl<'a> EventCtx<'a> {
    pub fn new(
        globals: &'a Globals,
        text: &'a mut dyn TextBackend,
        ui: &'a mut Context,
        event: Option<UiEventRef<'a>>,
        layout: &'a LayoutEngine,
        sink: &'a mut dyn MessageSink,
    ) -> Self {
        Self {
            globals,
            text,
            ui,
            event,
            layout,
            current_node: 0usize,
            offset: Position::splat(0),
            focus_scope: ROOT_SEED,
            clip: None,

            sink,
        }
    }
    pub(crate) fn __set_data(
        &mut self,
        current_node: usize,
        acc_tx: i32,
        acc_ty: i32,
        focus_scope: ScopeId,
        clip: Option<[i32; 4]>,
    ) {
        self.current_node = current_node;
        self.offset = Position::new(acc_tx, acc_ty);
        self.focus_scope = focus_scope;
        self.clip = clip.map(|[x, y, w, h]| Rect::new(x, y, w, h));
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
        self.ui.view_state.ensure(self.id(), default)
    }
    pub fn first_child_node(&self) -> Option<usize> {
        self.layout.nodes[self.current_node].first_child
    }

    pub fn child_content_height(&self) -> i32 {
        if let Some(cid) = self.first_child_node() {
            self.layout.nodes[cid].content_size.height.max(0)
        } else {
            0
        }
    }

    pub fn emit<M: 'static>(&mut self, msg: M) {
        self.sink.emit(Box::new(msg));
    }

    /// Focus scope this node belongs to.
    #[inline]
    pub fn focus_scope(&self) -> ScopeId {
        self.focus_scope
    }
    /// True if this widget currently holds keyboard focus.
    #[inline]
    pub fn is_focused(&self) -> bool {
        self.ui.focus.is_focused(self.id())
    }
    /// True if this widget is the current hover target.
    #[inline]
    pub fn is_hovered(&self) -> bool {
        self.ui.focus.is_hovered(self.id())
    }
    /// True if this widget is currently pressed/active (e.g. mid-drag).
    #[inline]
    pub fn is_pressed(&self) -> bool {
        self.ui.focus.is_pressed(self.id())
    }
    /// Whether pointer input should reach this node given any active trap or clip.
    #[inline]
    pub fn pointer_available(&self) -> bool {
        self.ui.focus.pointer_available(self.focus_scope)
            && self
                .clip
                .as_ref()
                .is_none_or(|c| c.contains(self.ui.mouse_pos))
    }
    /// Convenience: pointer is over this node *and* reaches it.
    #[inline]
    pub fn pointer_over(&self) -> bool {
        self.pointer_available() && self.rect().contains(self.ui.mouse_pos)
    }
    /// Claim hover for this node (last claimer in the walk wins).
    #[inline]
    pub fn claim_hover(&mut self) {
        let id = self.id();
        self.ui.focus.claim_hover(id);
    }
    /// Request keyboard focus for this node (resolved at end of walk).
    #[inline]
    pub fn request_focus(&mut self) {
        let id = self.id();
        self.ui.focus.request_set(id);
    }
    /// Request that keyboard focus be dropped.
    #[inline]
    pub fn clear_focus(&mut self) {
        self.ui.focus.request_clear();
    }
    /// Request a keyboard focus move (used by the engine's Tab handling).
    #[inline]
    pub fn move_focus(&mut self, dir: Dir) {
        self.ui.focus.request_move(dir);
    }
    /// Mark this node as pressed/active (persists across events until released).
    #[inline]
    pub fn begin_press(&mut self) {
        let id = self.id();
        self.ui.focus.begin_press(id);
    }
    /// Release this node's press if it holds it.
    #[inline]
    pub fn end_press(&mut self) {
        let id = self.id();
        self.ui.focus.end_press(id);
    }

    #[inline]
    pub fn is_mouse_pressed(&self, b: MouseButton) -> bool {
        matches!(
            self.event,
            Some(UiEventRef::MouseButton {
                button,
                state: KeyState::Pressed,
            }) if button == b
        ) && self.ui.is_button_pressed(b)
    }
    #[inline]
    pub fn is_mouse_released(&self, b: MouseButton) -> bool {
        matches!(
            self.event,
            Some(UiEventRef::MouseButton {
                button,
                state: KeyState::Released,
            }) if button == b
        ) && self.ui.is_button_released(b)
    }
}
