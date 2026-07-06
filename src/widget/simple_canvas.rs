use super::*;
use crate::render::pipeline::PipelineKey;

pub struct SimpleCanvas<M> {
    size: Size<Length>,
    key: &'static str,
    with_handle: Option<fn(&mut EventCtx<M>)>,
    min: Size<i32>,
    max: Size<i32>,
}

impl<M> SimpleCanvas<M> {
    pub fn new(
        size: Size<Length>,
        pipeline_key: &'static str,
        with_handle: Option<fn(&mut EventCtx<M>)>,
    ) -> Self {
        Self {
            size,
            key: pipeline_key,
            with_handle,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
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

impl<M> IntoElement for SimpleCanvas<M> {}

impl<M> Widget<M> for SimpleCanvas<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
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
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        unreachable!()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        let r = ctx.rect();
        out.push(Instance::new(
            PipelineKey::Other(self.key),
            Position::new(r.x as f32, r.y as f32),
            Size::new(r.w as f32, r.h as f32),
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ));
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        if let Some(f) = self.with_handle {
            f(ctx);
        }
    }
}
