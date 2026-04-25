use super::*;

pub struct Row<M> {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    children: Vec<Element<M>>,
    spacing: i32,
    padding: Vec4<i32>,
    size: Size<Length>,
    color: Color,
    min: Size<i32>,
    max: Size<i32>,
}

impl<M> Row<M> {
    pub fn new<I, E>(children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Element<M>>,
    {
        Self {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            children: children.into_iter().map(Into::into).collect(),
            spacing: 0,
            padding: Vec4::splat(0),
            size: Size::splat(Length::Fit),
            color: Color::TRANSPARENT,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
        }
    }
    pub fn spacing(mut self, amount: i32) -> Self {
        self.spacing = amount;
        self
    }
    pub fn size(mut self, size: Size<Length>) -> Self {
        self.size = size;
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
    pub fn padding(mut self, amount: Vec4<i32>) -> Self {
        self.padding = amount;
        self
    }
    pub fn min(mut self, size: Size<i32>) -> Self {
        self.min = size;
        self
    }
    pub fn max(mut self, size: Size<i32>) -> Self {
        self.max = size;
        self
    }

    pub(crate) fn set_spacing(&mut self, amount: i32) {
        self.spacing = amount;
    }

    pub fn push<E>(&mut self, element: E)
    where
        E: Into<Element<M>>,
    {
        self.children.push(element.into());
    }
}

impl<M> IntoElement for Row<M> {}

impl<M: 'static> Widget<M> for Row<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            layout_dir: Axis::Horizontal,
            padding: Padding {
                left: self.padding.x,
                top: self.padding.y,
                right: self.padding.z,
                bottom: self.padding.w,
            },
            spacing: self.spacing,
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
        self.children.len()
    }
    fn child_mut(&mut self, i: usize) -> &mut dyn Widget<M> {
        self.children[i].as_mut()
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        if self.color.a() > 0 {
            out.push(Instance::ui(
                Position::new(self.x as f32, self.y as f32),
                Size::new(self.w as f32, self.h as f32),
                self.color,
            ));
        }
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        for c in &mut self.children {
            c.as_mut().handle(ctx);
        }
    }
}
