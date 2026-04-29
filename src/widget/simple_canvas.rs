use super::*;
use crate::render::pipeline::PipelineKey;

pub struct SimpleCanvas<M> {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
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
            x: 0,
            y: 0,
            w: 0,
            h: 0,
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

impl<M: 'static> Widget<M> for SimpleCanvas<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            ..Default::default()
        }
    }

    fn set_layout(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.x = x;
        self.y = y;
        self.w = w;
        self.h = h;
    }

    fn child_count(&self) -> usize {
        0
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        unreachable!()
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        out.push(Instance::new(
            PipelineKey::Other(self.key),
            Position::new(self.x as f32, self.y as f32),
            Size::new(self.w as f32, self.h as f32),
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
