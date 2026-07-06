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

impl<M> IntoElement for Keyed<M> {}

impl<M, W: Widget<M>> Widget<M> for Keyed<W> {
    fn layout<'a>(&mut self, ctx: &mut LayoutCtx<'a, M>) -> Node {
        self.inner.layout(ctx)
    }
    fn identity_key(&self) -> Option<u64> {
        Some(self.key)
    }
    fn set_id(&mut self, id: Id) {
        self.inner.set_id(id);
    }
    fn child_count(&self) -> usize {
        self.inner.child_count()
    }
    fn child_mut(&mut self, idx: usize) -> &mut dyn Widget<M> {
        self.inner.child_mut(idx)
    }

    fn min_height_for_width<'a>(&mut self, ctx: &mut LayoutCtx<'a, M>, width: i32) -> Option<i32> {
        self.inner.min_height_for_width(ctx, width)
    }

    fn children_offset(&self, view_state: &mut ViewState) -> (i32, i32) {
        self.inner.children_offset(view_state)
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
    fn paint_overlay(&mut self, ctx: &mut PaintCtx, instancess: &mut Vec<Instance>) {
        self.inner.paint_overlay(ctx, instancess)
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        self.inner.handle(ctx)
    }
    fn handle_after(&mut self, ctx: &mut EventCtx<M>) {
        self.inner.handle_after(ctx)
    }
}
