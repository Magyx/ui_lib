use std::{any::Any, collections::HashMap};

use crate::{
    event::{KeyState, MouseButton, UiEventRef},
    graphics::{Globals, Gpu},
    layout::LayoutEngine,
    model::Position,
    render::{text::TextSystem, texture::TextureRegistry},
};

pub type Id = u64;

type ViewStateInner = HashMap<Id, Box<dyn Any>>;

#[derive(Default)]
pub struct ViewState {
    inner: ViewStateInner,
}

impl ViewState {
    pub fn map(&self) -> &ViewStateInner {
        &self.inner
    }
    pub fn map_mut(&mut self) -> &mut ViewStateInner {
        &mut self.inner
    }

    pub fn get<T: 'static>(&self, id: &Id) -> Option<&T> {
        self.inner.get(id)?.downcast_ref::<T>()
    }
    pub fn get_mut<T: 'static>(&mut self, id: &Id) -> Option<&mut T> {
        self.inner.get_mut(id)?.downcast_mut::<T>()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn ensure<T: 'static>(&mut self, id: Id, default: impl FnOnce() -> T) -> &mut T {
        use std::collections::hash_map::Entry;
        let entry = match self.inner.entry(id) {
            Entry::Vacant(v) => v.insert(Box::new(default())),
            Entry::Occupied(mut o) => {
                if !o.get().is::<T>() {
                    tracing::warn!(
                        "Id {} overlapped! Possible duplicate Keyed key under the same parent.",
                        id
                    );
                    *o.get_mut() = Box::new(default());
                }
                o.into_mut()
            }
        };

        entry.downcast_mut::<T>().unwrap()
    }
}

pub struct Context<M> {
    pub mouse_pos: Position<f32>,
    pub mouse_buttons_down: u32,
    pub mouse_buttons_pressed: u32,
    pub mouse_buttons_released: u32,

    pub hot_item: Option<Id>,
    pub active_item: Option<Id>,
    pub kbd_focus_item: Option<Id>,

    messages: Vec<M>,
    redraw_requested: bool,

    pub view_state: ViewState,
}

impl<M> Default for Context<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Context<M> {
    pub fn new() -> Self {
        Self {
            mouse_pos: Position::splat(0.0),
            mouse_buttons_down: 0,
            mouse_buttons_pressed: 0,
            mouse_buttons_released: 0,

            hot_item: None,
            active_item: None,
            kbd_focus_item: None,

            messages: Vec::new(),
            redraw_requested: false,

            view_state: ViewState::default(),
        }
    }
    #[inline]
    pub fn is_button_down(&self, b: MouseButton) -> bool {
        (self.mouse_buttons_down & (1 << b.bit())) != 0
    }
    #[inline]
    fn is_button_pressed(&self, b: MouseButton) -> bool {
        (self.mouse_buttons_pressed & (1 << b.bit())) != 0
    }
    #[inline]
    fn is_button_released(&self, b: MouseButton) -> bool {
        (self.mouse_buttons_released & (1 << b.bit())) != 0
    }

    pub fn take(&mut self) -> Vec<M> {
        std::mem::take(&mut self.messages)
    }

    pub fn emit(&mut self, msg: M) {
        self.messages.push(msg);
    }

    pub fn request_redraw(&mut self) {
        self.redraw_requested = true;
    }

    pub fn take_redraw(&mut self) -> bool {
        let r = self.redraw_requested;
        self.redraw_requested = false;
        r
    }
}

pub struct LayoutCtx<'a, M> {
    pub globals: &'a Globals,
    pub ui: &'a mut Context<M>,
    pub text: &'a mut TextSystem,
}

pub struct PrepareCtx<'a> {
    pub globals: &'a Globals,
    pub text: &'a mut TextSystem,
    pub gpu: &'a Gpu,
    pub texture: &'a mut TextureRegistry,
    pub(crate) layout: &'a LayoutEngine,
    pub(crate) current_node: usize,
    pub view_state: &'a mut ViewState,
}

