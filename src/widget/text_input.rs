use std::any::TypeId;
use std::borrow::Cow;
use std::marker::PhantomData;

use cosmic_text::{Attrs, Buffer, Cursor, Metrics, Motion, Shaping};

use super::*;
use crate::event::{KeyState, LogicalKey, MouseButton, UiEventRef};

pub struct TextInputViewState {
    hovered: bool,
    focused: bool,

    buffer: Buffer,
    value: String,
    width: i32,
    cursor: Cursor,
    cursor_rect: Option<(f32, f32, f32)>,
}

impl TextInputViewState {
    fn new(fs: &mut cosmic_text::FontSystem, font_size: f32, line_height: f32) -> Self {
        Self {
            hovered: false,
            focused: false,

            buffer: Buffer::new(fs, Metrics::relative(font_size, line_height)),
            value: String::new(),
            width: 0,
            cursor: Cursor::new(0, 0),
            cursor_rect: None,
        }
    }

    fn cursor_to_byte_offset(&self) -> usize {
        let mut offset = 0usize;
        let mut line = 0usize;
        // Walk through `value` counting newlines to find where cursor.line starts.
        for (i, ch) in self.value.char_indices() {
            if line == self.cursor.line {
                // We've reached the target line. Add the within-line index.
                return (i + self.cursor.index).min(self.value.len());
            }
            if ch == '\n' {
                line += 1;
                offset = i + 1;
            }
        }
        // If the cursor line matches the last line (no trailing \n found mid-loop)
        if line == self.cursor.line {
            return (offset + self.cursor.index).min(self.value.len());
        }
        // Cursor line is past all lines — clamp to end.
        self.value.len()
    }

    /// Convert a flat byte offset into `value` to a `Cursor` (line, index-within-line).
    fn byte_offset_to_cursor(value: &str, offset: usize) -> Cursor {
        let offset = offset.min(value.len());
        let mut line = 0usize;
        let mut line_start = 0usize;
        for (i, ch) in value.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                line_start = i + 1;
            }
        }
        Cursor::new(line, offset - line_start)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TextColors {
    pub text: Color,
    pub placeholder: Color,
    pub caret: Color,
    pub bg: Color,
    pub border: Color,
    pub focus_border: Color,
}

impl Default for TextColors {
    fn default() -> Self {
        Self {
            text: Color::rgb(240, 240, 245),
            placeholder: Color::rgb(140, 140, 150),
            caret: Color::rgb(240, 240, 245),
            bg: Color::rgb(30, 30, 36),
            border: Color::rgb(60, 60, 70),
            focus_border: Color::rgb(90, 120, 210),
        }
    }
}

pub type Handler<M> = dyn Fn(&str) -> M + Send + Sync + 'static;

pub trait TextMode {
    fn wrap() -> Wrap;
}

#[derive(Debug, Default)]
pub struct SingleLine;
#[derive(Debug, Default)]
pub struct MultiLine;

impl TextMode for SingleLine {
    #[inline]
    fn wrap() -> Wrap {
        Wrap::None
    }
}

impl TextMode for MultiLine {
    #[inline]
    fn wrap() -> Wrap {
        Wrap::Word
    }
}

pub struct TextInput<M, Mode: TextMode = SingleLine> {
    x: i32,
    y: i32,
    w: i32,
    h: i32,

    id: Id,
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,

    padding: Vec4<i32>,

    placeholder: Option<Cow<'static, str>>,
    font_size: f32,
    line_height: f32,
    attrs: Attrs<'static>,
    colors: TextColors,

    on_change: Option<Box<Handler<M>>>,
    on_submit: Option<Box<Handler<M>>>,

    _mode: PhantomData<Mode>,
}

