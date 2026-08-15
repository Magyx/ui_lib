use crate::{
    context::{Env, EventCtx, Id, LayoutCtx, PaintCtx, PrepareCtx, ViewState},
    layout::Node,
    primitive::InstanceStore,
    theme::Theme,
};

pub trait IntoElement {}

pub trait Widget: IntoElement {
    fn layout<'a>(&mut self, ctx: &mut LayoutCtx<'a>) -> Node;
    fn key(&self) -> Option<u64> {
        None
    }
    fn child_count(&self) -> usize;
    fn child_mut(&mut self, idx: usize) -> &mut dyn Widget;
    fn child_env(&self, env: Env, theme: &Theme) -> Env {
        let _ = theme;
        env
    }

    fn focusable(&self) -> bool {
        false
    }
    fn focus_trap(&self) -> bool {
        false
    }
    fn paint_focus_ring(&self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        ctx.focus_ring(out, ctx.rect().xywh());
    }

    fn min_height_for_width<'a>(&mut self, ctx: &mut LayoutCtx<'a>, width: i32) -> Option<i32> {
        let _ = (ctx, width);
        None
    }

    fn children_offset(&self, view_state: &mut ViewState, id: Id) -> (i32, i32) {
        let _ = (view_state, id);
        (0, 0)
    }
    fn prepare(&mut self, ctx: &mut PrepareCtx) {
        let _ = ctx;
    }
    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore);
    fn paint_overlay(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        let _ = (ctx, out);
    }

    fn handle(&mut self, ctx: &mut EventCtx) {
        let _ = ctx;
    }
    fn handle_after(&mut self, ctx: &mut EventCtx) {
        let _ = ctx;
    }
}

pub struct Element {
    inner: Box<dyn Widget>,
}

impl Element {
    pub fn new<W>(widget: W) -> Self
    where
        W: Widget + 'static,
    {
        Self {
            inner: Box::new(widget),
        }
    }
}

impl<W> From<W> for Element
where
    W: Widget + IntoElement + 'static,
{
    fn from(w: W) -> Self {
        Self::new(w)
    }
}

impl AsRef<dyn Widget> for Element {
    fn as_ref(&self) -> &(dyn Widget + 'static) {
        self.inner.as_ref()
    }
}

impl AsMut<dyn Widget + 'static> for Element {
    fn as_mut(&mut self) -> &mut (dyn Widget + 'static) {
        self.inner.as_mut()
    }
}

#[macro_export]
macro_rules! el {
    ( $( $x:expr ),* $(,)? ) => {
        vec![ $( Element::from($x) ),* ]
    };
}