impl<'a> PrepareCtx<'a> {
    pub(crate) fn __set_current_node(&mut self, i: usize) {
        self.current_node = i;
    }
    pub fn current_node_id(&self) -> usize {
        self.current_node
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
}

pub struct PaintCtx<'a> {
    pub globals: &'a Globals,
    pub text: &'a TextSystem,
    pub(crate) layout: &'a LayoutEngine,
    pub(crate) current_node: usize,
    pub view_state: &'a mut ViewState,
}

impl<'a> PaintCtx<'a> {
    pub fn new(
        globals: &'a Globals,
        text: &'a TextSystem,
        layout: &'a LayoutEngine,
        view_state: &'a mut ViewState,
    ) -> Self {
        Self {
            globals,
            text,
            layout,
            current_node: 0,
            view_state,
        }
    }

    pub(crate) fn __set_current_node(&mut self, i: usize) {
        self.current_node = i;
    }

    pub fn current_node_id(&self) -> usize {
        self.current_node
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
}

pub struct EventCtx<'a, M> {
    pub globals: &'a Globals,
    pub text: &'a mut TextSystem,
    pub ui: &'a mut Context<M>,
    pub event: Option<UiEventRef<'a>>,
    #[doc(hidden)]
    pub layout: &'a LayoutEngine,
    pub(crate) current_node: usize,
}

impl<'a, M> EventCtx<'a, M> {
    pub fn new(
        globals: &'a Globals,
        text: &'a mut TextSystem,
        ui: &'a mut Context<M>,
        event: Option<UiEventRef<'a>>,
        layout: &'a LayoutEngine,
        current_node: usize,
    ) -> Self {
        Self {
            globals,
            text,
            ui,
            event,
            layout,
            current_node,
        }
    }

    pub(crate) fn __set_current_node(&mut self, i: usize) {
        self.current_node = i;
    }

