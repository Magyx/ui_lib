use std::borrow::Cow;

use crate::{
    text::{RunStyle, Style, TextBackend, TextBuffer, TextMetrics, Weight, Wrap},
    theme::{TextStyle, Theme, Typography},
};

use super::*;

pub(super) struct TextViewState {
    pub(super) buffer: Box<dyn TextBuffer>,

    pub(super) shaped_text: Option<String>,
    shaped_width: f32,
    shaped_wrap: Wrap,

    layout_text: Option<String>,
    layout_font_size: f32,
    layout_line_height: f32,
    layout_wrap: Wrap,
    layout_family: Option<Family>,
    layout_style: Option<Style>,
    layout_weight: Option<Weight>,
    layout_intrinsic_w: i32,
    layout_line_count: usize,
}

impl TextViewState {
    pub(super) fn new(buffer: Box<dyn TextBuffer>) -> Self {
        Self {
            buffer,
            shaped_text: None,
            shaped_width: 0.0,
            shaped_wrap: Wrap::None,
            layout_text: None,
            layout_font_size: 0.0,
            layout_line_height: 0.0,
            layout_wrap: Wrap::None,
            layout_family: None,
            layout_style: None,
            layout_weight: None,
            layout_intrinsic_w: 0,
            layout_line_count: 0,
        }
    }

    fn layout_hit(&self, widget: &Text, rs: &TextStyle) -> bool {
        self.layout_text.as_deref() == Some(&*widget.text)
            && self.layout_font_size == rs.font_size
            && self.layout_line_height == rs.line_height
            && self.layout_wrap == widget.wrap
            && self.layout_family == widget.family
            && self.layout_style == widget.style
            && self.layout_weight == widget.weight
    }

    fn ensure_layout(&mut self, widget: &Text, rs: &TextStyle) -> (i32, usize) {
        if self.layout_hit(widget, rs) {
            return (self.layout_intrinsic_w, self.layout_line_count);
        }

        let wrap = widget.wrap;

        // (a) Unwrapped measurement.
        let style_unwrapped = widget.run_style(rs, Wrap::None);
        self.buffer.set_style(&style_unwrapped);
        self.buffer.set_text(&widget.text);
        self.buffer.set_width(None);
        self.buffer.shape();
        let unwrapped_w = self.buffer.measured().size.width;
        let line_count = self.buffer.line_count();
        let unwrapped_w_i = unwrapped_w.ceil() as i32;

        // (b) Minimal useful width with wrapping.
        let min_break_w_i = if wrap != Wrap::None {
            let mut longest = 0.0f32;
            for piece in widget.text.split_whitespace().filter(|s| !s.is_empty()) {
                self.buffer.set_text(piece);
                self.buffer.set_width(None);
                self.buffer.shape();
                longest = longest.max(self.buffer.measured().size.width);
            }
            longest.ceil() as i32
        } else {
            unwrapped_w_i
        };

        let intrinsic_w = if wrap == Wrap::None {
            unwrapped_w_i
        } else {
            min_break_w_i
        };

        // The per-word probing above left the buffer holding a stray word; the
        // final wrapped shape happens in `ensure_shaped`, so invalidate.
        self.shaped_text = None;

        self.layout_text = Some(widget.text.to_string());
        self.layout_font_size = rs.font_size;
        self.layout_line_height = rs.line_height;
        self.layout_wrap = widget.wrap;
        self.layout_family = widget.family.clone();
        self.layout_style = widget.style;
        self.layout_weight = widget.weight;
        self.layout_intrinsic_w = intrinsic_w;
        self.layout_line_count = line_count;

        (intrinsic_w, line_count)
    }

    fn is_shaped(&self, widget: &Text, target_w: f32) -> bool {
        self.shaped_text.as_deref() == Some(&*widget.text)
            && self.shaped_width == target_w
            && self.shaped_wrap == widget.wrap
    }

