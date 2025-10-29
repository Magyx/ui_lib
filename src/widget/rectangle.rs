use super::*;

pub struct Rectangle {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    size: Size<Length>,
    color: Color,
    min: Size<i32>,
    max: Size<i32>,
}

impl Rectangle {
    pub fn placeholder() -> Self {
        Self::new(Size::splat(Length::Fit), Color::TRANSPARENT)
    }
    pub fn new(size: Size<Length>, color: Color) -> Self {
        Self {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            size,
            color,
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

impl IntoElement for Rectangle {}

impl<M> Widget<M> for Rectangle {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
        Node {
            width: self.size.width,
            height: self.size.height,
            min_width: self.min.width,
            min_height: self.min.height,
            max_width: self.max.width,
            max_height: self.max.height,
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
        if self.color.a() != 0 {
            out.push(Instance::ui(
                Position::new(self.x, self.y),
                Size::new(self.w, self.h),
                self.color,
            ));
        }
    }
}
