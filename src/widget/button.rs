use super::*;

pub struct Button<M> {
    layout: Option<Layout>,

    id: Id,
    position: Position<i32>,
    size: Size<Length<i32>>,

    normal_color: Color,
    hover_color: Color,
    pressed_color: Color,

    hovered: bool,
    pressed: bool,

    min: Size<i32>,
    max: Size<i32>,

    on_press: Option<M>,
}

impl<M> Button<M> {
    pub fn new(size: Size<Length<i32>>, color: Color) -> Self {
        Self {
            layout: None,

            id: crate::context::next_id(),
            position: Position::splat(0),
            size,

            normal_color: color,
            hover_color: color,
            pressed_color: color,

            hovered: false,
            pressed: false,

            min: Size::splat(0),
            max: Size::splat(i32::MAX),

            on_press: None,
        }
    }

    pub fn hover_color(mut self, c: Color) -> Self {
        self.hover_color = c;
        self
    }
    pub fn pressed_color(mut self, c: Color) -> Self {
        self.pressed_color = c;
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
    pub fn on_press(mut self, msg: M) -> Self {
        self.on_press = Some(msg);
        self
    }

    #[inline]
    fn contains(&self, p: Position<f32>) -> bool {
        let sz = self.size.into_fixed();
        let l = self.position.x as f32;
        let t = self.position.y as f32;
        let r = l + sz.width as f32;
        let b = t + sz.height as f32;
        p.x >= l && p.x < r && p.y >= t && p.y < b
    }
}

impl<M: Clone + 'static> Widget<M> for Button<M> {
    fn id(&self) -> Id {
        self.id
    }

    fn layout(&self) -> Layout {
        self.layout.expect(LAYOUT_ERROR)
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
        self.layout().current_size
    }

    fn draw(&self, _ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        let color = if self.pressed {
            self.pressed_color
        } else if self.hovered {
            self.hover_color
        } else {
            self.normal_color
        };

        instances.push(Instance::ui(
            self.position,
            self.layout().current_size,
            color,
        ));
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        let was_hovered = self.hovered;
        let was_pressed = self.pressed;

        let inside = self.contains(ctx.ui.mouse_pos);
        self.hovered = inside;
        if inside {
            ctx.ui.hot_item = Some(self.id);
        }

        if inside && ctx.ui.mouse_pressed {
            ctx.ui.active_item = Some(self.id);
        }
        self.pressed = ctx.ui.active_item == Some(self.id) && ctx.ui.mouse_down;

        if ctx.ui.mouse_released && ctx.ui.active_item == Some(self.id) {
            if inside && let Some(m) = self.on_press.clone() {
                ctx.ui.emit(m);
            }
            ctx.ui.active_item = None;
        }

        if self.hovered != was_hovered || self.pressed != was_pressed {
            ctx.ui.request_redraw();
        }
    }
}
