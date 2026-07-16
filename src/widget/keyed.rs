use std::hash::{DefaultHasher, Hash, Hasher};

use super::*;

pub struct Keyed<W> {
    key: u64,
    inner: W,
}

impl<W> Keyed<W> {
    pub fn new<K: Hash>(key: K, inner: W) -> Self {
        let mut h = DefaultHasher::default();
        key.hash(&mut h);
        Self {
            key: h.finish(),
            inner,
        }
    }
}

impl<W> IntoElement for Keyed<W> {}

impl<W: Widget> Widget for Keyed<W> {
    fn layout<'a>(&mut self, ctx: &mut LayoutCtx<'a>) -> Node {
        self.inner.layout(ctx)
    }
    fn key(&self) -> Option<u64> {
        Some(self.key)
    }
    fn child_count(&self) -> usize {
        self.inner.child_count()
    }
    fn child_mut(&mut self, idx: usize) -> &mut dyn Widget {
        self.inner.child_mut(idx)
    }
    fn child_env(&self, env: Env, theme: &Theme) -> Env {
        self.inner.child_env(env, theme)
    }

    fn min_height_for_width<'a>(&mut self, ctx: &mut LayoutCtx<'a>, width: i32) -> Option<i32> {
        self.inner.min_height_for_width(ctx, width)
    }

    fn children_offset(&self, view_state: &mut ViewState, id: Id) -> (i32, i32) {
        self.inner.children_offset(view_state, id)
    }
    fn prepare(&mut self, ctx: &mut PrepareCtx) {
        self.inner.prepare(ctx)
    }
    fn prepare_overlay(&mut self, ctx: &mut PrepareCtx) {
        self.inner.prepare_overlay(ctx)
    }
    fn paint(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        self.inner.paint(ctx, instances)
    }
    fn paint_overlay(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        self.inner.paint_overlay(ctx, instances)
    }

    fn handle(&mut self, ctx: &mut EventCtx) {
        self.inner.handle(ctx)
    }
    fn handle_after(&mut self, ctx: &mut EventCtx) {
        self.inner.handle_after(ctx)
    }
}
