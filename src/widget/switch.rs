use crate::event::{KeyState, LogicalKey, MouseButton, UiEventRef};

use super::*;

use std::borrow::Cow;

const TRACK_W: i32 = 40;
const TRACK_H: i32 = 22;
const KNOB: i32 = 18;
const PAD: i32 = 2;

#[derive(Default)]
struct SwitchState {
    hovered: bool,
    pressed: bool,
}

pub struct Switch<M> {
    on: bool,
    disabled: bool,
    gap: i32,
    on_toggle: Option<Box<dyn Fn(bool) -> M>>,
    label: Option<Element>,
}

impl<M: 'static> Switch<M> {
    pub fn new(on: bool) -> Self {
        Self {
            on,
            disabled: false,
            gap: 8,
            on_toggle: None,
            label: None,
        }
    }

    /// A trailing label. Toggling the label toggles the box.
    pub fn label<S: Into<Cow<'static, str>>>(mut self, text: S) -> Self {
        self.label = Some(
            Text::body(text)
                .wrap(crate::text::Wrap::None)
                .size(Size::splat(Length::Fit))
                .into(),
        );
        self
    }
    /// Disable interaction and mute the colors.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
    /// Space between the box and the label.
    pub fn gap(mut self, n: i32) -> Self {
        self.gap = n;
        self
    }
    /// Emitted on toggle.
    pub fn on_toggle<F>(mut self, f: F) -> Self
    where
        F: Fn(bool) -> M + 'static,
    {
        self.on_toggle = Some(Box::new(f));
        self
    }
}

impl<M> IntoElement for Switch<M> {}

impl<M: 'static> Widget for Switch<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> Node {
        let left = if self.label.is_some() {
            TRACK_W + self.gap
        } else {
            0
        };
        Node {
            size: Size::new(Length::Fit, Length::Fixed(TRACK_H)),
            min: Size::new(TRACK_W, TRACK_H),
            layout_dir: Axis::Horizontal,
            cross_align: Align::Center,
            padding: Padding {
                left,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn child_count(&self) -> usize {
        self.label.is_some() as usize
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget {
        self.label.as_mut().expect("label requested").as_mut()
    }

    fn focusable(&self) -> bool {
        true
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        let r = ctx.rect();
        let theme = ctx.theme;

        let base = if self.on {
            theme.primary
        } else {
            theme.surface_variant
        };
        let track = if ctx.is_pressed() {
            theme.pressed(base)
        } else if ctx.is_hovered() {
            theme.hovered(base)
        } else {
            base
        };
        let border = if self.on {
            Color::TRANSPARENT
        } else {
            theme.outline
        };

        out.push(Instance::ui_rounded(
            Position::new(r.x as f32, r.y as f32),
            Size::new(TRACK_W as f32, TRACK_H as f32),
            track,
            TRACK_H as f32 / 2.0,
            theme.border_width,
            border,
        ));

        let knob_x = if self.on {
            r.x + TRACK_W - KNOB - PAD
        } else {
            r.x + PAD
        };
        let knob_color = if self.on {
            theme.on_primary
        } else {
            theme.on_surface_variant
        };
        out.push(Instance::ui_rounded(
            Position::new(knob_x as f32, (r.y + PAD) as f32),
            Size::new(KNOB as f32, KNOB as f32),
            knob_color,
            KNOB as f32 / 2.0,
            0,
            Color::TRANSPARENT,
        ));
    }

    fn paint_focus_ring(&self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        use crate::focus::{GAP, RING_WIDTH};
        let r = ctx.rect();
        out.push(Instance::ui_rounded(
            Position::new((r.x - GAP) as f32, (r.y - GAP) as f32),
            Size::new((TRACK_W + GAP * 2) as f32, (TRACK_H + GAP * 2) as f32),
            Color::TRANSPARENT,
            TRACK_H as f32 / 2.0 + GAP as f32,
            RING_WIDTH,
            ctx.theme.focus_outline,
        ));
    }

    fn handle_after(&mut self, ctx: &mut EventCtx) {
        if self.disabled {
            return;
        }

        let (was_hovered, was_pressed) = {
            let st = ctx.state_or(SwitchState::default);
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
            let st = ctx.state_or(SwitchState::default);
            st.hovered = hovered;
            st.pressed = new_pressed;
        }

        let mut toggled = false;
        if mouse_released && ctx.is_pressed() {
            if hovered {
                toggled = true;
            }
            ctx.end_press();
        }
        if key_activated {
            toggled = true;
        }

        if toggled && let Some(f) = &self.on_toggle {
            ctx.emit(f(!self.on));
        }

        if hovered != was_hovered || new_pressed != was_pressed || toggled {
            ctx.ui.request_redraw();
        }
    }
}
