use super::*;

pub struct Spacer {
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,
}

impl Spacer {
    pub fn new(size: Size<Length>) -> Self {
        Self {
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

impl Widget for Spacer {
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

    fn paint(&mut self, _ctx: &mut PaintCtx, _out: &mut InstanceStore) {}
}
