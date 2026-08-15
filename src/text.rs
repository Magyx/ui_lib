use std::borrow::Cow;

use crate::{
    graphics::Gpu,
    model::{Color, Position, Size},
    render::texture::{TextureHandle, TextureRegistry},
    theme::TextStyle,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    Monospace,
    SansSerif,
    Serif,
    Name(Cow<'static, str>),
}

/// Specifies the weight of glyphs in the font, their degree of blackness or stroke thickness.
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug, Hash)]
pub struct Weight(pub u16);

impl Default for Weight {
    #[inline]
    fn default() -> Weight {
        Weight::NORMAL
    }
}

impl Weight {
    /// Thin weight (100), the thinnest value.
    pub const THIN: Weight = Weight(100);
    /// Extra light weight (200).
    pub const EXTRA_LIGHT: Weight = Weight(200);
    /// Light weight (300).
    pub const LIGHT: Weight = Weight(300);
    /// Normal (400).
    pub const NORMAL: Weight = Weight(400);
    /// Medium weight (500, higher than normal).
    pub const MEDIUM: Weight = Weight(500);
    /// Semibold weight (600).
    pub const SEMIBOLD: Weight = Weight(600);
    /// Bold weight (700).
    pub const BOLD: Weight = Weight(700);
    /// Extra-bold weight (800).
    pub const EXTRA_BOLD: Weight = Weight(800);
    /// Black weight (900), the thickest value.
    pub const BLACK: Weight = Weight(900);
}

/// Allows italic or oblique faces to be selected.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub enum FontStyle {
    /// A face that is neither italic not obliqued.
    #[default]
    Normal,
    /// A form that is generally cursive in nature.
    Italic,
    /// A typically-sloped version of the regular face.
    Oblique,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Wrap {
    /// No wrapping
    None,
    /// Wraps at a glyph level
    Glyph,
    /// Wraps at the word level
    Word,
    /// Wraps at the word level, or fallback to glyph level if a word can't fit on a line by itself
    WordOrGlyph,
}

impl std::fmt::Display for Wrap {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::None => write!(f, "No Wrap"),
            Self::Word => write!(f, "Word Wrap"),
            Self::WordOrGlyph => write!(f, "Word Wrap or Character"),
            Self::Glyph => write!(f, "Character"),
        }
    }
}

/// Font size and relative line height (a multiplier of font size), matching the
/// theme's convention. Converted to the shaper's units inside the backend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMetrics {
    pub font_size: f32,
    /// Line-height scale (relative). Absolute line px is `font_size * line_height`.
    pub line_height: f32,
}

impl TextMetrics {
    pub fn new(font_size: f32, line_height: f32) -> Self {
        Self {
            font_size,
            line_height,
        }
    }

    /// Absolute line advance in logical pixels.
    pub fn line_px(&self) -> f32 {
        self.font_size * self.line_height
    }
}

impl From<&TextStyle> for TextMetrics {
    fn from(ts: &TextStyle) -> Self {
        Self::new(ts.font_size, ts.line_height)
    }
}

/// Everything a [`TextBuffer`] needs to shape a run. Self-contained so the
/// boundary doesn't depend on the shape of `crate::theme::TextStyle`.
#[derive(Clone, Debug)]
pub struct RunStyle {
    pub metrics: TextMetrics,
    pub family: Option<Family>,
    pub weight: Weight,
    pub style: FontStyle,
    pub wrap: Wrap,
    /// Default glyph color; per-glyph runs may still override it.
    pub color: Option<Color>,
}

impl RunStyle {
    /// Build from a theme [`TextStyle`] plus the per-widget bits the theme
    /// doesn't carry (family, wrap, an optional default color).
    pub fn from_theme(
        ts: &TextStyle,
        family: Option<Family>,
        wrap: Wrap,
        color: Option<Color>,
    ) -> Self {
        Self {
            metrics: TextMetrics::from(ts),
            family,
            weight: ts.weight,
            style: ts.style,
            wrap,
            color,
        }
    }
}

/// A logical caret position: a line and a byte index within that line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextCursor {
    pub line: usize,
    pub index: usize,
}

impl TextCursor {
    pub fn new(line: usize, index: usize) -> Self {
        Self { line, index }
    }
}

/// A cursor movement, resolved against shaped text by the backend so that
/// `Up`/`Down`/`Home`/`End` respect wrapping and visual order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
}

