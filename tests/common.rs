#[cfg(test)]
pub use harness::*;

#[allow(unused)]
#[cfg(test)]
mod harness {
    use core::fmt;
    use std::{any::Any, cell::Cell, rc::Rc, sync::Arc};

    use ui::{
        context::{Context, EventCtx, LayoutCtx, PaintCtx},
        event::UiEventRef,
        graphics::Globals,
        layout::{LayoutEngine, handle_tree, paint_tree, run_layout},
        model::{Color, Size},
        primitive::Instance,
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
        pub ctx: Context<TopMsg>,
        pub text: TextCosmic,
        pub engine: LayoutEngine,
        pub theme: Theme,
    }

    impl Harness {
        pub fn layout<W: Widget<TopMsg>>(&mut self, root: &mut W, max_w: i32, max_h: i32) -> usize {
            self.globals.window_size = [max_w as f32, max_h as f32];
            let mut lctx = LayoutCtx {
                globals: &self.globals,
                ui: &mut self.ctx,
                text: &mut self.text,
                theme: &self.theme,
            };
            run_layout(&mut self.engine, &mut lctx, root, max_w, max_h)
        }
        pub fn handle<W: Widget<TopMsg>>(&mut self, root: &mut W) {
            let mut ectx: EventCtx<TopMsg> = EventCtx::new(
                &self.globals,
                &mut self.text,
                &mut self.ctx,
                None,
                &self.engine,
                0usize,
            );
            let mut cursor = 0usize;
            handle_tree(root, &mut ectx, &mut cursor);
        }
        pub fn handle_event<W: Widget<TopMsg>>(&mut self, root: &mut W, event: UiEventRef) {
            let mut ectx: EventCtx<TopMsg> = EventCtx::new(
                &self.globals,
                &mut self.text,
                &mut self.ctx,
                Some(event),
                &self.engine,
                0usize,
            );
            let mut cursor = 0usize;
            handle_tree(root, &mut ectx, &mut cursor);
        }

        pub fn paint<W: Widget<TopMsg>>(&mut self, root: &mut W) -> Vec<Instance> {
            let mut out = Vec::new();
            let mut cursor = 0;
            let mut pctx = PaintCtx::new(
                &self.globals,
                &self.text,
                &self.engine,
                &mut self.ctx.view_state,
                &self.theme,
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
    pub struct Probe<M, W: Widget<M>> {
        inner: W,
        slot: RectSlot,
        _ph: std::marker::PhantomData<M>,
    }

    impl<M, W: Widget<M>> Probe<M, W> {
        pub fn new(inner: W) -> (Self, RectSlot) {
            let slot: RectSlot = Rc::new(Cell::new(None));
            (
                Self {
                    inner,
                    slot: slot.clone(),
                    _ph: std::marker::PhantomData,
                },
                slot,
            )
        }
    }

    impl<M, W: Widget<M>> IntoElement for Probe<M, W> {}

    impl<M: 'static, W: Widget<M>> Widget<M> for Probe<M, W> {
        fn layout<'a>(&mut self, ctx: &mut LayoutCtx<'a, M>) -> ui::layout::Node {
            self.inner.layout(ctx)
        }
        fn set_layout(&mut self, x: i32, y: i32, w: i32, h: i32) {
            self.inner.set_layout(x, y, w, h);
            self.slot.set(Some((x, y, w, h)));
        }
        fn child_count(&self) -> usize {
            self.inner.child_count()
        }
        fn child_mut(&mut self, i: usize) -> &mut dyn Widget<M> {
            self.inner.child_mut(i)
        }
        fn prepare(&mut self, ctx: &mut ui::context::PrepareCtx) {
            self.inner.prepare(ctx);
        }
        fn prepare_overlay(&mut self, ctx: &mut ui::context::PrepareCtx) {
            self.inner.prepare_overlay(ctx);
        }
        fn paint(
            &mut self,
            ctx: &mut ui::context::PaintCtx,
            out: &mut Vec<ui::primitive::Instance>,
        ) {
            self.inner.paint(ctx, out);
        }
        fn paint_overlay(&mut self, ctx: &mut PaintCtx, instancess: &mut Vec<Instance>) {
            self.inner.paint_overlay(ctx, instancess);
        }
        fn handle(&mut self, ctx: &mut ui::context::EventCtx<M>) {
            self.inner.handle(ctx);
        }
        fn handle_after(&mut self, ctx: &mut EventCtx<M>) {
            self.inner.handle_after(ctx);
        }
    }

    /// Convenience: build a probed rectangle, return (Element, slot).
    pub fn probed_rect(w: Length, h: Length, color: Color) -> (Element<TopMsg>, RectSlot) {
        let (p, slot) = Probe::new(Rectangle::new(Size::new(w, h), color));
        (Element::new(p), slot)
    }

    /// Convenience: unwrap the rect captured by a slot.
    pub fn read(slot: &RectSlot) -> Rect {
        slot.get().expect("layout did not call set_layout on Probe")
    }
}
