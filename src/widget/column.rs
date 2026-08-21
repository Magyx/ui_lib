use crate::widget::prelude::*;

#[derive(Widget)]
pub struct Column {
    children: Vec<Element>,
    spacing: i32,
    padding: Inset,
    size: Size<Length>,
    color: Color,
    min: Size<i32>,
    max: Size<i32>,
    main: Main,
    cross: Align,
    cross_self: Option<Align>,
    fill_cross: bool,
}
impl Column {
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
            padding: Inset::ZERO,
            size: Size::splat(Length::Fit),
            color: Color::TRANSPARENT,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            main: Main::default(),
            cross: Align::START,
            cross_self: None,
            fill_cross: false,
        }
    }
    /// Alignment of children along the layout axis (vertical for a `Column`).
    pub fn main(mut self, main: impl Into<Main>) -> Self {
        self.main = main.into();
        self
    }
    /// Alignment of children across the layout axis (horizontal for a `Column`).
    pub fn cross(mut self, align: Align) -> Self {
        self.cross = align;
        self
    }

    /// Override the alignment this container is given by *its* parent.
    pub fn cross_self(mut self, align: Align) -> Self {
        self.cross_self = Some(align);
        self
    }

    /// Make `Fit` children fill the cross axis. Replaces the old
    /// `cross(Align::Stretch)`.
    pub fn fill_cross(mut self, fill: bool) -> Self {
        self.fill_cross = fill;
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
    pub fn padding(mut self, amount: impl Into<Inset>) -> Self {
        self.padding = amount.into();
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
        E: Into<Element>,
    {
        self.children.push(element.into());
    }
}
impl Widget for Column {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            layout_dir: Axis::Vertical,
            padding: self.padding,
            spacing: self.spacing,
            main: self.main,
            cross: self.cross,
            cross_self: self.cross_self,
            fill_cross: self.fill_cross,
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

pub struct Center;
impl Center {
    #![allow(clippy::new_ret_no_self)]
    pub fn new<E>(child: E) -> Column
    where
        E: Into<Element>,
    {
        Column::new(std::iter::once(child))
            .size(Size::splat(Length::Fill(1.0)))
            .main(Align::CENTER)
            .cross(Align::CENTER)
    }
}
