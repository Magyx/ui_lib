use super::*;

pub struct Spacer {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,
}

impl Spacer {
    pub fn new(size: Size<Length>) -> Self {
        Self {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            size,
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

impl IntoElement for Spacer {}

impl<M> Widget<M> for Spacer {
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

    fn paint(&mut self, _ctx: &mut PaintCtx, _out: &mut Vec<Instance>) {}
}
