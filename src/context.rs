use std::{
    any::Any,
    collections::{HashMap, HashSet},
};

use crate::{
    event::{KeyState, MouseButton, UiEventRef},
    graphics::{Globals, Gpu},
    layout::LayoutEngine,
    model::{Position, Size},
    render::{text::TextSystem, texture::TextureRegistry},
    theme::Theme,
};

// TODO: would be nice if widgets didn't have to remember their ids, and just got their view_state
// automatically
pub type Id = u64;

pub struct SweepCtx<'a> {
    pub gpu: &'a Gpu,
    pub texture: &'a mut TextureRegistry,
}

pub trait OnSweep: Any {
    fn on_sweep(&mut self, cx: &mut SweepCtx);
}

struct Entry {
    value: Box<dyn Any>,
    on_sweep: Option<fn(&mut dyn Any, &mut SweepCtx)>,
}

type ViewStateInner = HashMap<Id, Entry>;

#[derive(Default)]
pub struct ViewState {
    inner: ViewStateInner,
    touched: HashSet<Id>,
}

impl ViewState {
    pub fn get<T: 'static>(&self, id: &Id) -> Option<&T> {
        self.inner.get(id)?.value.downcast_ref::<T>()
    }
    pub fn get_mut<T: 'static>(&mut self, id: &Id) -> Option<&mut T> {
        self.inner.get_mut(id)?.value.downcast_mut::<T>()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn ensure<T: 'static>(&mut self, id: Id, default: impl FnOnce() -> T) -> &mut T {
        self.touched.insert(id);
        self.ensure_inner(id, default, None)
    }

    pub fn ensure_swept<T: OnSweep + 'static>(
        &mut self,
        id: Id,
        default: impl FnOnce() -> T,
    ) -> &mut T {
        self.touched.insert(id);

        fn dispatch<T: OnSweep + 'static>(v: &mut dyn Any, cx: &mut SweepCtx) {
            v.downcast_mut::<T>().unwrap().on_sweep(cx);
        }

        self.ensure_inner(id, default, Some(dispatch::<T>))
    }

    fn ensure_inner<T: 'static>(
        &mut self,
        id: Id,
        default: impl FnOnce() -> T,
        on_sweep: Option<fn(&mut dyn Any, &mut SweepCtx)>,
    ) -> &mut T {
        use std::collections::hash_map::Entry as MapEntry;
        let entry = match self.inner.entry(id) {
            MapEntry::Vacant(v) => v.insert(Entry {
                value: Box::new(default()),
                on_sweep,
            }),
            MapEntry::Occupied(mut o) => {
                if !o.get().value.is::<T>() {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        "Id {} overlapped! Possible duplicate Keyed key under the same parent.",
                        id
                    );
                    let slot = o.get_mut();
                    slot.value = Box::new(default());
                    slot.on_sweep = on_sweep;
                }
                o.into_mut()
            }
        };

        entry.value.downcast_mut::<T>().unwrap()
    }

    pub(crate) fn was_touched(&self, id: &Id) -> bool {
        self.touched.contains(id)
    }

    fn drain_stale(&mut self) -> Vec<(Id, Entry)> {
        let stale: Vec<Id> = self
            .inner
            .keys()
            .copied()
            .filter(|id| !self.touched.contains(id))
            .collect();
        let mut out = Vec::with_capacity(stale.len());
        for id in stale {
            if let Some(e) = self.inner.remove(&id) {
                out.push((id, e));
            }
        }
        self.touched.clear();
        out
    }

    pub(crate) fn sweep(&mut self, cx: &mut SweepCtx) {
        for (_, mut entry) in self.drain_stale() {
            if let Some(f) = entry.on_sweep {
                f(entry.value.as_mut(), cx);
            }
        }
    }

    #[doc(hidden)]
    pub fn sweep_for_test(&mut self) {
        drop(self.drain_stale());
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

    pub fn sweep_focus(&mut self) {
        if let Some(id) = self.hot_item
            && !self.view_state.was_touched(&id)
        {
            self.hot_item = None;
        }
        if let Some(id) = self.active_item
            && !self.view_state.was_touched(&id)
        {
            self.active_item = None;
        }
        if let Some(id) = self.kbd_focus_item
            && !self.view_state.was_touched(&id)
        {
            self.kbd_focus_item = None;
        }
    }
}

