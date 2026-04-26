use std::num::NonZero;

use super::*;

pub struct Grid<M> {
    x: i32,
    y: i32,
    w: i32,
    h: i32,

    rows: Vec<Row<M>>,
    size: Size<Length>,
    color: Color,
    padding: Vec4<i32>,
    row_spacing: i32,
    min: Size<i32>,
    max: Size<i32>,
}

impl<M> Grid<M> {
    pub fn new<I, E>(columns: NonZero<usize>, children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Element<M>>,
    {
        let mut cells: Vec<Element<M>> = children.into_iter().map(Into::into).collect();
        let mut rows: Vec<Row<M>> = Vec::new();

        while !cells.is_empty() {
            let take = cells.len().min(columns.into());
            let mut row_cells = Vec::with_capacity(take);
            for _ in 0..take {
                row_cells.push(cells.remove(0));
            }
            rows.push(Row::new(row_cells).spacing(8).color(Color::TRANSPARENT));
        }

        Self {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
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

impl<M> IntoElement for Grid<M> {}

impl<M: 'static> Widget<M> for Grid<M> {
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

    fn set_layout(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.x = x;
        self.y = y;
        self.w = w;
        self.h = h;
    }

    fn child_count(&self) -> usize {
        self.rows.len()
    }
    fn child_mut(&mut self, i: usize) -> &mut dyn Widget<M> {
        &mut self.rows[i]
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
}
