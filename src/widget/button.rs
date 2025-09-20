use super::*;
use crate::{render::pipeline::PipelineKey, widget::helpers::clamp_size};

pub struct Button<M> {
    layout: Option<Layout>,

    id: Id,
    position: Position<i32>,
    size: Size<Length<i32>>,

    normal_color: Color<f32>,
    hover_color: Color<f32>,
    pressed_color: Color<f32>,

    hovered: bool,
    pressed: bool,

    min: Size<i32>,
    max: Size<i32>,

    on_press: Option<M>,
}

impl<M> Button<M> {
    pub fn new(size: Size<Length<i32>>, color: Color<f32>) -> Self {
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

    pub fn hover_color(mut self, c: Color<f32>) -> Self {
        self.hover_color = c;
        self
    }
    pub fn pressed_color(mut self, c: Color<f32>) -> Self {
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

    fn fit_size(&mut self) -> Layout {
        let current = clamp_size(self.size.into_fixed(), self.min, self.max);
        self.layout = Some(Layout {
            size: self.size,
            current_size: current,
            min: self.min,
            max: self.max,
        });
        self.layout.unwrap()
    }

    fn grow_size(&mut self, max: Size<i32>) {
        let width = match self.size.width {
            Length::Grow => max.width,
            Length::Fixed(x) => x,
            _ => 0,
        };
        let height = match self.size.height {
            Length::Grow => max.height,
            Length::Fixed(x) => x,
            _ => 0,
        };
        let clamped = clamp_size(Size::new(width, height), self.min, self.max);

        self.size.width = Length::Fixed(clamped.width);
        self.size.height = Length::Fixed(clamped.height);

        if let Some(layout) = self.layout.as_mut() {
            layout.current_size = clamped;
        }
    }

    fn place(&mut self, position: Position<i32>) -> Size<i32> {
        self.position = position;
        self.size.into_fixed()
    }

    fn draw(&self, instances: &mut Vec<Instance>) {
        let color = if self.pressed {
            self.pressed_color
        } else if self.hovered {
            self.hover_color
        } else {
            self.normal_color
        };

        instances.push(Instance::new(
            PipelineKey::Ui,
            self.position,
            self.size.into_fixed(),
            [color.r, color.g, color.b, color.a],
            [0, 0, 0, 0],
        ));
    }

    fn handle(&mut self, ctx: &mut Context<M>) {
        let was_hovered = self.hovered;
        let was_pressed = self.pressed;

        let inside = self.contains(ctx.mouse_pos);
        self.hovered = inside;
        if inside {
            ctx.hot_item = Some(self.id);
        }

        if inside && ctx.mouse_pressed {
            ctx.active_item = Some(self.id);
        }
        self.pressed = ctx.active_item == Some(self.id) && ctx.mouse_down;

        if ctx.mouse_released && ctx.active_item == Some(self.id) {
            if inside && let Some(m) = self.on_press.clone() {
                ctx.emit(m);
            }
            ctx.active_item = None;
        }

        if self.hovered != was_hovered || self.pressed != was_pressed {
            ctx.request_redraw();
        }
    }
}
