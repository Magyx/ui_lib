#[allow(unused)] // This is only used during tests.
#[cfg(test)]
pub use harness::*;

#[allow(unused)]
#[cfg(test)]
mod harness {
    use core::fmt;
    use std::{any::Any, cell::Cell, rc::Rc, sync::Arc};

    use ui::{
        context::{BasicMessageSink, Context, EventCtx, LayoutCtx, MessageSink, PaintCtx},
        event::UiEventRef,
        graphics::Globals,
        layout::{LayoutEngine, handle_tree, paint_tree, run_layout},
        model::{Color, Size},
        primitive::{Instance, InstanceStore},
        render::text_cosmic::TextCosmic,
        theme::Theme,
        widget::{Element, IntoElement, Length, Rectangle, Widget},
    };

    pub trait TestEvent: Any + fmt::Debug {
        fn eq_any(&self, other: &dyn Any) -> bool;
        fn as_any(&self) -> &dyn Any;
    }

    impl<T: Any + PartialEq + fmt::Debug> TestEvent for T {
        fn eq_any(&self, other: &dyn Any) -> bool {
            other.downcast_ref::<T>() == Some(self)
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    pub struct TopMsg {
        inner: Arc<dyn TestEvent>,
    }

    impl TopMsg {
        pub fn from<T: 'static + PartialEq + fmt::Debug>(w: T) -> Self {
            TopMsg { inner: Arc::new(w) }
        }

        pub fn get<T: 'static + PartialEq + fmt::Debug>(&self) -> Option<&T> {
            self.inner.as_any().downcast_ref::<T>()
        }
    }

