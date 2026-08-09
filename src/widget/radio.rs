use crate::event::{KeyState, LogicalKey, MouseButton, UiEventRef};

use super::*;

use std::borrow::Cow;

const SIZE: i32 = 18;
const DOT: i32 = 8;

struct RadioState {
    hovered: bool,
    pressed: bool,
}

#[derive(Widget)]
pub struct Radio<M> {
    selected: bool,
    gap: i32,
    on_select: Option<M>,

    label: Option<Element>,
}
impl<M: Clone + 'static> Radio<M> {
    pub fn new(selected: bool) -> Self {
        Self {
            selected,
            gap: 8,
            on_select: None,
            label: None,
        }
    }

    pub fn gap(mut self, n: i32) -> Self {
        self.gap = n;
        self
    }
    pub fn label<S: Into<Cow<'static, str>>>(mut self, text: S) -> Self {
        self.label = Some(
            Text::body(text)
                .wrap(crate::text::Wrap::None)
                .size(Size::splat(Length::Fit))
                .into(),
        );
        self
    }
    /// Message emitted when this radio is chosen.
    pub fn on_select(mut self, message: M) -> Self {
        self.on_select = Some(message);
        self
    }
}
impl<M: Clone + 'static> Widget for Radio<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> Node {
        let left = if self.label.is_some() {
            SIZE + self.gap
        } else {
            0
        };
        Node {
            size: Size::new(Length::Fit, Length::Fixed(SIZE)),
            min: Size::splat(SIZE),
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
    fn paint_focus_ring(&self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        use crate::focus::{GAP, RING_WIDTH};
        let r = ctx.rect();
        out.push(Instance::ui_rounded(
            Position::new((r.x - GAP) as f32, (r.y - GAP) as f32),
            Size::new((SIZE + GAP * 2) as f32, (SIZE + GAP * 2) as f32),
            Color::TRANSPARENT,
            SIZE as f32 / 2.0 + GAP as f32,
            RING_WIDTH,
            ctx.theme.focus_outline,
        ));
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        let r = ctx.rect();
        let theme = ctx.theme;

        let base = theme.surface_variant;
        let fill = if ctx.is_pressed() {
            theme.pressed(base)
        } else if ctx.is_hovered() {
            theme.hovered(base)
        } else {
            base
        };
        let border = if self.selected {
            theme.primary
        } else {
            theme.outline
        };

        out.push(Instance::ui_rounded(
            Position::new(r.x as f32, r.y as f32),
            Size::new(SIZE as f32, SIZE as f32),
            fill,
            SIZE as f32 / 2.0,
            theme.border_width.max(1),
            border,
        ));

        if self.selected {
            let off = (SIZE - DOT) as f32 / 2.0;
            out.push(Instance::ui_rounded(
                Position::new(r.x as f32 + off, r.y as f32 + off),
                Size::new(DOT as f32, DOT as f32),
                theme.primary,
                DOT as f32 / 2.0,
                0,
                Color::TRANSPARENT,
            ));
        }
    }

    fn handle_after(&mut self, ctx: &mut EventCtx) {
        let (was_hovered, was_pressed) = {
            let st = ctx.state_or(|| RadioState {
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
            let st = ctx.state_or(|| RadioState {
                hovered: false,
                pressed: false,
            });
            st.hovered = hovered;
            st.pressed = new_pressed;
        }

        let mut chosen = false;
        if mouse_released && ctx.is_pressed() {
            if hovered {
                chosen = true;
            }
            ctx.end_press();
        }
        if key_activated {
            chosen = true;
        }

        // Selecting an already-selected radio is a no-op (don't re-emit).
        if chosen
            && !self.selected
            && let Some(m) = self.on_select.clone()
        {
            ctx.emit(m);
        }

        if hovered != was_hovered || new_pressed != was_pressed || chosen {
            ctx.ui.request_redraw();
        }
    }
}

/// A vertical (or horizontal) group of mutually-exclusive [`Radio`] options.
#[derive(Widget)]
pub struct RadioGroup<M> {
    axis: Axis,
    spacing: i32,
    rows: Vec<Radio<M>>,
}
impl<M: Clone + 'static> RadioGroup<M> {
    pub fn new<I, S>(options: I, selected: Option<usize>) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Cow<'static, str>>,
    {
        Self {
            axis: Axis::Vertical,
            spacing: 8,
            rows: options
                .into_iter()
                .enumerate()
                .map(|(i, opt)| Radio::new(selected == Some(i)).label(opt))
                .collect(),
        }
    }

    pub fn horizontal(mut self) -> Self {
        self.axis = Axis::Horizontal;
        self
    }
    pub fn spacing(mut self, amount: i32) -> Self {
        self.spacing = amount;
        self
    }
    /// Space between the radio and the label.
    pub fn gap(mut self, n: i32) -> Self {
        for radio in &mut self.rows {
            radio.gap = n;
        }
        self
    }
    pub fn on_select<F>(mut self, f: F) -> Self
    where
        F: Fn(usize) -> M + 'static,
    {
        for (i, radio) in self.rows.iter_mut().enumerate() {
            radio.on_select = Some(f(i));
        }
        self
    }
}
impl<M: Clone + 'static> Widget for RadioGroup<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> Node {
        Node {
            size: Size::splat(Length::Fit),
            layout_dir: self.axis,
            spacing: self.spacing,
            ..Default::default()
        }
    }

    fn child_count(&self) -> usize {
        self.rows.len()
    }
    fn child_mut(&mut self, i: usize) -> &mut dyn Widget {
        &mut self.rows[i]
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _out: &mut InstanceStore) {}
}
