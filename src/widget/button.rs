use super::*;

pub struct Button<M> {
    layout: Option<Layout>,

    id: Id,
    position: Position<i32>,
    size: Size<Length<i32>>,
    content: Option<Element<M>>,

    normal_color: Color,
    hover_color: Color,
    pressed_color: Color,

    hovered: bool,
    pressed: bool,

    min: Size<i32>,
    max: Size<i32>,

    on_press: Option<M>,
}

impl<M: Clone + 'static> Button<M> {
    pub fn new(size: Size<Length<i32>>, color: Color) -> Self {
        Self {
            layout: None,

            id: crate::context::next_id(),
            position: Position::splat(0),
            size,
            content: None,

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

    pub fn new_with(content: Element<M>) -> Self {
        Self {
            layout: None,

            id: crate::context::next_id(),
            position: Position::splat(0),
            size: Size::splat(Length::Fit),
            content: Some(content),

            normal_color: Color::TRANSPARENT,
            hover_color: Color::TRANSPARENT,
            pressed_color: Color::TRANSPARENT,

            hovered: false,
            pressed: false,

            min: Size::splat(0),
            max: Size::splat(i32::MAX),

            on_press: None,
        }
    }

    pub fn color(mut self, c: Color) -> Self {
        self.normal_color = c;
        self
    }
    pub fn hover_color(mut self, c: Color) -> Self {
        self.hover_color = c;
        self
    }
    pub fn pressed_color(mut self, c: Color) -> Self {
        self.pressed_color = c;
        self
    }
    pub fn size(mut self, size: Size<Length<i32>>) -> Self {
        self.size = size;
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
        let sz = self.layout().current_size;
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
    fn position(&self) -> &Position<i32> {
        &self.position
    }
    fn layout(&self) -> &Layout {
        self.layout.as_ref().expect(LAYOUT_ERROR)
    }

    fn for_each_child(&self, f: &mut dyn for<'a> FnMut(&'a dyn Widget<M>)) {
        if let Some(child) = &self.content {
            f(child.as_ref());
        }
    }

    fn fit_width(&mut self, ctx: &mut LayoutCtx<M>) -> Layout {
        let mut min_w = 0;
        if let Some(child) = self.content.as_mut() {
            let Layout { current_size, .. } = child.fit_width(ctx);
            min_w = min_w.max(current_size.width);
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
        let l = self.layout.as_mut().expect(LAYOUT_ERROR);

        let target_w = match self.size.width {
            Length::Grow => parent_width,
            Length::Fixed(w) => w,
            Length::Fit => l.current_size.width,
        }
        .max(l.min.width)
        .min(l.max.width)
        .min(parent_width);

        // Propagate width to content
        if let Some(child) = self.content.as_mut() {
            child.grow_width(ctx, target_w);
        }

        l.current_size.width = target_w;
    }

    fn fit_height(&mut self, ctx: &mut LayoutCtx<M>) -> Layout {
        let mut min_h = 0;
        if let Some(child) = self.content.as_mut() {
            let Layout { current_size, .. } = child.fit_height(ctx);
            min_h = min_h.max(current_size.height);
        }

        let prev = self.layout.as_ref().expect(LAYOUT_ERROR);
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
        let l = self.layout.as_mut().expect(LAYOUT_ERROR);

        let target_h = match self.size.height {
            Length::Grow => parent_height,
            Length::Fixed(h) => h,
            Length::Fit => l.current_size.height,
        }
        .max(l.min.height)
        .min(l.max.height)
        .min(parent_height);

        if let Some(child) = self.content.as_mut() {
            child.grow_height(ctx, target_h);
        }

        l.current_size.height = target_h;
    }

    fn place(&mut self, ctx: &mut LayoutCtx<M>, position: Position<i32>) -> Size<i32> {
        self.position = position;

        if let Some(child) = self.content.as_mut() {
            let _ = child.place(ctx, self.position);
        }

        self.layout().current_size
    }

    fn draw_self(&self, _ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
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
        if let Some(child) = self.content.as_mut() {
            child.handle(ctx);
        }

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
