use crate::{
    model::{Color, Position, Size},
    render::texture::TextureHandle,
};

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