    impl fmt::Debug for TopMsg {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.inner.fmt(f)
        }
    }

    impl PartialEq for TopMsg {
        fn eq(&self, other: &Self) -> bool {
            self.inner.eq_any(other.inner.as_any())
        }
    }

    impl Clone for TopMsg {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
            }
        }
    }

    pub struct Harness {
        pub globals: Globals,
        pub ctx: Context,
        pub text: TextCosmic,
        pub message_sink: Box<dyn MessageSink>,
        pub engine: LayoutEngine,
        pub theme: Theme,
    }

    impl Harness {
        pub fn drain_messages(&mut self) -> Vec<TopMsg> {
            self.message_sink
                .drain()
                .into_iter()
                .map(|msg| {
                    *msg.downcast::<TopMsg>()
                        .expect("VecSink contained wrong message type")
                })
                .collect()
        }
        pub fn layout<W: Widget>(&mut self, root: &mut W, max_w: i32, max_h: i32) -> usize {
            self.globals.window_size = [max_w as f32, max_h as f32];
            let rood_id = {
                let mut lctx = LayoutCtx::new(
                    &self.globals,
                    &mut self.ctx.view_state,
                    &mut self.text,
                    &self.theme,
                );
                run_layout(&mut self.engine, &mut lctx, root, max_w, max_h)
            };
            let _ = self.paint(&mut *root);
            rood_id
        }
        pub fn handle<W: Widget>(&mut self, root: &mut W) {
            let mut ectx = EventCtx::new(
                &self.globals,
                &mut self.text,
                &mut self.ctx,
                None,
                &self.engine,
                &mut *self.message_sink,
            );
            let mut cursor = 0usize;
            handle_tree(root, &mut ectx, &mut cursor);
        }
        pub fn handle_event<W: Widget>(&mut self, root: &mut W, event: UiEventRef) {
            let mut ectx = EventCtx::new(
                &self.globals,
                &mut self.text,
                &mut self.ctx,
                Some(event),
                &self.engine,
                &mut *self.message_sink,
            );
            let mut cursor = 0usize;
            handle_tree(root, &mut ectx, &mut cursor);
        }

        pub fn paint<W: Widget>(&mut self, root: &mut W) -> InstanceStore {
            let mut out = InstanceStore::new();
            let mut cursor = 0;
            let mut pctx = PaintCtx::new(
                &self.globals,
                &self.text,
                &self.engine,
                &mut self.ctx.view_state,
                &self.theme,
                &self.ctx.focus,
            );
            let screen_clip = Some([
                0,
                0,
                self.globals.window_size[0] as i32,
                self.globals.window_size[1] as i32,
            ]);
            paint_tree(
                root,
                &mut pctx,
                &self.engine,
                &mut cursor,
                &mut out,
                screen_clip,
            );
            out
        }
    }

    impl Default for Harness {
        fn default() -> Self {
            Self {
                globals: Globals {
                    window_size: [0.0, 0.0],
                    mouse_pos: [0.0, 0.0],
                    mouse_buttons: 0,
                    time: 0.0,
                    delta_time: 0.0,
                    frame: 0,
                    scale: 1.0,
                },
                ctx: Context::new(),
                text: TextCosmic::default(),
                message_sink: Box::new(BasicMessageSink::new()),
                engine: LayoutEngine::new(),
                theme: Theme::dark(),
            }
        }
    }

    pub type Rect = (i32, i32, i32, i32);
    pub type RectSlot = Rc<Cell<Option<Rect>>>;

    /// Wraps any Widget and captures its final (x, y, w, h) in a shared cell.
    /// Cloning the Rc<Cell<...>> before moving the Probe into an Element lets
    /// us read back the captured rect after layout without any unsafe code.
    #[derive(Widget)]
    pub struct Probe<W: Widget> {
        inner: W,
        slot: RectSlot,
    }
    impl<W: Widget> Probe<W> {
        pub fn new(inner: W) -> (Self, RectSlot) {
            let slot: RectSlot = Rc::new(Cell::new(None));
            (
                Self {
                    inner,
                    slot: slot.clone(),
                },
                slot,
            )
        }
    }
    impl<W: Widget> Widget for Probe<W> {
        fn layout<'a>(&mut self, ctx: &mut LayoutCtx<'a>) -> ui::layout::Node {
            self.inner.layout(ctx)
        }
        fn child_count(&self) -> usize {
            self.inner.child_count()
        }
        fn child_mut(&mut self, i: usize) -> &mut dyn Widget {
            self.inner.child_mut(i)
        }
        fn prepare(&mut self, ctx: &mut ui::context::PrepareCtx) {
            self.inner.prepare(ctx);
        }
        fn paint(&mut self, ctx: &mut ui::context::PaintCtx, out: &mut InstanceStore) {
            let r = ctx.rect();
            self.slot.set(Some(r.xywh()));
            self.inner.paint(ctx, out);
        }
        fn paint_overlay(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
            self.inner.paint_overlay(ctx, out);
        }
        fn handle(&mut self, ctx: &mut ui::context::EventCtx) {
            self.inner.handle(ctx);
        }
        fn handle_after(&mut self, ctx: &mut EventCtx) {
            self.inner.handle_after(ctx);
        }
    }

    /// Convenience: build a probed rectangle, return (Element, slot).
    pub fn probed_rect(w: Length, h: Length, color: Color) -> (Element, RectSlot) {
        let (p, slot) = Probe::new(Rectangle::new(Size::new(w, h), color));
        (Element::new(p), slot)
    }

    /// Convenience: unwrap the rect captured by a slot.
    pub fn read(slot: &RectSlot) -> Rect {
        slot.get()
            .expect("layout+paint did not capture a rect for this Probe")
    }

    /// Minimal clip container for exercising `clip_children` layout behavior.
    #[derive(Widget)]
    pub struct ClipBox {
        child: Element,
        size: Size<Length>,
        clip: bool,
        max: Size<i32>,
    }
    impl ClipBox {
        pub fn new(size: Size<Length>, clip: bool, child: impl Into<Element>) -> Self {
            Self {
                child: child.into(),
                size,
                clip,
                max: Size::splat(i32::MAX),
            }
        }
        pub fn max(mut self, m: Size<i32>) -> Self {
            self.max = m;
            self
        }
    }
    impl Widget for ClipBox {
        fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> ui::layout::Node {
            ui::layout::Node {
                size: self.size,
                clip_children: self.clip,
                max: self.max,
                ..Default::default()
            }
        }
        fn child_count(&self) -> usize {
            1
        }
        fn child_mut(&mut self, _i: usize) -> &mut dyn Widget {
            self.child.as_mut()
        }
        fn paint(&mut self, _ctx: &mut PaintCtx, _out: &mut InstanceStore) {}
    }
}
