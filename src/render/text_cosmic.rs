use std::{cell::RefCell, rc::Rc};

use cosmic_text::{
    Attrs, Buffer, Color as CColor, Cursor, Family as CFamily, FontSystem, Metrics,
    Motion as CMotion, Shaping, Style as CStyle, SwashCache, SwashContent, SwashImage,
    Weight as CWeight, Wrap as CWrap,
};

use crate::{
    graphics::Gpu,
    model::{Color, Position, Size},
    render::{AllocatorKind, glyph_atlas::GlyphAtlas, texture::TextureRegistry},
    text::{
        CursorRect, Family, FontStyle, LineMetrics, Measured, Motion, PaintGlyph, RunStyle,
        SelectionRect, TextBackend, TextBuffer, TextCursor, TextMetrics, Weight, Wrap,
    },
};

/// sRGB-encode a coverage value so the atlas's `*Srgb` format decodes it back
/// to the original on sample.
///
/// Only needed where coverage lives in the *colour* channels: the alpha channel
/// of an sRGB format is linear and passes through untouched, so plain masks
/// need no round trip.
fn linear_coverage_to_srgb_u8(c: u8) -> u8 {
    let f = c as f32 / 255.0;
    let encoded = if f <= 0.0031308 {
        f * 12.92
    } else {
        1.055 * f.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Convert a rasterised glyph to **straight** (unmultiplied) RGBA.
///
/// The shader premultiplies once at the end of `fs_main`, so uploading
/// premultiplied data here would apply alpha twice — a glyph at 50% coverage
/// would render at 25%, i.e. visibly thin text.
fn straight_rgba(img: &SwashImage) -> Vec<u8> {
    match img.content {
        SwashContent::Mask => {
            // Straight white at `coverage` alpha. RGB is a constant 1.0, which
            // survives the atlas's sRGB decode unchanged, and alpha is linear
            // in an sRGB format — so no encoding round trip is needed.
            let a = &img.data;
            let mut out = Vec::with_capacity(a.len() * 4);
            for &aa in a {
                out.extend_from_slice(&[255, 255, 255, aa]);
            }
            out
        }
        SwashContent::SubpixelMask => {
            // Per-channel coverage cannot be expressed with a single straight
            // alpha, so normalise: RGB carries coverage relative to the
            // strongest channel and A carries that maximum. The shader's
            // premultiply then reconstitutes per-channel coverage exactly.
            //
            // RGB holds coverage in colour channels, so it does need the sRGB
            // round trip to survive the atlas format.
            let m = &img.data;
            let mut out = Vec::with_capacity(m.len() / 3 * 4);
            for px in m.chunks_exact(3) {
                let a = px[0].max(px[1]).max(px[2]);
                if a == 0 {
                    out.extend_from_slice(&[0, 0, 0, 0]);
                    continue;
                }
                let norm = |c: u8| {
                    linear_coverage_to_srgb_u8(
                        ((c as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8,
                    )
                };
                out.extend_from_slice(&[norm(px[0]), norm(px[1]), norm(px[2]), a]);
            }
            out
        }
        SwashContent::Color => {
            // Already straight sRGB from the font; the atlas format decodes it.
            img.data.clone()
        }
    }
}

fn motion_to_cosmic(m: Motion) -> CMotion {
    match m {
        Motion::Left => CMotion::Left,
        Motion::Right => CMotion::Right,
        Motion::Up => CMotion::Up,
        Motion::Down => CMotion::Down,
        Motion::Home => CMotion::Home,
        Motion::End => CMotion::End,
    }
}

fn cosmic_metrics(m: TextMetrics) -> Metrics {
    Metrics::relative(m.font_size, m.line_height)
}

fn cosmic_color(c: Color) -> CColor {
    CColor::rgba(c.r(), c.g(), c.b(), c.a())
}

fn cosmic_family(f: &Family) -> CFamily<'_> {
    match f {
        Family::Monospace => CFamily::Monospace,
        Family::SansSerif => CFamily::SansSerif,
        Family::Serif => CFamily::Serif,
        Family::Name(name) => CFamily::Name(name.as_ref()),
    }
}

fn cosmic_wrap(w: Wrap) -> CWrap {
    match w {
        Wrap::None => CWrap::None,
        Wrap::Glyph => CWrap::Glyph,
        Wrap::Word => CWrap::Word,
        Wrap::WordOrGlyph => CWrap::WordOrGlyph,
    }
}

fn cosmic_weight(w: Weight) -> CWeight {
    CWeight(w.0)
}

fn cosmic_style(s: FontStyle) -> CStyle {
    match s {
        FontStyle::Normal => CStyle::Normal,
        FontStyle::Italic => CStyle::Italic,
        FontStyle::Oblique => CStyle::Oblique,
    }
}

/// Font system + rasterizer + GPU glyph atlas, shared by the backend and every
/// buffer it hands out.
struct Shared {
    font_system: FontSystem,
    swash_cache: SwashCache,
    glyph_atlas: GlyphAtlas,
    scale_factor: f32,
}

/// `cosmic_text`-backed [`TextBackend`]. Choose the atlas packer at construction
/// (`CosmicText::new(AllocatorKind::Skyline)`) or use `Default` for the shelf
/// packer.
pub struct TextCosmic {
    shared: Rc<RefCell<Shared>>,
}

impl Default for TextCosmic {
    fn default() -> Self {
        Self::new(AllocatorKind::default())
    }
}

impl TextCosmic {
    pub fn new(allocator: AllocatorKind) -> Self {
        Self {
            shared: Rc::new(RefCell::new(Shared {
                font_system: FontSystem::new(),
                swash_cache: SwashCache::new(),
                glyph_atlas: GlyphAtlas::new(allocator),
                scale_factor: 1.0,
            })),
        }
    }
}

impl TextBackend for TextCosmic {
    fn create_buffer(&mut self, metrics: TextMetrics) -> Box<dyn TextBuffer> {
        let buffer = {
            let mut sh = self.shared.borrow_mut();
            Buffer::new(&mut sh.font_system, cosmic_metrics(metrics))
        };
        Box::new(CosmicBuffer {
            shared: self.shared.clone(),
            buffer,
            text: String::new(),
            metrics,
            family: None,
            weight: Default::default(),
            style: CStyle::Normal,
            wrap: CWrap::None,
            color: None,
            width: None,
            dirty: true,
            paint_glyphs: Vec::new(),
        })
    }

    fn set_scale_factor(&mut self, scale: f32) {
        self.shared.borrow_mut().scale_factor = scale;
    }

    fn tick(&mut self) {
        self.shared.borrow_mut().glyph_atlas.tick();
    }
}

/// A `cosmic_text::Buffer` presented through the engine's [`TextBuffer`] seam.
pub struct CosmicBuffer {
    shared: Rc<RefCell<Shared>>,
    buffer: Buffer,
    text: String,
    metrics: TextMetrics,
    family: Option<Family>,
    weight: CWeight,
    style: CStyle,
    wrap: CWrap,
    color: Option<Color>,
    width: Option<f32>,
    dirty: bool,
    paint_glyphs: Vec<PaintGlyph>,
}

impl TextBuffer for CosmicBuffer {
    fn set_style(&mut self, style: &RunStyle) {
        self.metrics = style.metrics;
        self.family = style.family.clone();
        self.weight = cosmic_weight(style.weight);
        self.style = cosmic_style(style.style);
        self.wrap = cosmic_wrap(style.wrap);
        self.color = style.color;
        self.dirty = true;
    }

    fn set_text(&mut self, text: &str) {
        if self.text != text {
            self.text.clear();
            self.text.push_str(text);
            self.dirty = true;
        }
    }

    fn set_width(&mut self, max_w: Option<f32>) {
        if self.width != max_w {
            self.width = max_w;
            self.dirty = true;
        }
    }

    fn shape(&mut self) {
        if !self.dirty {
            return;
        }
        let mut g = self.shared.borrow_mut();
        let fs = &mut g.font_system;

        self.buffer.set_metrics(cosmic_metrics(self.metrics));
        self.buffer.set_wrap(self.wrap);

        let mut attrs = Attrs::new().weight(self.weight).style(self.style);
        if let Some(family) = &self.family {
            attrs = attrs.family(cosmic_family(family));
        }
        if let Some(c) = self.color {
            attrs = attrs.color(cosmic_color(c));
        }

        self.buffer
            .set_text(&self.text, &attrs, Shaping::Basic, None);
        self.buffer.set_size(self.width, None);
        self.buffer.shape_until_scroll(fs, false);
        self.dirty = false;
    }

    fn metrics(&self) -> TextMetrics {
        self.metrics
    }

    fn measured(&self) -> Measured {
        let lh = self.metrics.line_px();
        let mut size = Size::new(0.0f32, 0.0f32);
        let mut lines = Vec::new();
        for run in self.buffer.layout_runs() {
            size.width = size.width.max(run.line_w);
            lines.push(LineMetrics {
                top: run.line_y,
                height: lh,
                width: run.line_w,
            });
        }
        size.height = lines.len() as f32 * lh;
        Measured { size, lines }
    }

    fn line_count(&self) -> usize {
        let mut n = 0usize;
        let mut last = f32::NAN;
        for run in self.buffer.layout_runs() {
            if run.line_y != last {
                n += 1;
                last = run.line_y;
            }
        }
        n.max(1)
    }

    fn prepare(&mut self, gpu: &Gpu, tex: &mut TextureRegistry) {
        self.shape();
        self.paint_glyphs.clear();

        let default_color = self.color;
        let mut g = self.shared.borrow_mut();
        let Shared {
            font_system,
            swash_cache,
            glyph_atlas,
            scale_factor,
        } = &mut *g;
        let sf = *scale_factor;

        for run in self.buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let phys = glyph.physical((0.0, run.line_y * sf), sf);
                let img = match swash_cache.get_image(font_system, phys.cache_key).as_ref() {
                    Some(img) => img,
                    None => continue,
                };
                let (w, h) = (img.placement.width, img.placement.height);
                if w == 0 || h == 0 {
                    continue;
                }

                let pos = Position::new(
                    (phys.x + img.placement.left) as f32 / sf,
                    (phys.y - img.placement.top) as f32 / sf,
                );
                let size = Size::new(w as f32 / sf, h as f32 / sf);
                let color = glyph
                    .color_opt
                    .map(|c| Color::rgba(c.r(), c.g(), c.b(), c.a()))
                    .or(default_color);

                let handle = if let Some(handle) = glyph_atlas.lookup(&phys.cache_key) {
                    glyph_atlas.touch(phys.cache_key);
                    handle
                } else {
                    let rgba = straight_rgba(img);
                    match glyph_atlas.upload(gpu, tex, phys.cache_key, w, h, &rgba) {
                        Some(handle) => handle,
                        None => continue, // oversized glyph; skip
                    }
                };

                self.paint_glyphs.push(PaintGlyph {
                    pos,
                    size,
                    handle,
                    color,
                });
            }
        }
    }

    fn glyphs(&self) -> &[PaintGlyph] {
        &self.paint_glyphs
    }

    fn hit(&self, x: f32, y: f32) -> Option<TextCursor> {
        self.buffer
            .hit(x, y)
            .map(|c| TextCursor::new(c.line, c.index))
    }

    fn cursor_motion(&mut self, cursor: TextCursor, motion: Motion) -> Option<TextCursor> {
        self.shape();
        let mut g = self.shared.borrow_mut();
        let fs = &mut g.font_system;
        self.buffer
            .cursor_motion(
                fs,
                Cursor::new(cursor.line, cursor.index),
                None,
                motion_to_cosmic(motion),
            )
            .map(|(c, _)| TextCursor::new(c.line, c.index))
    }

    fn cursor_rect(&self, cursor: TextCursor) -> Option<CursorRect> {
        let Metrics {
            font_size,
            line_height,
        } = self.buffer.metrics();
        let line_advance = line_height;
        let caret_h = font_size * 1.1;

        let mut last_matching_line_y: Option<f32> = None;
        let mut last_matching_end_x: f32 = 0.0;

        for run in self.buffer.layout_runs() {
            if run.line_i != cursor.line {
                if run.line_i < cursor.line {
                    last_matching_line_y = Some(run.line_y);
                }
                continue;
            }
            last_matching_line_y = Some(run.line_y);

            if let Some(first) = run.glyphs.first()
                && cursor.index <= first.start
            {
                return Some(CursorRect {
                    x: first.x,
                    y: run.line_y - font_size * 0.9,
                    height: caret_h,
                });
            }

            for glyph in run.glyphs.iter() {
                if cursor.index >= glyph.start && cursor.index <= glyph.end {
                    let x = if cursor.index == glyph.start {
                        glyph.x
                    } else if cursor.index == glyph.end {
                        glyph.x + glyph.w
                    } else {
                        let cluster_len = glyph.end - glyph.start;
                        let prefix_len = cursor.index - glyph.start;
                        let frac = prefix_len as f32 / cluster_len.max(1) as f32;
                        glyph.x + glyph.w * frac
                    };
                    return Some(CursorRect {
                        x,
                        y: run.line_y - font_size * 0.9,
                        height: caret_h,
                    });
                }
                last_matching_end_x = glyph.x + glyph.w;
            }
        }

        if let Some(line_y) = last_matching_line_y {
            let has_run_on_cursor_line = self.buffer.layout_runs().any(|r| r.line_i == cursor.line);
            if !has_run_on_cursor_line {
                return Some(CursorRect {
                    x: 0.0,
                    y: line_y + line_advance - font_size * 0.9,
                    height: caret_h,
                });
            }
            return Some(CursorRect {
                x: last_matching_end_x,
                y: line_y - font_size * 0.9,
                height: caret_h,
            });
        }

        let baseline = self
            .buffer
            .layout_runs()
            .next()
            .map(|r| r.line_y)
            .unwrap_or(line_advance);
        Some(CursorRect {
            x: 0.0,
            y: baseline - font_size * 0.9,
            height: caret_h,
        })
    }

    fn selection_rects(&self, start: TextCursor, end: TextCursor) -> Vec<SelectionRect> {
        let s = Cursor::new(start.line, start.index);
        let e = Cursor::new(end.line, end.index);
        let mut rects = Vec::new();
        for run in self.buffer.layout_runs() {
            for (x, w) in run.highlight(s, e) {
                if w > 0.0 {
                    rects.push(SelectionRect {
                        x,
                        top: run.line_top,
                        width: w,
                        height: run.line_height,
                    });
                }
            }
        }
        rects
    }
}
