use super::*;

pub struct Rectangle {
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
    pub fn min_y(mut self, height: i32) -> Self {
        self.min.height = height;
        self
    }
    pub fn min_x(mut self, width: i32) -> Self {
        self.min.width = width;
        self
    }
    pub fn max(mut self, s: Size<i32>) -> Self {
        self.max = s;
        self
    }
}

impl IntoElement for Rectangle {}

impl<M> Widget<M> for Rectangle {
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
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        unreachable!()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        if self.color.a() != 0 {
            let r = ctx.rect();
            out.push(Instance::ui(
                Position::new(r.x as f32, r.y as f32),
                Size::new(r.w as f32, r.h as f32),
                self.color,
            ));
        }
    }
}
