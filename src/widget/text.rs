use std::borrow::Cow;

use super::*;
use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Style, Weight, Wrap};

pub struct Text<'a> {
    layout: Option<Layout>,
    buffer: Option<Buffer>,
    preferred_size: Option<Size<i32>>,
    wrapped_size: Option<Size<i32>>,

    id: Id,
    text: Cow<'static, str>,
    font_size: f32,
    line_height: f32,
    atributes: Attrs<'a>,
    wrap: Wrap,
    position: Position<i32>,
    size: Size<Length<i32>>,
    min: Size<i32>,
    max: Size<i32>,
}

impl<'a> Text<'a> {
    pub fn new<S: Into<Cow<'static, str>>>(content: S, font_size: f32) -> Self {
        Self {
            layout: None,
            buffer: None,
            preferred_size: None,
            wrapped_size: None,

            id: crate::context::next_id(),
            text: content.into(),
            font_size,
            line_height: 1.2,
            atributes: Attrs::new(),
            wrap: Wrap::Word,
            position: Position::splat(0),
            size: Size::splat(Length::Fit),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
        }
    }

    pub fn family(mut self, family: Family<'a>) -> Self {
        self.atributes.family = family;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.atributes.style = style;
        self
    }

    pub fn weight(mut self, weight: Weight) -> Self {
        self.atributes.weight = weight;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.atributes.color_opt = Some(cosmic_text::Color::rgba(
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

    pub fn size(mut self, size: Size<Length<i32>>) -> Self {
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
}

impl<'a, M> Widget<M> for Text<'a> {
    fn id(&self) -> Id {
        self.id
    }
    fn position(&self) -> &Position<i32> {
        &self.position
    }
    fn layout(&self) -> &Layout {
        self.layout.as_ref().expect(LAYOUT_ERROR)
    }

    fn fit_width(&mut self, ctx: &mut LayoutCtx<M>) -> Layout {
        let fs = ctx.text.font_system_mut();

        if self.buffer.is_none() {
            let metrics = Metrics::relative(self.font_size, self.line_height);
            self.buffer = Some(Buffer::new(fs, metrics));
        }
        let buffer = self.buffer.as_mut().unwrap();

        buffer.set_wrap(fs, self.wrap);
        buffer.set_text(fs, &self.text, &self.atributes, Shaping::Basic);

        // Preferred
        buffer.set_size(fs, None, None);
        buffer.shape_until_scroll(fs, false);

        let mut pref_w = 0f32;
        let mut line_h = 0f32;
        for run in buffer.layout_runs() {
            pref_w = pref_w.max(run.line_w);
            line_h += run.line_height;
        }
        let pref_w = pref_w.ceil() as i32;
        let line_h = line_h.ceil() as i32;
        self.preferred_size = Some(Size::new(pref_w, line_h));

        let min_w = self.min.width.max(1).min(self.max.width);
        let current_w = pref_w.clamp(self.min.width, self.max.width);

        let l = Layout {
            size: self.size,
            current_size: Size::new(current_w, line_h),
            min: Size::new(min_w, self.min.height.min(self.max.height)),
            max: self.max,
        };
        self.layout = Some(l);
        l
    }

    fn grow_width(&mut self, ctx: &mut LayoutCtx<M>, parent_width: i32) {
        let l = self.layout.as_mut().expect(LAYOUT_ERROR);
        let fs = ctx.text.font_system_mut();
        let buffer = self.buffer.as_mut().expect("fit_width must run first");
        let pref = self
            .preferred_size
            .as_ref()
            .expect("preferred_size missing");

        let parent_cap = parent_width.min(self.max.width);
        let lower_bound = l.min.width.min(parent_cap);

        let target_w = match self.size.width {
            Length::Fixed(w) => w.min(parent_cap).max(lower_bound),
            Length::Fit => pref.width.min(parent_cap).max(lower_bound),
            Length::Grow => parent_cap.max(lower_bound),
        };

        buffer.set_size(fs, Some(target_w as f32), None);
        buffer.shape_until_scroll(fs, false);

        let mut shaped_w = 0f32;
        let mut total_h = 0f32;
        for run in buffer.layout_runs() {
            shaped_w = shaped_w.max(run.line_w);
            total_h += run.line_height;
        }
        let shaped_w = shaped_w.ceil() as i32;
        let natural_h = total_h.ceil() as i32;

        let final_w = target_w
            .max(shaped_w)
            .max(l.min.width)
            .min(self.max.width)
            .min(parent_width);
        self.wrapped_size = Some(Size::new(final_w, natural_h));

        l.current_size.width = final_w;
    }

    fn fit_height(&mut self, _ctx: &mut LayoutCtx<M>) -> Layout {
        let l = self.layout.as_ref().expect(LAYOUT_ERROR);
        let natural = self.wrapped_size.as_ref().unwrap();

        let min_h = self.min.height.min(self.max.height);
        let current_h = natural.height.clamp(min_h, self.max.height);

        let l = Layout {
            size: l.size,
            current_size: Size::new(l.current_size.width, current_h),
            min: Size::new(l.min.width, min_h),
            max: l.max,
        };
        self.layout = Some(l);
        l
    }

    fn grow_height(&mut self, _ctx: &mut LayoutCtx<M>, parent_height: i32) {
        let l = self.layout.as_mut().expect(LAYOUT_ERROR);
        let natural_h = self
            .wrapped_size
            .map(|s| s.height)
            .unwrap_or(l.current_size.height);

        // Resolve by Length
        let mut target_h = match self.size.height {
            Length::Fixed(h) => h,
            Length::Fit => natural_h,
            Length::Grow => parent_height,
        };

        target_h = target_h
            .max(self.min.height)
            .max(natural_h)
            .min(self.max.height)
            .min(parent_height);

        l.current_size.height = target_h;
    }

    fn place(&mut self, ctx: &mut LayoutCtx<M>, position: Position<i32>) -> Size<i32> {
        self.position = position;
        <Text as Widget<M>>::layout(self).current_size
    }

    fn draw_self(&self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        const BASE_COLOR: cosmic_text::Color = cosmic_text::Color::rgba(255, 255, 255, 255);
        let buffer = self.buffer.as_ref().expect("draw called before fit");
        let size = <Text as Widget<M>>::layout(self).current_size;
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let (Position { x: left, y: top }, Size { width, height }, cache_key) =
                    match ctx.text.get_glyph_data(glyph) {
                        Some(v) => v,
                        None => continue,
                    };

                let top_left = Position::new(
                    (self.position.x as f32 + glyph.x).round() as i32 + left,
                    (self.position.y as f32 + glyph.y + run.line_y).round() as i32 - top,
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
                        .upload_glyph(ctx.config, ctx.texture, cache_key, width, height)
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
