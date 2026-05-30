use std::any::TypeId;
use std::borrow::Cow;
use std::marker::PhantomData;

use cosmic_text::{Buffer, Cursor, Motion};

use super::*;
use crate::{
    event::{KeyState, LogicalKey, MouseButton, UiEventRef},
    layout::mix64,
};

#[derive(Clone, Copy, PartialEq)]
struct CaretKey {
    cursor: Cursor,
    l: i32,
    t: i32,
    w: i32,
    focused: bool,
}

#[derive(Default)]
struct CaretCache {
    key: Option<CaretKey>,
    rect: Option<(f32, f32, f32)>,
}

pub struct TextInputViewState {
    hovered: bool,
    focused: bool,

    value: String,
    cursor: Cursor,
    caret_cache: CaretCache,
}

impl TextInputViewState {
    fn new() -> Self {
        Self {
            hovered: false,
            focused: false,

            value: String::new(),
            cursor: Cursor::new(0, 0),
            caret_cache: CaretCache::default(),
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

pub type Handler<M> = dyn Fn(&str) -> M + Send + Sync + 'static;

pub trait TextMode {
    fn wrap() -> Wrap;
    fn line_height() -> f32;
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
    #[inline]
    fn line_height() -> f32 {
        1.2
    }
}

impl TextMode for MultiLine {
    #[inline]
    fn wrap() -> Wrap {
        Wrap::Word
    }
    #[inline]
    fn line_height() -> f32 {
        1.3
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
    border: i32,

    placeholder: Option<Cow<'static, str>>,
    bg: Option<Color>,
    text_color: Option<Color>,
    caret: Option<Color>,

    child_id: Id,
    child: Text,

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
            border: 0,
            placeholder: None,
            bg: None,
            text_color: None,
            caret: None,
            child_id: 0,
            child: Text::new("")
                .font_size(14.0)
                .line_height(Mode::line_height())
                .wrap(Mode::wrap()),
            on_change: None,
            on_submit: None,
            _mode: PhantomData,
        }
    }
    pub fn family(mut self, family: Family) -> Self {
        self.child = self.child.family(family);
        self
    }
    pub fn style(mut self, style: Style) -> Self {
        self.child = self.child.style(style);
        self
    }
    pub fn weight(mut self, weight: Weight) -> Self {
        self.child = self.child.weight(weight);
        self
    }
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }
    pub fn caret_color(mut self, color: Color) -> Self {
        self.caret = Some(color);
        self
    }
    pub fn placeholder(mut self, text: impl Into<Cow<'static, str>>) -> Self {
        self.placeholder = Some(text.into());
        self
    }
    pub fn padding(mut self, p: Vec4<i32>) -> Self {
        self.padding = p;
        self
    }
    pub fn font_size(mut self, size: f32) -> Self {
        self.child = self.child.font_size(size);
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
    fn text_origin(&self) -> (i32, i32) {
        (
            self.x + self.padding.x + self.border,
            self.y + self.padding.y + self.border,
        )
    }

    fn ensure_state<'b>(&self, view_state: &'b mut ViewState) -> &'b mut TextInputViewState {
        view_state.ensure(self.id, TextInputViewState::new)
    }
    fn state<'b>(&self, view_state: &'b ViewState) -> Option<&'b TextInputViewState> {
        view_state.get::<TextInputViewState>(&self.id)
    }
    fn state_mut<'b>(&self, view_state: &'b mut ViewState) -> Option<&'b mut TextInputViewState> {
        view_state.get_mut::<TextInputViewState>(&self.id)
    }

    // TODO: this should be cached.
    /// Compute the pixel (x, y, height) of the cursor by finding the glyph at cursor.index
    /// in the layout runs, using cosmic_text's own layout data.
    fn compute_cursor_rect(
        buffer: &Buffer,
        cursor: Cursor,
        l: i32,
        t: i32,
    ) -> Option<(f32, f32, f32)> {
        let cosmic_text::Metrics {
            font_size,
            line_height,
        } = buffer.metrics();
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

    /// Apply a `cosmic_text::Motion` to the cursor stored in view state.
    /// Returns true if the cursor actually moved.
    fn apply_motion(
        &self,
        view_state: &mut ViewState,
        fs: &mut cosmic_text::FontSystem,
        motion: Motion,
    ) -> bool {
        let Some(cursor) = self.state(view_state).map(|s| s.cursor) else {
            return false;
        };
        let new = view_state
            .get_mut::<text::TextViewState>(&self.child_id)
            .and_then(|tv| tv.buffer.cursor_motion(fs, cursor, None, motion))
            .map(|(c, _)| c);
        new.is_some_and(|nc| {
            self.state_mut(view_state).is_some_and(|st| {
                let moved = st.cursor != nc;
                st.cursor = nc;
                moved
            })
        })
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

impl<M, Mode: TextMode + 'static> Widget<M> for TextInput<M, Mode> {
    fn layout<'b>(&mut self, ctx: &mut LayoutCtx<'b, M>) -> Node {
        let value = self.ensure_state(&mut ctx.ui.view_state).value.clone();
        let placeholder = value.is_empty();

        let text = if placeholder {
            self.placeholder.clone().unwrap_or_default()
        } else {
            Cow::Owned(value)
        };
        let color = if placeholder {
            ctx.theme.on_surface_variant
        } else {
            self.text_color.unwrap_or(ctx.theme.on_surface)
        };
        self.child.set_content(text, color);

        let b = ctx.theme.border_width;
        self.border = b;
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            clip_children: true,
            padding: Padding {
                left: self.padding.x + b,
                top: self.padding.y + b,
                right: self.padding.z + b,
                bottom: self.padding.w + b,
            },
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
        self.child_id = mix64(id, 1);
    }
    fn child_count(&self) -> usize {
        1
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        &mut self.child
    }

    fn prepare(&mut self, ctx: &mut PrepareCtx) {
        let Some((cursor, focused)) = self.state(ctx.view_state).map(|s| (s.cursor, s.focused))
        else {
            return;
        };
        let (l, t) = self.text_origin();
        let key = CaretKey {
            cursor,
            l,
            t,
            w: self.w,
            focused,
        };
        if self
            .state(ctx.view_state)
            .is_some_and(|s| s.caret_cache.key == Some(key))
        {
            return; // nothing relevant changed -> skip layout_runs scan
        }
        let rect = if focused {
            ctx.view_state
                .get::<text::TextViewState>(&self.child_id)
                .and_then(|tv| Self::compute_cursor_rect(&tv.buffer, cursor, l, t))
        } else {
            None
        };
        if let Some(st) = self.state_mut(ctx.view_state) {
            st.caret_cache.key = Some(key);
            st.caret_cache.rect = rect;
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        let theme = ctx.theme;
        let focused = self.state(ctx.view_state).is_some_and(|s| s.focused);
        let fill = self.bg.unwrap_or(theme.surface_variant);
        let border = if focused {
            theme.focus_outline
        } else {
            theme.outline
        };
        instances.push(Instance::ui_rounded(
            Position::new(self.x as f32, self.y as f32),
            Size::new(self.w as f32, self.h as f32),
            fill,
            theme.corner_radius,
            theme.border_width,
            border,
        ));
    }

    fn paint_overlay(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        let Some(st) = self.state(ctx.view_state) else {
            return;
        };
        if !st.focused {
            return;
        }
        if let Some((cx, cy, ch)) = st.caret_cache.rect {
            // no h-scroll yet: hide caret once it leaves the field
            if cx < self.x as f32 || cx > (self.x + self.w) as f32 {
                return;
            }
            let caret = self.caret.unwrap_or(ctx.theme.on_surface);
            instances.push(Instance::ui(
                Position::new(cx, cy),
                Size::new(1.0, ch),
                caret,
            ));
        }
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        let (was_hovered, hovered, focused) = {
            let st = self.ensure_state(&mut ctx.ui.view_state);
            let inside = ctx.ui.mouse_pos.x >= self.x as f32
                && ctx.ui.mouse_pos.x < (self.x + self.w) as f32
                && ctx.ui.mouse_pos.y >= self.y as f32
                && ctx.ui.mouse_pos.y < (self.y + self.h) as f32;
            let was_hovered = st.hovered;
            st.hovered = inside;
            st.focused = ctx.ui.kbd_focus_item == Some(self.id);
            (was_hovered, st.hovered, st.focused)
        };

        if hovered && ctx.is_mouse_released(MouseButton::Left) {
            let (l, t) = self.text_origin();
            let click_x = ctx.ui.mouse_pos.x - l as f32;
            let click_y = ctx.ui.mouse_pos.y - t as f32;

            let hit_cursor = ctx
                .ui
                .view_state
                .get::<text::TextViewState>(&self.child_id)
                .and_then(|tv| tv.buffer.hit(click_x, click_y));
            ctx.ui.kbd_focus_item = Some(self.id);
            if let Some(st) = self.state_mut(&mut ctx.ui.view_state) {
                st.focused = true;
                st.cursor = hit_cursor.unwrap_or_else(|| Cursor::new(0, st.value.len()));
            }
            ctx.ui.request_redraw();
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

        let mut needs_redraw = false;
        let mut queued_emit: Option<M> = None;
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
                            needs_redraw |=
                                self.apply_motion(&mut ctx.ui.view_state, fs, Motion::Left);
                        }
                        ArrowRight => {
                            let fs = ctx.text.font_system_mut();
                            needs_redraw |=
                                self.apply_motion(&mut ctx.ui.view_state, fs, Motion::Right);
                        }
                        ArrowUp => {
                            if TypeId::of::<Mode>() == TypeId::of::<MultiLine>() {
                                let fs = ctx.text.font_system_mut();
                                needs_redraw |=
                                    self.apply_motion(&mut ctx.ui.view_state, fs, Motion::Up);
                            }
                        }
                        ArrowDown => {
                            if TypeId::of::<Mode>() == TypeId::of::<MultiLine>() {
                                let fs = ctx.text.font_system_mut();
                                needs_redraw |=
                                    self.apply_motion(&mut ctx.ui.view_state, fs, Motion::Down);
                            }
                        }
                        Home => {
                            let fs = ctx.text.font_system_mut();
                            needs_redraw |=
                                self.apply_motion(&mut ctx.ui.view_state, fs, Motion::Home);
                        }
                        End => {
                            let fs = ctx.text.font_system_mut();
                            needs_redraw |=
                                self.apply_motion(&mut ctx.ui.view_state, fs, Motion::End);
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
