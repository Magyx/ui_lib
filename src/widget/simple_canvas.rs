use std::marker::PhantomData;

use super::*;
use crate::primitive::{Instanced, Primitive};

pub struct SimpleCanvas<P: Instanced<Primitive>> {
    size: Size<Length>,
    with_handle: Option<fn(&mut EventCtx)>,
    min: Size<i32>,
    max: Size<i32>,
    _pipeline: PhantomData<fn() -> P>,
}

impl<P: Instanced<Primitive>> SimpleCanvas<P> {
    /// Draw a full-bounds quad through `P`.
    pub fn new(size: Size<Length>, with_handle: Option<fn(&mut EventCtx)>) -> Self {
        Self {
            size,
            with_handle,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            _pipeline: PhantomData,
        }
    }
    pub fn min(mut self, s: Size<i32>) -> Self {
        self.min = s;
        self
    }
    pub fn max(mut self, s: Size<i32>) -> Self {
        self.max = s;
        self
    }
}

impl<P: Instanced<Primitive>> IntoElement for SimpleCanvas<P> {}

impl<P: Instanced<Primitive>> Widget for SimpleCanvas<P> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            ..Default::default()
        }
    }

    fn child_count(&self) -> usize {
        0
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget {
        unreachable!()
    }

    fn prepare(&mut self, ctx: &mut PrepareCtx) {
        // Declaring the dependency here is what registers `P`.
        ctx.ensure_pipeline::<P>();
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        let r = ctx.rect();
        out.push(Instance::new::<P>(
            Position::new(r.x as f32, r.y as f32),
            Size::new(r.w as f32, r.h as f32),
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ));
    }

    fn handle(&mut self, ctx: &mut EventCtx) {
        if let Some(f) = self.with_handle {
            f(ctx);
        }
    }
}