    fn ensure_shaped(&mut self, widget: &Text, target_w: f32, rs: &TextStyle) {
        if !self.is_shaped(widget, target_w) {
            let style = widget.run_style(rs, widget.wrap);
            self.buffer.set_style(&style);
            self.buffer.set_text(&widget.text);
            self.buffer.set_width(Some(target_w));
            self.buffer.shape();

            self.shaped_text = Some(widget.text.to_string());
            self.shaped_width = target_w;
            self.shaped_wrap = widget.wrap;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextRole {
    H1,
    H2,
    H3,
    #[default]
    Body,
    Label,
    Caption,
}

impl TextRole {
    fn style(self, typo: &Typography) -> TextStyle {
        match self {
            TextRole::H1 => typo.h1,
            TextRole::H2 => typo.h2,
            TextRole::H3 => typo.h3,
            TextRole::Body => typo.body,
            TextRole::Label => typo.label,
            TextRole::Caption => typo.caption,
        }
    }
}

pub struct Text {
    id: Id,
    text: Cow<'static, str>,
    role: TextRole,
    font_size: Option<f32>,
    line_height: Option<f32>,
    family: Option<Family>,
    style: Option<Style>,
    weight: Option<Weight>,
    color_opt: Option<Color>,
    wrap: Wrap,

    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,
}

impl Text {
    pub fn new<S: Into<Cow<'static, str>>>(content: S) -> Self {
        Self {
            id: 0,
            text: content.into(),
            role: TextRole::default(),
            font_size: None,
            line_height: None,
            family: None,
            style: None,
            weight: None,
            color_opt: None,
            wrap: Wrap::Word,
            size: Size::new(Length::Grow, Length::Fit),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
        }
    }

    pub fn h1<S: Into<Cow<'static, str>>>(c: S) -> Self {
        Self::new(c).role(TextRole::H1)
    }
    pub fn h2<S: Into<Cow<'static, str>>>(c: S) -> Self {
        Self::new(c).role(TextRole::H2)
    }
    pub fn h3<S: Into<Cow<'static, str>>>(c: S) -> Self {
        Self::new(c).role(TextRole::H3)
    }
    pub fn body<S: Into<Cow<'static, str>>>(c: S) -> Self {
        Self::new(c).role(TextRole::Body)
    }
    pub fn label<S: Into<Cow<'static, str>>>(c: S) -> Self {
        Self::new(c).role(TextRole::Label)
    }
    pub fn caption<S: Into<Cow<'static, str>>>(c: S) -> Self {
        Self::new(c).role(TextRole::Caption)
    }

    pub fn role(mut self, role: TextRole) -> Self {
        self.role = role;
        self
    }
    pub fn family(mut self, family: Family) -> Self {
        self.family = Some(family);
        self
    }
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
    pub fn weight(mut self, weight: Weight) -> Self {
        self.weight = Some(weight);
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color_opt = Some(color);
        self
    }
    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = Some(font_size);
        self
    }
    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = Some(line_height);
        self
    }
    pub fn wrap(mut self, wrap: Wrap) -> Self {
        self.wrap = wrap;
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

    pub(super) fn set_content(&mut self, t: impl Into<Cow<'static, str>>, c: Color) {
        self.text = t.into();
        self.color_opt = Some(c);
    }

    fn resolved_style(&self, theme: &Theme) -> TextStyle {
        let base = self.role.style(&theme.typography);
        TextStyle {
            font_size: self.font_size.unwrap_or(base.font_size),
            line_height: self.line_height.unwrap_or(base.line_height),
            weight: self.weight.unwrap_or(base.weight),
            style: self.style.unwrap_or(base.style),
        }
    }
    fn run_style(&self, rs: &TextStyle, wrap: Wrap) -> RunStyle {
        RunStyle::from_theme(rs, self.family.clone(), wrap, self.color_opt)
    }

    fn ensure_state<'b>(
        &self,
        view_state: &'b mut ViewState,
        text: &mut dyn TextBackend,
        metrics: TextMetrics,
    ) -> &'b mut TextViewState {
        let state = view_state.ensure(self.id, || TextViewState::new(text.create_buffer(metrics)));

        if state.buffer.metrics() != metrics {
            state.shaped_text = None;
            state.layout_text = None;
        }

        state
    }
}

impl IntoElement for Text {}

impl<M> Widget<M> for Text {
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }

    fn child_count(&self) -> usize {
        0
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        unreachable!()
    }

    fn layout<'b>(&mut self, ctx: &mut LayoutCtx<'b, M>) -> Node {
        let rs = self.resolved_style(ctx.theme);
        let metrics = TextMetrics::new(rs.font_size, rs.line_height);
        let state = self.ensure_state(&mut ctx.ui.view_state, ctx.text, metrics);
        let (intrinsic_w, line_count) = state.ensure_layout(self, &rs);

        let line_px = (rs.font_size * rs.line_height).ceil() as i32;
        let intrinsic_h = (line_count as i32).saturating_mul(line_px);

        Node {
            size: self.size,
            min: self
                .min
                .max(Size::new(intrinsic_w, intrinsic_h))
                .min(self.max),
            max: self.max,
            ..Default::default()
        }
    }

    fn min_height_for_width<'b>(&mut self, ctx: &mut LayoutCtx<'b, M>, width: i32) -> Option<i32> {
        let rs = self.resolved_style(ctx.theme);
        let target_w = width.max(1) as f32;
        let metrics = TextMetrics::new(rs.font_size, rs.line_height);
        let state = self.ensure_state(&mut ctx.ui.view_state, ctx.text, metrics);
        state.ensure_shaped(self, target_w, &rs);

        let lines = state.buffer.line_count() as i32;
        let line_px = (rs.font_size * rs.line_height).ceil() as i32;
        let h = lines.saturating_mul(line_px);

        Some(h.clamp(self.min.height, self.max.height))
    }

    fn prepare(&mut self, ctx: &mut PrepareCtx) {
        if let Some(state) = ctx.view_state.get_mut::<TextViewState>(&self.id) {
            state.buffer.prepare(ctx.gpu, ctx.texture);
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        let r = ctx.rect();
        let base_color = ctx.theme.on_surface;

        let Some(state) = ctx.view_state.get::<TextViewState>(&self.id) else {
            return;
        };

        for glyph in state.buffer.glyphs() {
            let tint = glyph.color.unwrap_or(base_color);
            instances.push(Instance::ui_tex(
                Position::new(glyph.pos.x + r.x as f32, glyph.pos.y + r.y as f32),
                glyph.size,
                tint,
                glyph.handle,
            ));
        }
    }
}
