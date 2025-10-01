use super::*;
use crate::widget::helpers::{Height, equalize_sizes};

pub struct Column<M> {
    layout: Option<Layout>,

    id: Id,
    children: Vec<Element<M>>,
    spacing: i32,
    position: Position<i32>,
    size: Size<Length<i32>>,
    color: Color,
    padding: Vec4<i32>,
    min: Size<i32>,
    max: Size<i32>,
}

impl<M> Column<M> {
    pub fn new(children: Vec<Element<M>>) -> Self {
        Self {
            layout: None,

            id: crate::context::next_id(),
            children,
            spacing: 0,
            position: Position::splat(0),
            size: Size::splat(Length::Fit),
            color: Color::TRANSPARENT,
            padding: Vec4::splat(0),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
        }
    }

    pub fn spacing(mut self, amount: i32) -> Self {
        self.spacing = amount;
        self
    }

    pub fn size(mut self, size: Size<Length<i32>>) -> Self {
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
}

impl<M: 'static> Widget<M> for Column<M> {
    fn id(&self) -> Id {
        self.id
    }

    fn layout(&self) -> Layout {
        self.layout.expect(LAYOUT_ERROR)
    }

    fn fit_width(&mut self, ctx: &mut LayoutCtx<M>) -> Layout {
        let width_padding = self.padding.x + self.padding.z;

        let mut max_child_w = 0;
        for child in self.children.iter_mut() {
            let Layout { current_size, .. } = child.fit_width(ctx);
            max_child_w = max_child_w.max(current_size.width);
        }
        let min_w = max_child_w + width_padding;

        if matches!(self.size.width, Length::Fit) {
            self.size.width = Length::Fixed(min_w);
        }

        let resolved_w = self
            .size
            .into_fixed()
            .width
            .clamp(min_w.max(self.min.width), self.max.width);

        let l = Layout {
            size: self.size,
            current_size: Size::new(resolved_w, 0),
            min: Size::new(min_w.max(self.min.width), self.min.height),
            max: self.max,
        };
        self.layout = Some(l);
        l
    }

    fn grow_width(&mut self, ctx: &mut LayoutCtx<M>, parent_width: i32) {
        let l = self.layout.expect(LAYOUT_ERROR);

        let target_w = match self.size.width {
            Length::Grow => parent_width,
            Length::Fixed(w) => w,
            Length::Fit => l.current_size.width,
        }
        .max(l.min.width)
        .min(l.max.width)
        .min(parent_width);

        self.size.width = Length::Fixed(target_w);

        let inner_w = (target_w - self.padding.x - self.padding.z).max(0);
        for child in self.children.iter_mut() {
            child.grow_width(ctx, inner_w);
        }

        if let Some(lay) = self.layout.as_mut() {
            lay.current_size.width = target_w;
        }
    }

    fn fit_height(&mut self, ctx: &mut LayoutCtx<M>) -> Layout {
        let height_padding = self.padding.y + self.padding.w;

        let mut min_h = (self.children.len() as i32 - 1) * self.spacing + height_padding;
        for child in self.children.iter_mut() {
            let Layout { current_size, .. } = child.fit_height(ctx);
            min_h += current_size.height;
        }

        if matches!(self.size.height, Length::Fit) {
            self.size.height = Length::Fixed(min_h);
        }

        let prev = self.layout.expect(LAYOUT_ERROR);
        let prev_w = prev.current_size.width;

        let requested_h = match self.size.height {
            Length::Fixed(h) => h,
            _ => min_h,
        };
        let resolved_h = requested_h
            .max(self.min.height.max(min_h))
            .min(self.max.height);

        let l = Layout {
            size: self.size,
            current_size: Size::new(prev_w, resolved_h),
            min: Size::new(prev.min.width, self.min.height.max(min_h)),
            max: self.max,
        };
        self.layout = Some(l);
        l
    }

    fn grow_height(&mut self, ctx: &mut LayoutCtx<M>, parent_height: i32) {
        let l = self.layout.expect(LAYOUT_ERROR);

        let target_h = match self.size.height {
            Length::Grow => parent_height,
            Length::Fixed(h) => h,
            Length::Fit => l.current_size.height,
        }
        .max(l.min.height)
        .min(l.max.height)
        .min(parent_height);

        self.size.height = Length::Fixed(target_h);

        let inner_h = target_h
            - (self.children.len() as i32 - 1) * self.spacing
            - self.padding.y
            - self.padding.w;

        let eq = equalize_sizes(&self.children, Height, Height, inner_h.max(0));
        for (i, h) in eq {
            self.children[i].grow_height(ctx, h);
        }

        if let Some(lay) = self.layout.as_mut() {
            lay.current_size.height = target_h;
        }
    }

    fn place(&mut self, ctx: &mut LayoutCtx<M>, position: Position<i32>) -> Size<i32> {
        self.position = position;
        let mut cursor = Position::new(
            self.position.x + self.padding.x,
            self.position.y + self.padding.y,
        );
        for child in self.children.iter_mut() {
            let child_size = child.place(ctx, cursor);
            cursor.y += child_size.height + self.spacing;
        }
        self.layout().current_size
    }

    fn draw(&self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        instances.push(Instance::ui(
            self.position,
            self.layout().current_size,
            self.color,
        ));
        for child in self.children.iter() {
            child.draw(ctx, instances);
        }
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        for child in self.children.iter_mut() {
            child.handle(ctx);
        }
    }
}
