use std::borrow::Cow;

use crate::{
    event::{KeyState, LogicalKey, MouseButton, UiEventRef},
    text::{FontStyle, RunStyle, TextBuffer, TextMetrics, Weight, Wrap},
};

use super::*;

/// Font size used to shape glyph-backed marks.
const MARK_FONT: f32 = 14.0;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Mark {
    #[default]
    /// A check `✓` (U+2713). The default.
    Check,
    /// A ballot `✗` (U+2717).
    Cross,
    /// Any author-supplied glyph, shaped via the text backend.
    Glyph(char),
    /// A filled dot, drawn as a primitive (no text).
    Dot,
}

impl Mark {
    /// The glyph to shape, or `None` for marks drawn as primitives.
    #[inline]
    fn glyph(self) -> Option<char> {
        match self {
            Mark::Check => Some('\u{2713}'),
            Mark::Cross => Some('\u{2717}'),
            Mark::Glyph(c) => Some(c),
            Mark::Dot => None,
        }
    }
}

/// Tri-state checkbox value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckState {
    /// Empty box.
    Off,
    /// The chosen [`Mark`] is shown.
    On,
    /// Indeterminate: a horizontal bar is shown.
    Mixed,
}
impl CheckState {
    /// State produced by a click. Follows the platform-standard cycle, where
    /// an indeterminate box resolves to [`On`](CheckState::On).
    #[inline]
    fn toggled(self) -> Self {
        match self {
            CheckState::Off | CheckState::Mixed => CheckState::On,
            CheckState::On => CheckState::Off,
        }
    }
}
impl From<bool> for CheckState {
    #[inline]
    fn from(value: bool) -> Self {
        if value {
            CheckState::On
        } else {
            CheckState::Off
        }
    }
}
impl From<CheckState> for bool {
    #[inline]
    fn from(value: CheckState) -> Self {
        match value {
            CheckState::Off | CheckState::Mixed => false,
            CheckState::On => true,
        }
    }
}

struct CheckboxState {
    hovered: bool,
    pressed: bool,
    /// Lazily created the first time a glyph-backed mark is shown.
    buffer: Option<Box<dyn TextBuffer>>,
    /// Which glyph the buffer currently holds shaped/uploaded.
    shaped: Option<char>,
    /// Offset that centers the shaped glyph inside the box.
    mark_offset: Position<f32>,
}
impl Default for CheckboxState {
    fn default() -> Self {
        Self {
            hovered: false,
            pressed: false,
            buffer: None,
            shaped: None,
            mark_offset: Position::splat(0.0),
        }
    }
}

pub struct Checkbox<M> {
    state: CheckState,
    mark: Mark,
    disabled: bool,
    gap: i32,
    size: i32,
    on_toggle: Option<Box<dyn Fn(CheckState) -> M>>,
    label: Option<Element>,
}

impl<M: 'static> Checkbox<M> {
    /// A checkbox in the given state. Accepts a `bool` (`true` -> `On`) or a
    /// [`CheckState`] directly.
    pub fn new(state: impl Into<CheckState>) -> Self {
        Self {
            state: state.into(),
            mark: Mark::default(),
            disabled: false,
            gap: 8,
            size: 18,
            on_toggle: None,
            label: None,
        }
    }

    /// Choose what the box draws when [`On`](CheckState::On).
    pub fn mark(mut self, mark: Mark) -> Self {
        self.mark = mark;
        self
    }
    /// A trailing label. Toggling the label toggles the box.
    pub fn label<S: Into<Cow<'static, str>>>(mut self, text: S) -> Self {
        self.label = Some(
            Text::body(text)
                .wrap(Wrap::None)
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
    /// Size of the box.
    pub fn size(mut self, n: i32) -> Self {
        self.size = n;
        self
    }
    /// Emitted on toggle with the *next* state (see [`CheckState::toggled`]).
    pub fn on_toggle<F>(mut self, f: F) -> Self
    where
        F: Fn(CheckState) -> M + 'static,
    {
        self.on_toggle = Some(Box::new(f));
        self
    }
}

impl<M> Checkbox<M> {
    #[inline]
    fn visible_glyph(&self) -> Option<char> {
        if self.state == CheckState::On {
            self.mark.glyph()
        } else {
            None
        }
    }

    fn run_style(&self) -> RunStyle {
        RunStyle {
            metrics: TextMetrics::new(MARK_FONT, 1.0),
            family: None,
            weight: Weight::BOLD,
            style: FontStyle::Normal,
            wrap: Wrap::None,
            color: None, // tinted at paint time
        }
    }

    #[inline]
    fn mark_color(&self, theme: &Theme) -> Color {
        if self.disabled {
            theme.on_surface_variant
        } else {
            theme.on_primary
        }
    }

