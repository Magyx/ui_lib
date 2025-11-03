use std::{any::Any, borrow::Cow, collections::HashMap};

use super::*;
use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Style, Weight, Wrap};

struct TextViewState {
    buffer: Buffer,
}

pub struct Text<'a> {
    x: i32,
    y: i32,
    w: i32,
    h: i32,

    id: Id,
    text: Cow<'static, str>,
    font_size: f32,
    line_height: f32,
    attrs: Attrs<'a>,
    wrap: Wrap,

    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,
}

impl<'a> Text<'a> {
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
            attrs: Attrs::new(),
            wrap: Wrap::Word,
            size: Size::new(Length::Grow, Length::Fit),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
        }
    }
    pub fn family(mut self, family: Family<'a>) -> Self {
        self.attrs.family = family;
        self
    }
    pub fn style(mut self, style: Style) -> Self {
        self.attrs.style = style;
        self
    }
    pub fn weight(mut self, weight: Weight) -> Self {
        self.attrs.weight = weight;
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.attrs.color_opt = Some(cosmic_text::Color::rgba(
            color.r(),
            color.g(),
            color.b(),
            color.a(),
        ));
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

    fn ensure_buffer<'b>(
        &self,
        view_state: &'b mut HashMap<Id, Box<dyn Any>>,
        fs: &mut cosmic_text::FontSystem,
    ) -> &'b mut Buffer {
        dbg!(&self.id);
        let b = view_state
            .entry(self.id)
            .or_insert_with(|| {
                Box::new(TextViewState {
                    buffer: Buffer::new(fs, Metrics::relative(self.font_size, self.line_height)),
                })
            })
            .downcast_mut::<TextViewState>()
            .expect("View state was wrong type");
        &mut b.buffer
    }
}

impl<'a> IntoElement for Text<'a> {}

impl<'a, M> Widget<M> for Text<'a> {
    fn layout<'b>(&mut self, ctx: &mut LayoutCtx<'b, M>) -> Node {
        let wrap = self.wrap;
        let text = self.text.clone();
        let attrs = self.attrs.clone();

        let (intrinsic_w, line_count) = {
            let fs = ctx.text.font_system_mut();
            let b = self.ensure_buffer(&mut ctx.ui.view_state, fs);

            // (a) Unwrapped measurement
            b.set_wrap(fs, Wrap::None);
            b.set_text(fs, &text, &attrs, Shaping::Basic);
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
                for piece in text.split_whitespace().filter(|s| !s.is_empty()) {
                    b.set_wrap(fs, Wrap::None);
                    b.set_text(fs, piece, &attrs, Shaping::Basic);
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
            width: self.size.width,
            height: self.size.height,
            min_width: self.min.width.max(intrinsic_w).min(self.max.width),
            min_height: self.min.height.max(intrinsic_h).min(self.max.height),
            max_width: self.max.width,
            max_height: self.max.height,
            ..Default::default()
        }
    }

    fn min_height_for_width<'b>(&mut self, ctx: &mut LayoutCtx<'b, M>, width: i32) -> Option<i32> {
        let fs = ctx.text.font_system_mut();
        let wrap = self.wrap;
        let text = self.text.clone();
        let attrs = self.attrs.clone();

        let b = self.ensure_buffer(&mut ctx.ui.view_state, fs);
        b.set_wrap(fs, wrap);
        b.set_text(fs, &text, &attrs, Shaping::Basic);
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

    fn paint(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        const BASE_COLOR: cosmic_text::Color = cosmic_text::Color::rgba(255, 255, 255, 255);

        let wrap = self.wrap;
        let text = self.text.clone();
        let attrs = self.attrs.clone();
        let target_w = self.w.max(1) as f32;

        let fs = ctx.text.font_system_mut();
        let buf = self.ensure_buffer(ctx.view_state, fs);
        buf.set_wrap(fs, wrap);
        buf.set_text(fs, &text, &attrs, Shaping::Basic);
        buf.set_size(fs, Some(target_w), None);
        buf.shape_until_scroll(fs, false);

        for run in buf.layout_runs() {
            for glyph in run.glyphs {
                let (Position { x: left, y: top }, Size { width, height }, cache_key) =
                    match ctx.text.get_glyph_data(glyph) {
                        Some(v) => v,
                        None => continue,
                    };

                let top_left = Position::new(
                    (self.x as f32 + glyph.x).round() as i32 + left,
                    (self.y as f32 + glyph.y + run.line_y).round() as i32 - top,
                );

                let glyph_color = glyph.color_opt.unwrap_or(BASE_COLOR);
                let tint = Color::rgba(
                    glyph_color.r(),
                    glyph_color.g(),
                    glyph_color.b(),
                    glyph_color.a(),
                );

                let handle =
                    match ctx
                        .text
                        .upload_glyph(ctx.gpu, ctx.texture, cache_key, width, height)
                    {
                        Some(h) => h,
                        None => continue,
                    };

                instances.push(Instance::ui_tex(
                    top_left,
                    Size::new(width as i32, height as i32),
                    tint,
                    handle,
                ));
            }
        }
    }
}
