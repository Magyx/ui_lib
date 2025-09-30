use super::*;
use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Style, Weight, Wrap};

pub struct Text<'a> {
    layout: Option<Layout>,
    buffer: Option<Buffer>,

    id: Id,
    text: &'a str,
    font_size: f32,
    line_height: f32,
    atributes: Attrs<'a>,
    position: Position<i32>,
    size: Size<Length<i32>>,
    min: Size<i32>,
    max: Size<i32>,
}

impl<'a> Text<'a> {
    pub fn new(content: &'a str, font_size: f32) -> Self {
        Self {
            layout: None,
            buffer: None,

            id: crate::context::next_id(),
            text: content,
            font_size,
            line_height: 1.2,
            atributes: Attrs::new(),
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

// TODO: fix wrapping issue
impl<'a, M> Widget<M> for Text<'a> {
    fn id(&self) -> Id {
        self.id
    }

    fn layout(&self) -> Layout {
        self.layout.expect(LAYOUT_ERROR)
    }

    fn fit_size(&mut self, ctx: &mut FitCtx<M>) -> Layout {
        let font_system = ctx.text.font_system_mut();

        let metrics = Metrics::relative(self.font_size, self.line_height);
        let mut buffer = Buffer::new(font_system, metrics);

        buffer.set_wrap(font_system, Wrap::WordOrGlyph);
        buffer.set_text(font_system, self.text, &self.atributes, Shaping::Advanced);
        buffer.set_size(font_system, None, None);
        buffer.shape_until_scroll(font_system, false);

        let mut line_w: f32 = 0.0;
        let mut line_h: f32 = 0.0;
        for run in buffer.layout_runs() {
            line_w = line_w.max(run.line_w);
            line_h = line_h.max(run.line_height);
        }
        let base = Size::new(line_w.ceil() as i32, line_h.ceil() as i32);

        let min = self.min;
        let max = self.max;

        let current = Size::new(
            base.width.clamp(min.width, max.width),
            base.height.clamp(min.height, max.height),
        );

        self.layout = Some(Layout {
            size: self.size,
            current_size: current,
            min,
            max,
        });
        self.buffer = Some(buffer);
        self.layout.unwrap()
    }

    fn grow_size(&mut self, ctx: &mut GrowCtx<M>, parent: Size<i32>) {
        let target_w = match self.size.width {
            Length::Grow => parent.width,
            Length::Fixed(w) => w,
            Length::Fit => {
                let base = self.layout.as_ref().unwrap().current_size.width;
                base.min(parent.width)
            }
        };
        let cap_h = match self.size.height {
            Length::Grow => parent.height,
            Length::Fixed(h) => h,
            Length::Fit => i32::MAX,
        };

        let fs = ctx.text.font_system_mut();
        let buffer = self.buffer.as_mut().expect("grow called before fit");

        buffer.set_wrap(fs, Wrap::WordOrGlyph);
        buffer.set_size(fs, Some(target_w as f32), None);
        buffer.shape_until_scroll(fs, false);

        let mut total_h: f32 = 0.0;
        let mut content_w: f32 = 0.0;
        for run in buffer.layout_runs() {
            total_h += run.line_height;
            content_w = content_w.max(run.line_w);
        }

        let mut final_w = target_w.max(content_w.ceil() as i32);
        let mut final_h = total_h.ceil() as i32;

        let l = self.layout.as_ref().unwrap();
        final_w = final_w.clamp(l.min.width, l.max.width);
        final_h = final_h.clamp(l.min.height, l.max.height).min(cap_h);

        self.size.width = Length::Fixed(final_w);
        self.size.height = Length::Fixed(final_h);
        if let Some(m) = self.layout.as_mut() {
            m.current_size = Size::new(final_w, final_h);
        }
    }

    fn place(&mut self, ctx: &mut PlaceCtx<M>, position: Position<i32>) -> Size<i32> {
        self.position = position;
        self.size.into_fixed()
    }

    fn draw(&self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        const BASE_COLOR: cosmic_text::Color = cosmic_text::Color::rgba(255, 255, 255, 255);
        let buffer = self.buffer.as_ref().expect("draw called before fit");
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