    pub fn current_node_id(&self) -> usize {
        self.current_node
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::MouseButton;

    // Context is generic over message type M. We use a simple enum here.
    #[derive(Debug, PartialEq, Clone)]
    enum Msg {
        A,
        B(i32),
    }

    #[test]
    fn context_new_defaults_are_empty() {
        let ctx: Context<Msg> = Context::new();
        assert_eq!(ctx.mouse_pos.x, 0.0);
        assert_eq!(ctx.mouse_pos.y, 0.0);
        assert_eq!(ctx.mouse_buttons_down, 0);
        assert_eq!(ctx.mouse_buttons_pressed, 0);
        assert_eq!(ctx.mouse_buttons_released, 0);
        assert!(ctx.hot_item.is_none());
        assert!(ctx.active_item.is_none());
        assert!(ctx.kbd_focus_item.is_none());
    }

    #[test]
    fn is_button_down_reads_bitfield() {
        let mut ctx: Context<Msg> = Context::new();
        ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        assert!(ctx.is_button_down(MouseButton::Left));
        assert!(!ctx.is_button_down(MouseButton::Right));
    }

    #[test]
    fn is_button_pressed_and_released_read_respective_fields() {
        let mut ctx: Context<Msg> = Context::new();
        ctx.mouse_buttons_pressed = 1 << MouseButton::Right.bit();
        ctx.mouse_buttons_released = 1 << MouseButton::Middle.bit();

        assert!(ctx.is_button_pressed(MouseButton::Right));
        assert!(!ctx.is_button_pressed(MouseButton::Middle));

        assert!(ctx.is_button_released(MouseButton::Middle));
        assert!(!ctx.is_button_released(MouseButton::Right));

        // Down field is independent.
        assert!(!ctx.is_button_down(MouseButton::Right));
    }

    #[test]
    fn multiple_buttons_coexist_in_bitfield() {
        let mut ctx: Context<Msg> = Context::new();
        ctx.mouse_buttons_down = (1 << MouseButton::Left.bit()) | (1 << MouseButton::Right.bit());
        assert!(ctx.is_button_down(MouseButton::Left));
        assert!(ctx.is_button_down(MouseButton::Right));
        assert!(!ctx.is_button_down(MouseButton::Middle));
    }

    #[test]
    fn emit_and_take_round_trips_messages_in_order() {
        let mut ctx: Context<Msg> = Context::new();
        ctx.emit(Msg::A);
        ctx.emit(Msg::B(42));
        ctx.emit(Msg::A);

        let taken = ctx.take();
        assert_eq!(taken, vec![Msg::A, Msg::B(42), Msg::A]);

        // Take drains the queue.
        assert!(ctx.take().is_empty());
    }

    #[test]
    fn take_on_empty_returns_empty_vec() {
        let mut ctx: Context<Msg> = Context::new();
        assert!(ctx.take().is_empty());
    }

    #[test]
    fn request_redraw_is_consumed_by_take_redraw() {
        let mut ctx: Context<Msg> = Context::new();
        assert!(!ctx.take_redraw(), "initial take_redraw should be false");

        ctx.request_redraw();
        assert!(
            ctx.take_redraw(),
            "after request_redraw, take should be true"
        );
        assert!(
            !ctx.take_redraw(),
            "second take after a single request should be false"
        );
    }

    #[test]
    fn multiple_request_redraw_calls_only_need_one_take() {
        let mut ctx: Context<Msg> = Context::new();
        ctx.request_redraw();
        ctx.request_redraw();
        ctx.request_redraw();
        assert!(ctx.take_redraw());
        assert!(!ctx.take_redraw());
    }

    #[derive(Debug, PartialEq)]
    struct DummyState {
        counter: u32,
        label: &'static str,
    }

    #[test]
    fn view_state_starts_empty() {
        let ctx: Context<Msg> = Context::new();
        assert!(ctx.view_state.is_empty());
    }

    #[test]
    fn view_state_insert_and_downcast_mut() {
        let mut ctx: Context<Msg> = Context::new();
        let id: crate::context::Id = 42;

        // Widget-typical pattern: or_insert_with + downcast_mut.
        let st = ctx.view_state.ensure(id, || DummyState {
            counter: 0,
            label: "init",
        });
        st.counter += 1;
        st.label = "touched";

        // Second access sees the prior state.
        let again = ctx.view_state.get_mut::<DummyState>(&id).unwrap();
        assert_eq!(again.counter, 1);
        assert_eq!(again.label, "touched");
    }

    #[test]
    fn view_state_different_ids_are_independent() {
        let mut ctx: Context<Msg> = Context::new();

        for id in [1u64, 2, 99, 1_000_000] {
            ctx.view_state.ensure(id, || DummyState {
                counter: id as u32,
                label: "x",
            });
        }

        for id in [1u64, 2, 99, 1_000_000] {
            let st = ctx.view_state.get_mut::<DummyState>(&id).unwrap();
            assert_eq!(st.counter, id as u32);
        }
    }

    #[test]
    fn view_state_wrong_type_downcast_returns_none() {
        // If a widget tries to downcast to the wrong type (e.g. two
        // widgets collide on an Id), downcast_mut returns None rather
        // than corrupting memory. Widgets in this codebase `.expect()`
        // the downcast, which will panic — but the panic is a safer
        // failure mode than UB.

        let mut ctx: Context<Msg> = Context::new();
        let id: crate::context::Id = 7;

        ctx.view_state.ensure(id, || 123u32);

        let as_dummy = ctx.view_state.get_mut::<DummyState>(&id);
        assert!(as_dummy.is_none(), "wrong-type downcast must be None");

        let as_u32 = ctx.view_state.get_mut::<u32>(&id).copied();
        assert_eq!(as_u32, Some(123));
    }
}
