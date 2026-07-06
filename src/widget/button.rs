use crate::event::MouseButton;

use super::*;

struct ButtonState {
    hovered: bool,
    pressed: bool,
}

pub struct Button<M> {
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,

    content: Option<Element<M>>,

    normal_color: Option<Color>,
    hover_color: Option<Color>,
    pressed_color: Option<Color>,
    border: bool,

    on_press: Option<M>,
}

impl<M: Clone + 'static> Button<M> {
    pub fn new(size: Size<Length>, color: Color) -> Self {
        Self {
            size,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            content: None,
            normal_color: Some(color),
            hover_color: Some(color),
            pressed_color: Some(color),
            border: false,
            on_press: None,
        }
    }

    pub fn new_with<E>(content: E) -> Self
    where
        E: Into<Element<M>>,
    {
        Self {
            size: Size::splat(Length::Fit),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            content: Some(content.into()),
            normal_color: None,
            hover_color: None,
            pressed_color: None,
            border: false,
            on_press: None,
        }
    }

    pub fn color(mut self, c: Color) -> Self {
        self.normal_color = Some(c);
        self
    }
    pub fn hover_color(mut self, c: Color) -> Self {
        self.hover_color = Some(c);
        self
    }
    pub fn pressed_color(mut self, c: Color) -> Self {
        self.pressed_color = Some(c);
        self
    }
    pub fn border(mut self) -> Self {
        self.border = true;
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

    fn child_count(&self) -> usize {
        if self.content.is_some() { 1 } else { 0 }
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        self.content.as_mut().unwrap().as_mut()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        let ctx_rect = ctx.rect();
        let id = ctx.id();
        let st = ctx.view_state.get::<ButtonState>(&id);
        let hovered = st.is_some_and(|s| s.hovered);
        let pressed = st.is_some_and(|s| s.pressed);
        let theme = ctx.theme;
        let fill = if pressed {
            self.pressed_color.unwrap_or(theme.primary_container)
        } else if hovered {
            self.hover_color.unwrap_or(theme.primary_container)
        } else {
            self.normal_color.unwrap_or(theme.primary)
        };
        let border = if self.border {
            theme.outline
        } else {
            Color::TRANSPARENT
        };
        ctx.surface(instances, ctx_rect.xywh(), fill, border);
    }

    fn handle_after(&mut self, ctx: &mut EventCtx<M>) {
        let id = ctx.id();
        let (was_hovered, was_pressed) = {
            let st = ctx.state_or(|| ButtonState {
                hovered: false,
                pressed: false,
            });
            (st.hovered, st.pressed)
        };

        let hovered = ctx.rect().contains(ctx.ui.mouse_pos);
        let mouse_pressed = ctx.is_mouse_pressed(MouseButton::Left);
        let mouse_released = ctx.is_mouse_released(MouseButton::Left);

        if hovered {
            ctx.ui.hot_item = Some(id);
        }
        if hovered && mouse_pressed {
            ctx.ui.active_item = Some(id);
        }

        let new_pressed =
            ctx.ui.active_item == Some(id) && ctx.ui.is_button_down(MouseButton::Left);

        let st = ctx.state_or(|| ButtonState {
            hovered: false,
            pressed: false,
        });
        st.hovered = hovered;
        st.pressed = new_pressed;

        if mouse_released && ctx.ui.active_item == Some(id) {
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
