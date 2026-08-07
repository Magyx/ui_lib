use super::*;
use crate::render::pipeline::{Pipeline, PipelineId};

pub struct SimpleCanvas {
    size: Size<Length>,
    pipeline: PipelineId,
    with_handle: Option<fn(&mut EventCtx)>,
    min: Size<i32>,
    max: Size<i32>,
}

impl SimpleCanvas {
    /// Draw a full-bounds quad through `P`.
    ///
    /// `P` does not need to be registered yet — the index is reserved on first
    /// use and the pipeline is looked up at draw time.
    pub fn new<P: Pipeline>(size: Size<Length>, with_handle: Option<fn(&mut EventCtx)>) -> Self {
        Self {
            size,
            pipeline: PipelineId::of::<P>(),
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

impl IntoElement for SimpleCanvas {}

impl Widget for SimpleCanvas {
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

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        let r = ctx.rect();
        out.push(Instance::with_pipeline(
            self.pipeline,
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
