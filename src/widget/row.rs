use super::*;

#[derive(Widget)]
pub struct Row {
    children: Vec<Element>,
    spacing: i32,
    padding: Vec4<i32>,
    size: Size<Length>,
    color: Color,
    min: Size<i32>,
    max: Size<i32>,
    main_align: Align,
    cross_align: Align,
}
impl Row {
    pub fn empty() -> Self {
        Self::new::<Vec<_>, Element>(el!())
    }
    pub fn new<I, E>(children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Element>,
    {
        Self {
            children: children.into_iter().map(Into::into).collect(),
            spacing: 0,
            padding: Vec4::splat(0),
            size: Size::splat(Length::Fit),
            color: Color::TRANSPARENT,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            main_align: Align::Start,
            cross_align: Align::Start,
        }
    }
    /// Alignment of children along the layout axis (horizontal for a `Row`).
    pub fn main(mut self, align: Align) -> Self {
        self.main_align = align;
        self
    }
    /// Alignment of children across the layout axis (vertical for a `Row`).
    pub fn cross(mut self, align: Align) -> Self {
        self.cross_align = align;
        self
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
        E: Into<Element>,
    {
        self.children.push(element.into());
    }
}
impl Widget for Row {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> Node {
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
            main_align: self.main_align,
            cross_align: self.cross_align,
            ..Default::default()
        }
    }

    fn child_count(&self) -> usize {
        self.children.len()
    }
    fn child_mut(&mut self, i: usize) -> &mut dyn Widget {
        self.children[i].as_mut()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        if self.color.a() > 0 {
            let r = ctx.rect();
            ctx.surface(out, r.xywh(), self.color, Color::TRANSPARENT);
        }
    }
}