    /// Box fill and border, accounting for state, hover/press, and disabled.
    fn box_colors(&self, hovered: bool, pressed: bool, theme: &Theme) -> (Color, Color) {
        if self.disabled {
            return (theme.surface_variant, theme.outline_variant);
        }
        let filled = matches!(self.state, CheckState::On | CheckState::Mixed);
        let base = if filled {
            theme.primary
        } else {
            theme.surface_variant
        };
        let fill = if pressed {
            theme.pressed(base)
        } else if hovered {
            theme.hovered(base)
        } else {
            base
        };
        let border = if filled { fill } else { theme.outline };
        (fill, border)
    }
}

impl<M> IntoElement for Checkbox<M> {}

impl<M: 'static> Widget for Checkbox<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> Node {
        let left = if self.label.is_some() {
            self.size + self.gap
        } else {
            0
        };
        Node {
            size: Size::new(Length::Fit, Length::Fixed(self.size)),
            min: Size::splat(self.size),
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
        !self.disabled
    }
    fn paint_focus_ring(&self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        use crate::focus::{GAP, RING_WIDTH};
        let r = ctx.rect();
        out.push(Instance::ui_rounded(
            Position::new((r.x - GAP) as f32, (r.y - GAP) as f32),
            Size::new((self.size + GAP * 2) as f32, (self.size + GAP * 2) as f32),
            Color::TRANSPARENT,
            ctx.theme.corner_radius + GAP as f32 * 2.0,
            RING_WIDTH,
            ctx.theme.focus_outline,
        ));
    }

    fn prepare(&mut self, ctx: &mut PrepareCtx) {
        let Some(ch) = self.visible_glyph() else {
            return;
        };
        let id = ctx.id();
        let metrics = TextMetrics::new(MARK_FONT, 1.0);

        // Disjoint-field borrow: text backend + view_state (mirrors Text).
        let text = &mut *ctx.text;
        let st = ctx.view_state.ensure(id, CheckboxState::default);

        if st.buffer.is_none() {
            st.buffer = Some(text.create_buffer(metrics));
        }
        let buf = st.buffer.as_mut().unwrap();

        if st.shaped != Some(ch) {
            buf.set_style(&self.run_style());
            let mut utf8 = [0u8; 4];
            buf.set_text(ch.encode_utf8(&mut utf8));
            buf.set_width(None);
            buf.shape();
            let m = buf.measured().size;
            st.mark_offset = Position::new(
                (self.size as f32 - m.width) / 2.0,
                (self.size as f32 - m.height) / 2.0,
            );
            st.shaped = Some(ch);
        }

        buf.prepare(ctx.gpu, ctx.texture);
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        let r = ctx.rect();
        let id = ctx.id();
        let theme = ctx.theme;
        let radius = theme.corner_radius.max(3.0);

        let (fill, border) = self.box_colors(ctx.is_hovered(), ctx.is_pressed(), theme);

        let bx = r.x as f32;
        let by = r.y as f32;
        out.push(Instance::ui_rounded(
            Position::new(bx, by),
            Size::new(self.size as f32, self.size as f32),
            fill,
            radius,
            theme.border_width,
            border,
        ));

        let mark = self.mark_color(theme);

        match self.state {
            CheckState::Off => {}
            CheckState::Mixed => {
                // Centered horizontal bar.
                let w = self.size as f32 * 0.55;
                let h = (self.size as f32 * 0.14).max(2.0);
                out.push(Instance::ui_rounded(
                    Position::new(
                        bx + (self.size as f32 - w) / 2.0,
                        by + (self.size as f32 - h) / 2.0,
                    ),
                    Size::new(w, h),
                    mark,
                    h / 2.0,
                    0,
                    Color::TRANSPARENT,
                ));
            }
            CheckState::On => {
                if self.mark.glyph().is_some() {
                    if let Some(st) = ctx.view_state.get::<CheckboxState>(&id)
                        && let Some(buf) = st.buffer.as_ref()
                    {
                        let ox = bx + st.mark_offset.x;
                        let oy = by + st.mark_offset.y;
                        for g in buf.glyphs() {
                            out.push(Instance::ui_tex(
                                Position::new(ox + g.pos.x, oy + g.pos.y),
                                g.size,
                                g.color.unwrap_or(mark),
                                g.handle,
                            ));
                        }
                    }
                } else {
                    // Dot: centered rounded square.
                    let d = self.size as f32 * 0.5;
                    out.push(Instance::ui_rounded(
                        Position::new(
                            bx + (self.size as f32 - d) / 2.0,
                            by + (self.size as f32 - d) / 2.0,
                        ),
                        Size::new(d, d),
                        mark,
                        d / 2.0,
                        0,
                        Color::TRANSPARENT,
                    ));
                }
            }
        }
    }

    fn handle_after(&mut self, ctx: &mut EventCtx) {
        if self.disabled {
            return;
        }

        let (was_hovered, was_pressed) = {
            let st = ctx.state_or(CheckboxState::default);
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
            let st = ctx.state_or(CheckboxState::default);
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
            ctx.emit(f(self.state.toggled()));
        }

        if hovered != was_hovered || new_pressed != was_pressed || toggled {
            ctx.ui.request_redraw();
        }
    }
}
