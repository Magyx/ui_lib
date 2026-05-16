use std::borrow::Cow;

use cosmic_text::{Attrs, Buffer, Metrics, Shaping};

use super::*;

struct TextViewState {
    buffer: Buffer,
}

pub struct Text {
    x: i32,
    y: i32,
    w: i32,
    h: i32,

    id: Id,
    text: Cow<'static, str>,
    font_size: f32,
    line_height: f32,
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
    pub fn new<S: Into<Cow<'static, str>>>(content: S, font_size: f32) -> Self {
        Self {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            id: 0,
            text: content.into(),
            font_size,
            line_height: 1.2,
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
    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
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

    fn attrs(&self) -> Attrs<'_> {
        let mut attrs = Attrs::new();

        if let Some(family) = &self.family {
            attrs = attrs.family(family.as_cosmic());
        }

        if let Some(style) = self.style {
            attrs = attrs.style(style);
        }

        if let Some(weight) = self.weight {
            attrs = attrs.weight(weight);
        }

        if let Some(color) = self.color_opt {
            attrs = attrs.color(cosmic_text::Color(color.0));
        }

        attrs
    }
    fn get_buffer<'b>(&self, view_state: &'b ViewState) -> Option<&'b Buffer> {
        view_state.get::<TextViewState>(&self.id).map(|s| &s.buffer)
    }
    fn ensure_buffer<'b>(
        &self,
        view_state: &'b mut ViewState,
        fs: &mut cosmic_text::FontSystem,
    ) -> &'b mut Buffer {
        let desired = Metrics::relative(self.font_size, self.line_height);
        let state = view_state.ensure(self.id, || TextViewState {
            buffer: Buffer::new(fs, desired),
        });

        if state.buffer.metrics().font_size != desired.font_size
            || state.buffer.metrics().line_height != desired.line_height
        {
            state.buffer.set_metrics(fs, desired);
        }

        &mut state.buffer
    }
}

impl IntoElement for Text {}

impl<M> Widget<M> for Text {
    fn layout<'b>(&mut self, ctx: &mut LayoutCtx<'b, M>) -> Node {
        let wrap = self.wrap;

        let (intrinsic_w, line_count) = {
            let fs = ctx.text.font_system_mut();
            let b = self.ensure_buffer(&mut ctx.ui.view_state, fs);

            // (a) Unwrapped measurement
            b.set_wrap(fs, Wrap::None);
            b.set_text(fs, &self.text, &self.attrs(), Shaping::Basic);
            b.set_size(fs, None, None);
            b.shape_until_scroll(fs, false);

            let mut unwrapped_w = 0.0f32;
            let mut lines = 0usize;
            let mut prev_y: Option<f32> = None;
            for run in b.layout_runs() {
                unwrapped_w = unwrapped_w.max(run.line_w);
                if prev_y != Some(run.line_y) {
                    lines += 1;
                    prev_y = Some(run.line_y);
                }
            }
            let unwrapped_w_i = unwrapped_w.ceil() as i32;

            // (b) Minimal useful width with wrapping
            let min_break_w_i = if wrap != Wrap::None {
                let mut longest = 0.0f32;
                for piece in self.text.split_whitespace().filter(|s| !s.is_empty()) {
                    b.set_wrap(fs, Wrap::None);
                    b.set_text(fs, piece, &self.attrs(), Shaping::Basic);
                    b.set_size(fs, None, None);
                    b.shape_until_scroll(fs, false);
                    for run in b.layout_runs() {
                        longest = longest.max(run.line_w);
                    }
                }
                longest.ceil() as i32
            } else {
                unwrapped_w_i
            };

            let intrinsic_w = if self.wrap == Wrap::None {
                unwrapped_w_i
            } else {
                min_break_w_i
            };

            (intrinsic_w, lines.max(1))
        };
        let line_px = (self.font_size * self.line_height).ceil() as i32;
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
        let fs = ctx.text.font_system_mut();

        let b = self.ensure_buffer(&mut ctx.ui.view_state, fs);
        b.set_wrap(fs, self.wrap);
        b.set_text(fs, &self.text, &self.attrs(), Shaping::Basic);
        b.set_size(fs, Some(width.max(1) as f32), None);
        b.shape_until_scroll(fs, false);

        let mut lines = 0usize;
        let mut last = f32::NAN;
        for run in b.layout_runs() {
            if run.line_y != last {
                lines += 1;
                last = run.line_y;
            }
        }
        let lines = lines.max(1) as i32;
        let line_px = (self.font_size * self.line_height).ceil() as i32;
        let h = lines.saturating_mul(line_px);

        Some(h.clamp(self.min.height, self.max.height))
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
        0
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        unreachable!()
    }

    // TODO: cache shape and compare to avoid reshaping every (re)paint
    fn prepare(&mut self, ctx: &mut PrepareCtx) {
        let fs = ctx.text.font_system_mut();
        let buf = self.ensure_buffer(ctx.view_state, fs);
        buf.set_wrap(fs, self.wrap);
        buf.set_text(fs, &self.text, &self.attrs(), Shaping::Basic);
        buf.set_size(fs, Some(self.w.max(1) as f32), None);
        buf.shape_until_scroll(fs, false);

        for key in buf.layout_runs().flat_map(|r| r.glyphs) {
            if let Some((size, key)) = ctx.text.prepare_glyph_data(key) {
                let _ = ctx
                    .text
                    .upload_glyph(ctx.gpu, ctx.texture, key, size.width, size.height);
            }
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        const BASE_COLOR: Color = Color::rgba(255, 255, 255, 255);

        let Some(buf) = self.get_buffer(ctx.view_state) else {
            return;
        };

        for run in buf.layout_runs() {
            for glyph in run.glyphs {
                let Some((Position { x: left, y: top }, Size { width, height }, cache_key)) = ctx
                    .text
                    .get_glyph_data(glyph, (self.x as f32, self.y as f32), run.line_y)
                else {
                    continue;
                };

                let tint = glyph
                    .color_opt
                    .map(|c| Color::rgba(c.r(), c.g(), c.b(), c.a()))
                    .unwrap_or(BASE_COLOR);

                let Some(handle) = ctx.text.lookup_glyph_handle(cache_key) else {
                    continue;
                };

                instances.push(Instance::ui_tex(
                    Position::new(left, top),
                    Size::new(width, height),
                    tint,
                    handle,
                ));
            }
        }
    }
}
