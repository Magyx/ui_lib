use std::borrow::Cow;

use crate::{model::Color, theme::TextStyle};

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