impl<M, Mode: TextMode + 'static> TextInput<M, Mode> {
    fn new_impl(size: Size<Length>) -> Self {
        Self {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            id: 0,
            size,
            min: Size::splat(28),
            max: Size::splat(i32::MAX),
            padding: Vec4::new(8, 6, 8, 6),
            placeholder: None,
            font_size: 14.0,
            line_height: if std::any::TypeId::of::<Mode>() == std::any::TypeId::of::<MultiLine>() {
                1.3
            } else {
                1.2
            },
            attrs: Attrs::new(),
            colors: TextColors::default(),
            on_change: None,
            on_submit: None,
            _mode: PhantomData,
        }
    }

    pub fn placeholder(mut self, text: impl Into<Cow<'static, str>>) -> Self {
        self.placeholder = Some(text.into());
        self
    }
    pub fn padding(mut self, p: Vec4<i32>) -> Self {
        self.padding = p;
        self
    }
    pub fn colors(mut self, c: TextColors) -> Self {
        self.colors = c;
        self
    }
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }
    pub fn min(mut self, s: Size<i32>) -> Self {
        self.min = s;
        self
    }
    pub fn max(mut self, s: Size<i32>) -> Self {
        self.max = s;
        self
    }

    pub fn on_change<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) -> M + Send + Sync + 'static,
    {
        self.on_change = Some(Box::new(f));
        self
    }

    pub fn on_submit<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) -> M + Send + Sync + 'static,
    {
        self.on_submit = Some(Box::new(f));
        self
    }

    #[inline]
    fn inner_bounds(&self) -> (i32, i32, i32, i32) {
        let l = self.x + self.padding.x;
        let t = self.y + self.padding.y;
        let r = self.x + self.w - self.padding.z;
        let b = self.y + self.h - self.padding.w;
        (l, t, r, b)
    }

    fn ensure_state<'b>(
        &self,
        view_state: &'b mut ViewState,
        fs: &mut cosmic_text::FontSystem,
    ) -> &'b mut TextInputViewState {
        let desired = Metrics::relative(self.font_size, self.line_height);

        let state = view_state.ensure(self.id, || {
            TextInputViewState::new(fs, self.font_size, self.line_height)
        });

        if state.buffer.metrics().font_size != desired.font_size
            || state.buffer.metrics().line_height != desired.line_height
        {
            state.buffer.set_metrics(fs, desired);
        }

        state
    }

    fn state<'b>(&self, view_state: &'b ViewState) -> Option<&'b TextInputViewState> {
        view_state.get::<TextInputViewState>(&self.id)
    }

    fn state_mut<'b>(&self, view_state: &'b mut ViewState) -> Option<&'b mut TextInputViewState> {
        view_state.get_mut::<TextInputViewState>(&self.id)
    }

    /// Compute the pixel (x, y, height) of the cursor by finding the glyph at cursor.index
    /// in the layout runs, using cosmic_text's own layout data.
    fn compute_cursor_rect(
        buffer: &Buffer,
        cursor: Cursor,
        l: i32,
        t: i32,
        font_size: f32,
        line_height: f32,
    ) -> Option<(f32, f32, f32)> {
        let line_advance = font_size * line_height;
        let caret_h = font_size * 1.1;

        // Walk layout runs to find the one matching our cursor line,
        // then find the glyph span containing cursor.index.
        let mut last_matching_line_y: Option<f32> = None;
        let mut last_matching_end_x: f32 = 0.0;

        for run in buffer.layout_runs() {
            if run.line_i != cursor.line {
                // Track the last run before our cursor line for positioning
                // a cursor on an empty trailing line.
                if run.line_i < cursor.line {
                    last_matching_line_y = Some(run.line_y);
                }
                continue;
            }
            last_matching_line_y = Some(run.line_y);

            // Check if cursor is at or before the first glyph
            if let Some(first) = run.glyphs.first()
                && cursor.index <= first.start
            {
                let cx = l as f32 + first.x;
                let cy = t as f32 + run.line_y - font_size * 0.9;
                return Some((cx, cy, caret_h));
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
                    let cx = l as f32 + x;
                    let cy = t as f32 + run.line_y - font_size * 0.9;
                    return Some((cx, cy, caret_h));
                }
                last_matching_end_x = glyph.x + glyph.w;
            }
        }

        // The cursor's line had runs but the index was past all glyphs (end of line).
        if let Some(line_y) = last_matching_line_y {
            // Check if cursor is on a line with no layout runs of its own
            // (e.g. an empty line after a newline). In that case,
            // the cursor line_i won't have matched any run.line_i.
            // We detect this: if we never found a run with run.line_i == cursor.line,
            // place the cursor one line_advance below the last preceding run.
            let has_run_on_cursor_line = buffer.layout_runs().any(|r| r.line_i == cursor.line);

            if !has_run_on_cursor_line {
                let cx = l as f32;
                let cy = t as f32 + line_y + line_advance - font_size * 0.9;
                return Some((cx, cy, caret_h));
            }

            let cx = l as f32 + last_matching_end_x;
            let cy = t as f32 + line_y - font_size * 0.9;
            return Some((cx, cy, caret_h));
        }

        // Completely empty buffer — place cursor at origin.
        let baseline = buffer
            .layout_runs()
            .next()
            .map(|r| r.line_y)
            .unwrap_or(line_advance);
        let cx = l as f32;
        let cy = t as f32 + baseline - font_size * 0.9;
        Some((cx, cy, caret_h))
    }

    fn prepare_text_and_caret(&mut self, ctx: &mut PrepareCtx) {
        let (l, t, r, _b) = self.inner_bounds();
        let available_w = (r - l).max(1) as f32;

        let fs = ctx.text.font_system_mut();
        let st = self.ensure_state(ctx.view_state, fs);

        let show_placeholder = st.value.is_empty();
        let text = if show_placeholder {
            self.placeholder.as_deref().unwrap_or("")
        } else {
            st.value.as_str()
        };

        let mut attrs = self.attrs.clone();
        if show_placeholder {
            attrs = attrs.color(cosmic_text::Color::rgba(
                self.colors.placeholder.r(),
                self.colors.placeholder.g(),
                self.colors.placeholder.b(),
                self.colors.placeholder.a(),
            ));
        } else {
            attrs = attrs.color(cosmic_text::Color::rgba(
                self.colors.text.r(),
                self.colors.text.g(),
                self.colors.text.b(),
                self.colors.text.a(),
            ));
        }

        // Shape the main buffer.
        {
            let b = &mut st.buffer;
            if st.width != self.w {
                st.width = self.w;
            }
            b.set_wrap(fs, Mode::wrap());
            b.set_text(fs, text, &attrs, Shaping::Basic);
            b.set_size(fs, Some(available_w), None);
            b.shape_until_scroll(fs, false);
        }
        for glyph in st.buffer.layout_runs().flat_map(|r| r.glyphs) {
            if let Some((_, size, key)) = ctx.text.prepare_glyph_data(glyph) {
                let _ = ctx
                    .text
                    .upload_glyph(ctx.gpu, ctx.texture, key, size.width, size.height);
            }
        }

        // Compute cursor rect if focused and not placeholder.
        let Some(st) = self.state_mut(ctx.view_state) else {
            return;
        };
        st.cursor_rect = if st.focused && !show_placeholder {
            Self::compute_cursor_rect(
                &st.buffer,
                st.cursor,
                l,
                t,
                self.font_size,
                self.line_height,
            )
        } else {
            None
        };
    }

    fn paint_text_and_caret(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        // Border and background.
        let border_color = if self.state(ctx.view_state).is_some_and(|s| s.focused) {
            self.colors.focus_border
        } else {
            self.colors.border
        };
        instances.push(Instance::ui(
            Position::new(self.x as f32, self.y as f32),
            Size::new(self.w as f32, self.h as f32),
            border_color,
        ));
        instances.push(Instance::ui(
            Position::new((self.x + 1) as f32, (self.y + 1) as f32),
            Size::new((self.w - 2) as f32, (self.h - 2) as f32),
            self.colors.bg,
        ));

        let (l, t, _r, _b) = self.inner_bounds();

        // Pure state access.
        let Some(st) = self.state(ctx.view_state) else {
            return;
        };

        const BASE_COLOR: Color = Color::rgba(255, 255, 255, 255);

        for run in st.buffer.layout_runs() {
            for glyph in run.glyphs {
                let Some((Position { x: left, y: top }, size, cache_key)) =
                    ctx.text.get_glyph_data(glyph)
                else {
                    continue;
                };

                let top_left = Position::new(
                    l as f32 + glyph.x + left as f32,
                    t as f32 + glyph.y + run.line_y - top as f32,
                );

                let tint = glyph
                    .color_opt
                    .map(|c| Color::rgba(c.r(), c.g(), c.b(), c.a()))
                    .unwrap_or(BASE_COLOR);

                let Some(handle) = ctx.text.lookup_glyph_handle(cache_key) else {
                    continue;
                };

                instances.push(Instance::ui_tex(
                    top_left,
                    Size::new(size.width as f32, size.height as f32),
                    tint,
                    handle,
                ));
            }
        }

        // Caret.
        if let Some((cx, cy, ch)) = st.cursor_rect {
            instances.push(Instance::ui(
                Position::new(cx, cy),
                Size::new(1.0, ch),
                self.colors.caret,
            ));
        }
    }

    /// Apply a `cosmic_text::Motion` to the cursor stored in view state.
    /// Returns true if the cursor actually moved.
    fn apply_motion(
        &self,
        view_state: &mut ViewState,
        fs: &mut cosmic_text::FontSystem,
        motion: Motion,
    ) -> bool {
        let st = match self.state_mut(view_state) {
            Some(s) => s,
            None => return false,
        };
        let old = st.cursor;
        if let Some((new_cursor, _)) = st.buffer.cursor_motion(fs, st.cursor, None, motion) {
            st.cursor = new_cursor;
        }
        st.cursor != old
    }

    /// Insert text at the current cursor position, advancing the cursor.
    fn insert_at_cursor(st: &mut TextInputViewState, text: &str) {
        let offset = st.cursor_to_byte_offset();
        st.value.insert_str(offset, text);
        let new_offset = offset + text.len();
        st.cursor = TextInputViewState::byte_offset_to_cursor(&st.value, new_offset);
    }

    /// Delete the grapheme before the cursor (backspace behaviour).
    /// Returns true if anything was deleted.
    fn delete_before_cursor(st: &mut TextInputViewState) -> bool {
        let offset = st.cursor_to_byte_offset();
        if offset == 0 || st.value.is_empty() {
            return false;
        }
        let mut new_offset = offset - 1;
        while new_offset > 0 && !st.value.is_char_boundary(new_offset) {
            new_offset -= 1;
        }
        st.value.replace_range(new_offset..offset, "");
        st.cursor = TextInputViewState::byte_offset_to_cursor(&st.value, new_offset);
        true
    }

    /// Delete the grapheme after the cursor (delete key behaviour).
    /// Returns true if anything was deleted.
    fn delete_after_cursor(st: &mut TextInputViewState) -> bool {
        let offset = st.cursor_to_byte_offset();
        if offset >= st.value.len() {
            return false;
        }
        let mut end = offset + 1;
        while end < st.value.len() && !st.value.is_char_boundary(end) {
            end += 1;
        }
        st.value.drain(offset..end);
        // Cursor byte offset stays the same, but line/index may have changed
        // (e.g. deleting a \n merges two lines).
        st.cursor = TextInputViewState::byte_offset_to_cursor(&st.value, offset);
        true
    }
}

