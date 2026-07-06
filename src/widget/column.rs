use super::*;

pub struct Column<M> {
    children: Vec<Element<M>>,
    spacing: i32,
    padding: Vec4<i32>,
    size: Size<Length>,
    color: Color,
    min: Size<i32>,
    max: Size<i32>,
}

impl<M> Column<M> {
    pub fn empty() -> Self {
        Self::new::<Vec<_>, Element<M>>(el!())
    }
    pub fn new<I, E>(children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Element<M>>,
    {
        Self {
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

    pub fn push<E>(&mut self, element: E)
    where
        E: Into<Element<M>>,
    {
        self.children.push(element.into());
    }
}

impl<M> IntoElement for Column<M> {}

impl<M: 'static> Widget<M> for Column<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            layout_dir: Axis::Vertical,
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

    fn child_count(&self) -> usize {
        self.children.len()
    }
    fn child_mut(&mut self, i: usize) -> &mut dyn Widget<M> {
        self.children[i].as_mut()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        let r = ctx.rect();
        ctx.surface(out, r.xywh(), self.color, Color::TRANSPARENT);
    }
}
