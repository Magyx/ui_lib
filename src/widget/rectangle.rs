use super::*;

pub struct Rectangle {
    layout: Option<Layout>,

    id: Id,
    position: Position<i32>,
    size: Size<Length<i32>>,
    color: Color,

    min: Size<i32>,
    max: Size<i32>,
}

impl Rectangle {
    pub fn new(size: Size<Length<i32>>, color: Color) -> Self {
        Self {
            layout: None,

            id: crate::context::next_id(),
            position: Position::splat(0),
            size,
            color,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
        }
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

impl<M> Widget<M> for Rectangle {
    fn position(&self) -> &Position<i32> {
        &self.position
    }

    fn layout(&self) -> &Layout {
        self.layout.as_ref().expect(LAYOUT_ERROR)
    }

    fn id(&self) -> Id {
        self.id
    }

    fn fit_width(&mut self, _ctx: &mut LayoutCtx<M>) -> Layout {
        let base_w = match self.size.width {
            Length::Fixed(w) => w,
            _ => 0,
        };
        let cur_w = base_w.clamp(self.min.width, self.max.width);

        let l = Layout {
            size: self.size,
            current_size: Size::new(cur_w, 0),
            min: self.min,
            max: self.max,
        };
        self.layout = Some(l);
        l
    }

    fn grow_width(&mut self, _ctx: &mut LayoutCtx<M>, parent_width: i32) {
        let l = self.layout.as_ref().expect(LAYOUT_ERROR);

        let target_w = match self.size.width {
            Length::Grow => parent_width,
            Length::Fixed(w) => w,
            Length::Fit => l.current_size.width,
        };

        let final_w = target_w
            .max(self.min.width)
            .min(self.max.width)
            .min(parent_width);

        if let Some(m) = self.layout.as_mut() {
            m.current_size.width = final_w;
        }
        self.size.width = Length::Fixed(final_w);
    }

    fn fit_height(&mut self, _ctx: &mut LayoutCtx<M>) -> Layout {
        let base_h = match self.size.height {
            Length::Fixed(h) => h,
            _ => 0,
        };
        let cur_h = base_h.clamp(self.min.height, self.max.height);

        let cur_w = self.layout.map(|l| l.current_size.width).unwrap_or(0);

        let l = Layout {
            size: self.size,
            current_size: Size::new(cur_w, cur_h),
            min: self.min,
            max: self.max,
        };
        self.layout = Some(l);
        l
    }

    fn grow_height(&mut self, _ctx: &mut LayoutCtx<M>, parent_height: i32) {
        let l = self.layout.as_ref().expect(LAYOUT_ERROR);
        let target_h = match self.size.height {
            Length::Grow => parent_height,
            Length::Fixed(h) => h,
            Length::Fit => l.current_size.height,
        };

        let final_h = target_h
            .max(self.min.height)
            .min(self.max.height)
            .min(parent_height);

        if let Some(m) = self.layout.as_mut() {
            m.current_size.height = final_h;
        }
        self.size.height = Length::Fixed(final_h);
    }

    fn place(&mut self, _ctx: &mut LayoutCtx<M>, position: Position<i32>) -> Size<i32> {
        self.position = position;
        <Rectangle as Widget<M>>::layout(self).current_size
    }

    fn draw_self(&self, _ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        if self.color.a() != Color::TRANSPARENT.a() {
            instances.push(Instance::ui(
                self.position,
                <Rectangle as Widget<M>>::layout(self).current_size,
                self.color,
            ));
        }
    }
}