impl<M> TextInput<M, SingleLine> {
    pub fn new(size: Size<Length>) -> Self {
        Self::new_impl(size)
    }
}
impl<M> TextInput<M, MultiLine> {
    pub fn new(size: Size<Length>) -> Self {
        let mut s = Self::new_impl(size);
        s.min = Size::new(60, 60);
        s
    }
}

impl<M, Mode: TextMode> IntoElement for TextInput<M, Mode> {}

impl<M: 'static, Mode: TextMode + 'static> Widget<M> for TextInput<M, Mode> {
    fn layout<'b>(&mut self, _ctx: &mut LayoutCtx<'b, M>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            ..Default::default()
        }
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

    fn prepare(&mut self, ctx: &mut PrepareCtx) {
        self.prepare_text_and_caret(ctx);
    }

    fn paint(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        self.paint_text_and_caret(ctx, instances);
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        let (was_hovered, hovered, focused) = {
            let st = self.ensure_state(&mut ctx.ui.view_state, ctx.text.font_system_mut());
            let inside = ctx.ui.mouse_pos.x >= self.x as f32
                && ctx.ui.mouse_pos.x < (self.x + self.w) as f32
                && ctx.ui.mouse_pos.y >= self.y as f32
                && ctx.ui.mouse_pos.y < (self.y + self.h) as f32;
            let was_hovered = st.hovered;
            st.hovered = inside;
            st.focused = ctx.ui.kbd_focus_item == Some(self.id);
            (was_hovered, st.hovered, st.focused)
        };

        let mut needs_redraw = false;
        let mut queued_emit: Option<M> = None;

        if hovered && ctx.is_mouse_released(MouseButton::Left) {
            let (l, t, _r, _b) = self.inner_bounds();
            let click_x = ctx.ui.mouse_pos.x - l as f32;
            let click_y = ctx.ui.mouse_pos.y - t as f32;

            if let Some(st) = self.state_mut(&mut ctx.ui.view_state) {
                ctx.ui.kbd_focus_item = Some(self.id);
                st.focused = true;
                if let Some(hit_cursor) = st.buffer.hit(click_x, click_y) {
                    st.cursor = hit_cursor;
                } else {
                    // Click didn't hit any glyph — place at end.
                    st.cursor = Cursor::new(0, st.value.len());
                }
            }
            needs_redraw = true;
        }

        if ctx.is_mouse_pressed(MouseButton::Left) && !hovered && focused {
            if let Some(st) = self.state_mut(&mut ctx.ui.view_state) {
                st.focused = false;
            }
            ctx.ui.kbd_focus_item = None;
        }

        if !focused {
            if was_hovered != hovered {
                ctx.ui.request_redraw();
            }
            return;
        }

        if let Some(ev) = ctx.event {
            match ev {
                UiEventRef::Text(t) => {
                    if let Some(st) = self.state_mut(&mut ctx.ui.view_state) {
                        Self::insert_at_cursor(st, &t.text);
                        if let Some(f) = &self.on_change {
                            queued_emit = Some(f(&st.value));
                        }
                        ctx.ui.request_redraw();
                    }
                }
                UiEventRef::Key(k) if k.state == KeyState::Pressed => {
                    use LogicalKey::*;
                    match k.logical_key {
                        Backspace => {
                            if let Some(st) = self.state_mut(&mut ctx.ui.view_state)
                                && Self::delete_before_cursor(st)
                            {
                                if let Some(f) = &self.on_change {
                                    queued_emit = Some(f(&st.value));
                                }
                                needs_redraw = true;
                            }
                        }
                        Delete => {
                            if let Some(st) = self.state_mut(&mut ctx.ui.view_state)
                                && Self::delete_after_cursor(st)
                            {
                                if let Some(f) = &self.on_change {
                                    queued_emit = Some(f(&st.value));
                                }
                                needs_redraw = true;
                            }
                        }
                        ArrowLeft => {
                            let fs = ctx.text.font_system_mut();
                            if self.apply_motion(&mut ctx.ui.view_state, fs, Motion::Left) {
                                needs_redraw = true;
                            }
                        }
                        ArrowRight => {
                            let fs = ctx.text.font_system_mut();
                            if self.apply_motion(&mut ctx.ui.view_state, fs, Motion::Right) {
                                needs_redraw = true;
                            }
                        }
                        ArrowUp => {
                            if TypeId::of::<Mode>() == TypeId::of::<MultiLine>() {
                                let fs = ctx.text.font_system_mut();
                                if self.apply_motion(&mut ctx.ui.view_state, fs, Motion::Up) {
                                    needs_redraw = true;
                                }
                            }
                        }
                        ArrowDown => {
                            if TypeId::of::<Mode>() == TypeId::of::<MultiLine>() {
                                let fs = ctx.text.font_system_mut();
                                if self.apply_motion(&mut ctx.ui.view_state, fs, Motion::Down) {
                                    needs_redraw = true;
                                }
                            }
                        }
                        Home => {
                            let fs = ctx.text.font_system_mut();
                            if self.apply_motion(&mut ctx.ui.view_state, fs, Motion::Home) {
                                needs_redraw = true;
                            }
                        }
                        End => {
                            let fs = ctx.text.font_system_mut();
                            if self.apply_motion(&mut ctx.ui.view_state, fs, Motion::End) {
                                needs_redraw = true;
                            }
                        }
                        Enter => {
                            if TypeId::of::<Mode>() == TypeId::of::<SingleLine>() {
                                // Submit
                                if let Some(st) = self.state_mut(&mut ctx.ui.view_state)
                                    && let Some(f) = &self.on_submit
                                {
                                    queued_emit = Some(f(&st.value));
                                }
                            } else {
                                // MultiLine: insert newline
                                if let Some(st) = self.state_mut(&mut ctx.ui.view_state) {
                                    Self::insert_at_cursor(st, "\n");
                                    if let Some(f) = &self.on_change {
                                        queued_emit = Some(f(&st.value));
                                    }
                                    needs_redraw = true;
                                }
                            }
                        }
                        Tab => {
                            if let Some(st) = self.state_mut(&mut ctx.ui.view_state) {
                                st.focused = false;
                            }
                            if ctx.ui.kbd_focus_item == Some(self.id) {
                                ctx.ui.kbd_focus_item = None;
                            }
                            needs_redraw = true;
                        }
                        Character(_) | Space => {
                            let s = match k.logical_key {
                                Space => " ",
                                Character(ref s) => s,
                                _ => unreachable!(),
                            };
                            if s.is_empty() {
                                return;
                            }
                            if let Some(st) = self.state_mut(&mut ctx.ui.view_state) {
                                Self::insert_at_cursor(st, s);
                                if let Some(f) = &self.on_change {
                                    queued_emit = Some(f(&st.value));
                                }
                                needs_redraw = true;
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if let Some(msg) = queued_emit {
            ctx.ui.emit(msg);
        }
        if needs_redraw || was_hovered != hovered {
            ctx.ui.request_redraw();
        }
    }
}

pub type TextField<M> = TextInput<M, SingleLine>;
pub type TextArea<M> = TextInput<M, MultiLine>;
