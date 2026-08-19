use crate::{
    event::{KeyState, LogicalKey, MouseButton, UiEventRef},
    theme::Style,
    widget::prelude::*,
};

struct ButtonState {
    hovered: bool,
    pressed: bool,
}

#[derive(Widget)]
pub struct Button<M> {
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,

    content: Option<Element>,

    style: Style,
    border: bool,
    can_focus: bool,

    on_press: Option<M>,
}
impl<M: Clone> Button<M> {
    pub fn new(size: Size<Length>, color: Color) -> Self {
        Self {
            size,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            content: None,
            style: Style {
                fill: Some(color),
                ..Default::default()
            },
            border: false,
            can_focus: true,
            on_press: None,
        }
    }

    pub fn new_with<E>(content: E) -> Self
    where
        E: Into<Element>,
    {
        Self {
            size: Size::splat(Length::Fit),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            content: Some(content.into()),
            style: Style::default(),
            border: false,
            can_focus: true,
            on_press: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.style.fill = Some(color);
        self
    }
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
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
    pub fn can_focus(mut self, enabled: bool) -> Self {
        self.can_focus = enabled;
        self
    }
    pub fn on_press(mut self, msg: M) -> Self {
        self.on_press = Some(msg);
        self
    }
}
impl<M: Clone + 'static> Widget for Button<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            layout_dir: Axis::Horizontal,
            cross_align: Align::Center,
            main_align: Align::Center,
            ..Default::default()
        }
    }

    fn child_count(&self) -> usize {
        self.content.is_some() as usize
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget {
        self.content.as_mut().unwrap().as_mut()
    }
    fn child_env(&self, env: Env, theme: &Theme) -> Env {
        Env {
            foreground: theme.on_primary,
            ..env
        }
    }

    fn focusable(&self) -> bool {
        self.can_focus
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        let ctx_rect = ctx.rect();
        let id = ctx.id();
        let st = ctx.view_state.get::<ButtonState>(&id);
        let hovered = st.is_some_and(|s| s.hovered);
        let pressed = st.is_some_and(|s| s.pressed);
        let theme = ctx.theme;
        let base = self.style.fill_or(theme.primary);
        let fill = if pressed {
            theme.pressed(base)
        } else if hovered {
            theme.hovered(base)
        } else {
            base
        };
        let border = if self.border {
            self.style.border_or(theme.outline)
        } else {
            Color::TRANSPARENT
        };
        ctx.surface(out, ctx_rect.xywh(), fill, border);
    }

    fn handle_after(&mut self, ctx: &mut EventCtx) {
        let (was_hovered, was_pressed) = {
            let st = ctx.state_or(|| ButtonState {
                hovered: false,
                pressed: false,
            });
            (st.hovered, st.pressed)
        };

        let hovered = ctx.pointer_over();
        let mouse_pressed = ctx.is_mouse_pressed(MouseButton::Left);
        let mouse_released = ctx.is_mouse_released(MouseButton::Left);

        if hovered {
            ctx.claim_hover();
        }
        if hovered && mouse_pressed {
            ctx.begin_press();
            ctx.request_focus();
        }

        let new_pressed = ctx.is_pressed() && ctx.ui.is_button_down(MouseButton::Left);

        let key_activated = ctx.is_focused()
            && matches!(
                ctx.event,
                Some(UiEventRef::Key(k))
                    if k.state == KeyState::Pressed
                        && matches!(k.logical_key, LogicalKey::Enter | LogicalKey::Space)
            );

        {
            let st = ctx.state_or(|| ButtonState {
                hovered: false,
                pressed: false,
            });
            st.hovered = hovered;
            st.pressed = new_pressed;
        }

        if mouse_released && ctx.is_pressed() {
            if hovered && let Some(m) = self.on_press.clone() {
                ctx.emit(m);
            }
            ctx.end_press();
        }

        if key_activated && let Some(m) = self.on_press.clone() {
            ctx.emit(m);
        }

        if hovered != was_hovered || new_pressed != was_pressed || key_activated {
            ctx.ui.request_redraw();
        }
    }
}
