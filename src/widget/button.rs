use crate::event::MouseButton;

use super::*;

struct ButtonState {
    hovered: bool,
    pressed: bool,
}

pub struct Button<M> {
    x: i32,
    y: i32,
    w: i32,
    h: i32,

    id: Id,
    size: Size<Length>,
    content: Option<Element<M>>,

    normal_color: Color,
    hover_color: Color,
    pressed_color: Color,

    min: Size<i32>,
    max: Size<i32>,

    on_press: Option<M>,
}

impl<M: Clone + 'static> Button<M> {
    pub fn new(size: Size<Length>, color: Color) -> Self {
        Self {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            id: 0,
            size,
            content: None,
            normal_color: color,
            hover_color: color,
            pressed_color: color,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            on_press: None,
        }
    }

    pub fn new_with<E>(content: E) -> Self
    where
        E: Into<Element<M>>,
    {
        Self {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            id: 0,
            size: Size::splat(Length::Fit),
            content: Some(content.into()),
            normal_color: Color::TRANSPARENT,
            hover_color: Color::TRANSPARENT,
            pressed_color: Color::TRANSPARENT,
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
    pub fn size(mut self, size: Size<Length>) -> Self {
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
        let l = self.x as f32;
        let t = self.y as f32;
        let r = l + self.w as f32;
        let b = t + self.h as f32;
        p.x >= l && p.x < r && p.y >= t && p.y < b
    }
}

impl<M> IntoElement for Button<M> {}

impl<M: Clone + 'static> Widget<M> for Button<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            layout_dir: Axis::Horizontal,
            ..Default::default()
        }
    }

    fn set_layout(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.x = x;
        self.y = y;
        self.w = w;
        self.h = h;
    }

    fn set_id(&mut self, id: Id) {
        self.id = id;
    }

    fn child_count(&self) -> usize {
        if self.content.is_some() { 1 } else { 0 }
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        self.content.as_mut().unwrap().as_mut()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        let st = ctx.view_state.get::<ButtonState>(&self.id);
        let hovered = st.is_some_and(|s| s.hovered);
        let pressed = st.is_some_and(|s| s.pressed);
        let color = if pressed {
            self.pressed_color
        } else if hovered {
            self.hover_color
        } else {
            self.normal_color
        };
        instances.push(Instance::ui(
            Position::new(self.x as f32, self.y as f32),
            Size::new(self.w as f32, self.h as f32),
            color,
        ));
    }

    fn handle_after(&mut self, ctx: &mut EventCtx<M>) {
        let (was_hovered, was_pressed) = {
            let st = ctx.ui.view_state.ensure(self.id, || ButtonState {
                hovered: false,
                pressed: false,
            });
            (st.hovered, st.pressed)
        };

        let hovered = self.contains(ctx.ui.mouse_pos);
        let mouse_pressed = ctx.is_mouse_pressed(MouseButton::Left);
        let mouse_released = ctx.is_mouse_released(MouseButton::Left);

        if hovered {
            ctx.ui.hot_item = Some(self.id);
        }
        if hovered && mouse_pressed {
            ctx.ui.active_item = Some(self.id);
        }

        let new_pressed =
            ctx.ui.active_item == Some(self.id) && ctx.ui.is_button_down(MouseButton::Left);

        let st = ctx.ui.view_state.ensure(self.id, || ButtonState {
            hovered: false,
            pressed: false,
        });
        st.hovered = hovered;
        st.pressed = new_pressed;

        if mouse_released && ctx.ui.active_item == Some(self.id) {
            if hovered && let Some(m) = self.on_press.clone() {
                ctx.ui.emit(m);
            }
            ctx.ui.active_item = None;
        }

        if hovered != was_hovered || new_pressed != was_pressed {
            ctx.ui.request_redraw();
        }
    }
}