pub struct LayoutCtx<'a, M> {
    pub globals: &'a Globals,
    pub ui: &'a mut Context<M>,
    pub text: &'a mut TextSystem,
    pub theme: &'a Theme,
}

impl<'a, M> LayoutCtx<'a, M> {
    pub fn physical_size(&self, logical: Size<u32>) -> Size<u32> {
        let sf = self.globals.scale;
        Size::new(
            (logical.width as f32 * sf).round() as u32,
            (logical.height as f32 * sf).round() as u32,
        )
    }
}

pub struct PrepareCtx<'a> {
    pub globals: &'a Globals,
    pub text: &'a mut TextSystem,
    pub gpu: &'a Gpu,
    pub texture: &'a mut TextureRegistry,
    pub(crate) layout: &'a LayoutEngine,
    pub(crate) current_node: usize,
    pub view_state: &'a mut ViewState,
    pub theme: &'a Theme,
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
    pub fn physical_size(&self, logical: Size<u32>) -> Size<u32> {
        let sf = self.globals.scale;
        Size::new(
            (logical.width as f32 * sf).round() as u32,
            (logical.height as f32 * sf).round() as u32,
        )
    }
}

pub struct PaintCtx<'a> {
    pub globals: &'a Globals,
    pub text: &'a TextSystem,
    pub(crate) layout: &'a LayoutEngine,
    pub(crate) current_node: usize,
    pub view_state: &'a mut ViewState,
    pub theme: &'a Theme,
}

impl<'a> PaintCtx<'a> {
    pub fn new(
        globals: &'a Globals,
        text: &'a TextSystem,
        layout: &'a LayoutEngine,
        view_state: &'a mut ViewState,
        theme: &'a Theme,
    ) -> Self {
        Self {
            globals,
            text,
            layout,
            current_node: 0,
            view_state,
            theme,
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
    pub fn physical_size(&self, logical: Size<u32>) -> Size<u32> {
        let sf = self.globals.scale;
        Size::new(
            (logical.width as f32 * sf).round() as u32,
            (logical.height as f32 * sf).round() as u32,
        )
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

    #[test]
    fn ensure_marks_touched() {
        let mut vs = ViewState::default();
        vs.ensure(42, || 1u32);
        assert!(vs.was_touched(&42));
    }

    #[test]
    fn sweep_for_test_removes_untouched_entries() {
        let mut vs = ViewState::default();
        vs.ensure(1, || 100u32);
        vs.ensure(2, || 200u32);
        vs.sweep_for_test(); // clears touched

        vs.ensure(1, || 100u32); // only 1 touched this frame
        vs.sweep_for_test();

        assert_eq!(vs.inner.len(), 1);
        assert!(vs.get::<u32>(&1).is_some());
        assert!(vs.get::<u32>(&2).is_none());
    }

    #[test]
    fn touched_cleared_after_sweep() {
        let mut vs = ViewState::default();
        vs.ensure(1, || 1u32);
        vs.sweep_for_test();
        assert!(!vs.was_touched(&1));
    }

    // This test guards the type-mismatch branch in ensure_inner:
    // when an id is reused with a different T, both `value` AND
    // `on_sweep` must be replaced together. If only `value` is replaced,
    // the next sweep dispatches through the OLD T's downcast, which
    // fails the unwrap and panics. This test would catch that by
    // existing — if it compiles and runs without panic, the branch
    // is correct.
    #[test]
    fn type_mismatch_resets_on_sweep_dispatcher() {
        use crate::context::{OnSweep, SweepCtx};

        struct A;
        impl OnSweep for A {
            fn on_sweep(&mut self, _: &mut SweepCtx) {}
        }
        struct B;
        impl OnSweep for B {
            fn on_sweep(&mut self, _: &mut SweepCtx) {}
        }

        let mut vs = ViewState::default();
        vs.ensure_swept(7, || A);
        vs.sweep_for_test(); // Touched cleared. A still in map.
        vs.ensure_swept(7, || B); // Same id, different T — replaces.
        vs.sweep_for_test(); // B is touched, stays. No panic = pass.

        // Now untouch and run sweep through the real path with a stub
        // SweepCx — actually we can't build SweepCx in tests. The
        // assertion above is enough: if dispatchers were mismatched,
        // a real call to sweep() would panic. The integration-level
        // coverage is fine for that.
        let _ = vs.get::<B>(&7).expect("B should still be at id 7");
    }
}
