use crate::{gpu::Gpu, render::texture::TextureRegistry};

pub mod style;
pub use style::*;

pub mod layout;
pub use layout::*;

pub mod atlas;

#[cfg(feature = "text_cosmic")]
pub mod cosmic;

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
