use std::num::NonZero;

use super::*;

pub struct WrappingRows<M> {
    rows: Vec<Row<M>>,
    size: Size<Length>,
    color: Color,
    padding: Vec4<i32>,
    row_spacing: i32,
    min: Size<i32>,
    max: Size<i32>,
}

impl<M> WrappingRows<M> {
    pub fn empty(columns: NonZero<usize>) -> Self {
        Self::new::<Vec<_>, Element<M>>(columns, el!())
    }
    pub fn new<I, E>(columns: NonZero<usize>, children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Element<M>>,
    {
        let mut cells: Vec<Element<M>> = children.into_iter().map(Into::into).collect();
        let mut rows: Vec<Row<M>> = Vec::new();

        while !cells.is_empty() {
            let take = cells.len().min(columns.into());
            rows.push(
                Row::new(cells.drain(..take))
                    .spacing(8)
                    .color(Color::TRANSPARENT),
            );
        }

        Self {
            rows,
            size: Size::splat(Length::Fit),
            color: Color::TRANSPARENT,
            padding: Vec4::splat(0),
            row_spacing: 8,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
        }
    }

    pub fn row_spacing(mut self, px: i32) -> Self {
        self.row_spacing = px;
        self
    }
    pub fn col_spacing(mut self, px: i32) -> Self {
        for r in self.rows.iter_mut() {
            r.set_spacing(px);
        }
        self
    }
    pub fn padding(mut self, pad: Vec4<i32>) -> Self {
        self.padding = pad;
        self
    }
    pub fn size(mut self, s: Size<Length>) -> Self {
        self.size = s;
        self
    }
    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
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

impl<M> IntoElement for WrappingRows<M> {}

impl<M> Widget<M> for WrappingRows<M> {
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
            spacing: self.row_spacing,
            ..Default::default()
        }
    }

    fn child_count(&self) -> usize {
        self.rows.len()
    }
    fn child_mut(&mut self, i: usize) -> &mut dyn Widget<M> {
        &mut self.rows[i]
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        if self.color.a() > 0 {
            let r = ctx.rect();
            ctx.surface(out, r.xywh(), self.color, Color::TRANSPARENT);
        }
    }
}