/// Caret geometry for a [`TextCursor`], in buffer-local logical coordinates
/// (origin at the buffer's top-left; the widget adds its own position).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CursorRect {
    pub x: f32,
    pub y: f32,
    pub height: f32,
}

/// One line's selection-highlight rectangle, in buffer-local logical
/// coordinates (the widget adds its own position and any padding).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SelectionRect {
    pub x: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

/// One line of a measured run.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LineMetrics {
    /// Top of the line in run-local coordinates.
    pub top: f32,
    /// Line height in logical pixels.
    pub height: f32,
    /// Advance width of the line's content.
    pub width: f32,
}

/// Result of laying out a run: its bounding size and per-line metrics.
#[derive(Clone, Debug, Default)]
pub struct Measured {
    pub size: Size<f32>,
    pub lines: Vec<LineMetrics>,
}

/// A glyph ready to draw, in buffer-local logical pixels (the widget offsets by
/// its own position). Produced by [`TextBuffer::glyphs`] after
/// [`TextBuffer::prepare`] has uploaded it to the atlas.
#[derive(Clone, Copy, Debug)]
pub struct PaintGlyph {
    pub pos: Position<f32>,
    pub size: Size<f32>,
    pub handle: TextureHandle,
    pub color: Option<Color>,
}

/// A persistent, shapeable run of text owned by a [`TextBackend`].
///
/// Lifecycle per frame: set inputs ([`set_style`](Self::set_style),
/// [`set_text`](Self::set_text), [`set_width`](Self::set_width)) → in the
/// prepare phase call [`prepare`](Self::prepare) (shapes if needed and uploads
/// glyphs to the GPU) → in the paint phase read [`glyphs`](Self::glyphs). The
/// `&self` query methods assume the buffer has been shaped.
pub trait TextBuffer {
    /// Replace the style/metrics/wrap used for subsequent shaping.
    fn set_style(&mut self, style: &RunStyle);

    /// Replace the text content.
    fn set_text(&mut self, text: &str);

    /// Wrapping width in logical pixels, or `None` for unbounded.
    fn set_width(&mut self, max_w: Option<f32>);

    /// Lay out the current text/style/width. Idempotent if nothing changed.
    fn shape(&mut self);

    /// Current metrics (font size / relative line height).
    fn metrics(&self) -> TextMetrics;

    /// Bounding size and per-line metrics of the shaped run.
    fn measured(&self) -> Measured;

    /// Number of visual lines after shaping (>= 1).
    fn line_count(&self) -> usize;

    /// Shape if needed, then rasterize and upload every glyph into the
    /// backend's GPU atlas, caching the resulting [`PaintGlyph`]s for
    /// [`glyphs`](Self::glyphs). Must run in the prepare phase, where the `Gpu`
    /// and [`TextureRegistry`] are available.
    fn prepare(&mut self, gpu: &Gpu, tex: &mut TextureRegistry);

    /// Draw list produced by the most recent [`prepare`](Self::prepare), in
    /// buffer-local coordinates.
    fn glyphs(&self) -> &[PaintGlyph];

    /// Map a pixel point (buffer-local) to the nearest caret position.
    fn hit(&self, x: f32, y: f32) -> Option<TextCursor>;

    /// Move `cursor` by `motion` through the shaped text. `None` when the
    /// motion has no effect (e.g. empty buffer).
    fn cursor_motion(&mut self, cursor: TextCursor, motion: Motion) -> Option<TextCursor>;

    /// Caret geometry for `cursor`, in buffer-local coordinates.
    fn cursor_rect(&self, cursor: TextCursor) -> Option<CursorRect>;

    // Highlight rectangles for the selection `[start, end)`, one per visual
    /// line, in buffer-local coordinates. Empty when `start == end`.
    fn selection_rects(&self, start: TextCursor, end: TextCursor) -> Vec<SelectionRect>;
}

/// A swappable text system. See the [module docs](crate::render::text).
pub trait TextBackend {
    /// Create an empty buffer with the given starting metrics.
    fn create_buffer(&mut self, metrics: TextMetrics) -> Box<dyn TextBuffer>;

    /// Device-pixel ratio for rasterization/hinting. Called per frame.
    fn set_scale_factor(&mut self, scale: f32);

    /// Advance per-frame atlas state (LRU counters). Called once per render
    /// pass, before the prepare phase.
    fn tick(&mut self);
}
