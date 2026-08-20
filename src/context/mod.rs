use std::any::Any;

use crate::{
    event::{Modifiers, MouseButton},
    focus::Focus,
    model::{Color, Position},
    task::TaskStore,
    theme::TextStyle,
};

mod state;
pub use state::*;

mod layout;
pub use layout::*;

mod prepare;
pub use prepare::*;

mod paint;
pub use paint::*;

mod event;
pub use event::*;

pub type Id = u64;

pub const POINTER_ELSEWHERE: Position<f32> = Position::splat(f32::NEG_INFINITY);

pub struct Context {
    pub mouse_pos: Position<f32>,
    pub mouse_buttons_down: u32,
    pub mouse_buttons_pressed: u32,
    pub mouse_buttons_released: u32,
    pub modifiers: Modifiers,
    pub surface_focused: bool,
    pub focus: Focus,
    pub view_state: ViewState,

    pub(crate) tasks: TaskStore,
    redraw_requested: bool,
}
impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
impl Context {
    pub fn new() -> Self {
        Self {
            mouse_pos: Position::splat(0.0),
            mouse_buttons_down: 0,
            mouse_buttons_pressed: 0,
            mouse_buttons_released: 0,
            modifiers: Modifiers::default(),
            surface_focused: true,
            focus: Focus::new(),
            view_state: ViewState::default(),

            tasks: TaskStore::default(),
            redraw_requested: false,
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

    pub fn request_redraw(&mut self) {
        self.redraw_requested = true;
    }
    pub fn take_redraw(&mut self) -> bool {
        let r = self.redraw_requested;
        self.redraw_requested = false;
        r
    }

    pub fn sweep_focus(&mut self) {
        let vs = &self.view_state;
        self.focus.sweep(|id| vs.was_touched(&id));
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Env {
    /// Tonal elevation for surface-color resolution.
    pub elevation: u8,
    /// Inherited foreground (text/icon) color; widgets resolve their default
    /// content color from this instead of hardcoding `theme.on_surface`.
    pub foreground: Color,
    /// Inherited default text style.
    pub text: TextStyle,
}

/// A drain-able queue of type-erased app messages.
pub trait MessageSink {
    /// Push a type-erased message onto the queue.
    fn emit(&mut self, msg: Box<dyn Any>);
    /// Take everything queued so far, leaving the queue empty.
    fn drain(&mut self) -> Vec<Box<dyn Any>>;
}

#[derive(Default)]
pub struct BasicMessageSink {
    messages: Vec<Box<dyn Any>>,
}
impl BasicMessageSink {
    pub fn new() -> Self {
        Self::default()
    }
}
impl MessageSink for BasicMessageSink {
    fn emit(&mut self, msg: Box<dyn Any>) {
        self.messages.push(msg);
    }
    fn drain(&mut self) -> Vec<Box<dyn Any>> {
        std::mem::take(&mut self.messages)
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
        let ctx = Context::new();
        assert_eq!(ctx.mouse_pos.x, 0.0);
        assert_eq!(ctx.mouse_pos.y, 0.0);
        assert_eq!(ctx.mouse_buttons_down, 0);
        assert_eq!(ctx.mouse_buttons_pressed, 0);
        assert_eq!(ctx.mouse_buttons_released, 0);
        assert!(ctx.focus.focused().is_none());
        assert!(ctx.focus.hovered().is_none());
        assert!(ctx.focus.pressed().is_none());
    }

    #[test]
    fn is_button_down_reads_bitfield() {
        let mut ctx = Context::new();
        ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        assert!(ctx.is_button_down(MouseButton::Left));
        assert!(!ctx.is_button_down(MouseButton::Right));
    }

    #[test]
    fn is_button_pressed_and_released_read_respective_fields() {
        let mut ctx = Context::new();
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
        let mut ctx = Context::new();
        ctx.mouse_buttons_down = (1 << MouseButton::Left.bit()) | (1 << MouseButton::Right.bit());
        assert!(ctx.is_button_down(MouseButton::Left));
        assert!(ctx.is_button_down(MouseButton::Right));
        assert!(!ctx.is_button_down(MouseButton::Middle));
    }

    #[test]
    fn emit_and_take_round_trips_messages_in_order() {
        let mut sink = BasicMessageSink::new();
        sink.emit(Box::new(Msg::A));
        sink.emit(Box::new(Msg::B(42)));
        sink.emit(Box::new(Msg::A));

        let taken: Vec<Msg> = sink
            .drain()
            .into_iter()
            .map(|msg| *msg.downcast::<Msg>().unwrap())
            .collect();

        assert_eq!(taken, vec![Msg::A, Msg::B(42), Msg::A]);
    }

    #[test]
    fn take_on_empty_returns_empty_vec() {
        let mut sink = BasicMessageSink::new();
        assert!(sink.drain().is_empty());
    }

    // FIX: request_redraw is now part of EventCtx
    //
    // #[test]
    // fn request_redraw_is_consumed_by_take_redraw() {
    //     let mut sink = VecSink::<Msg>::new();
    //     let mut ctx = Context::new(&mut sink);
    //     assert!(!ctx.take_redraw(), "initial take_redraw should be false");
    //
    //     ctx.request_redraw();
    //     assert!(
    //         ctx.take_redraw(),
    //         "after request_redraw, take should be true"
    //     );
    //     assert!(
    //         !ctx.take_redraw(),
    //         "second take after a single request should be false"
    //     );
    // }
    //
    // #[test]
    // fn multiple_request_redraw_calls_only_need_one_take() {
    //     let mut sink = VecSink::<Msg>::new();
    //     let mut ctx = Context::new(&mut sink);
    //     ctx.request_redraw();
    //     ctx.request_redraw();
    //     ctx.request_redraw();
    //     assert!(ctx.take_redraw());
    //     assert!(!ctx.take_redraw());
    // }
}
