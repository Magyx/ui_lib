use std::any::TypeId;
use std::borrow::Cow;
use std::marker::PhantomData;

use super::*;
use crate::{
    event::{KeyState, LogicalKey, MouseButton, UiEventRef},
    layout::mix64,
    text::{Family, FontStyle, Motion, TextCursor, Weight, Wrap},
};

#[derive(Clone, Copy, PartialEq)]
struct CaretKey {
    cursor: TextCursor,
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

    cursor: TextCursor,
    selection_anchor: Option<TextCursor>,
    dragging: bool,
    caret_cache: CaretCache,
}

impl TextInputViewState {
    fn new() -> Self {
        Self {
            hovered: false,
            focused: false,

            cursor: TextCursor::new(0, 0),
            selection_anchor: None,
            dragging: false,
            caret_cache: CaretCache::default(),
        }
    }

    fn cursor_to_byte_offset(&self, value: &str) -> usize {
        cursor_byte_offset(value, self.cursor)
    }

    /// The selected byte range as `(start, end)` with `start <= end`, or `None`
    /// when there is no (non-empty) selection.
    fn selection_range(&self, value: &str) -> Option<(usize, usize)> {
        let anchor = self.selection_anchor?;
        let a = cursor_byte_offset(value, anchor);
        let c = self.cursor_to_byte_offset(value);
        if a == c {
            None
        } else {
            Some((a.min(c), a.max(c)))
        }
    }

    fn has_selection(&self, value: &str) -> bool {
        self.selection_range(value).is_some()
    }

    #[allow(unused)]
    /// The currently selected text, if any.
    fn selected_text(&self, value: &str) -> Option<String> {
        let (s, e) = self.selection_range(value)?;
        let len = value.len();
        Some(value[s.min(len)..e.min(len)].to_string())
    }

    /// Delete the selected range, placing the cursor at its start. Returns true
    /// if a non-empty selection was removed. Always clears the anchor.
    fn delete_selection(&mut self, value: &mut String) -> bool {
        let removed = if let Some((s, e)) = self.selection_range(value) {
            value.replace_range(s..e, "");
            self.cursor = Self::byte_offset_to_cursor(value, s);
            true
        } else {
            false
        };
        self.selection_anchor = None;
        removed
    }

    /// Select the entire contents. Returns true if there was anything to select.
    fn select_all(&mut self, value: &str) -> bool {
        if value.is_empty() {
            return false;
        }
        self.selection_anchor = Some(TextCursor::new(0, 0));
        self.cursor = Self::byte_offset_to_cursor(value, value.len());
        true
    }

    /// Convert a flat byte offset into `value` to a `Cursor` (line, index-within-line).
    fn byte_offset_to_cursor(value: &str, offset: usize) -> TextCursor {
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
        TextCursor::new(line, offset - line_start)
    }
}

pub type Handler<M> = dyn Fn(&str) -> M + 'static;

fn cursor_byte_offset(value: &str, cursor: TextCursor) -> usize {
    let mut offset = 0usize;
    let mut line = 0usize;
    // Walk through `value` counting newlines to find where cursor.line starts.
    for (i, ch) in value.char_indices() {
        if line == cursor.line {
            // We've reached the target line. Add the within-line index.
            return (i + cursor.index).min(value.len());
        }
        if ch == '\n' {
            line += 1;
            offset = i + 1;
        }
    }
    // If the cursor line matches the last line (no trailing \n found mid-loop)
    if line == cursor.line {
        return (offset + cursor.index).min(value.len());
    }
    // Cursor line is past all lines — clamp to end.
    value.len()
}

/// Order two cursors so the returned pair is `(start, end)` in document order.
fn order_cursors(a: TextCursor, b: TextCursor) -> (TextCursor, TextCursor) {
    if (a.line, a.index) <= (b.line, b.index) {
        (a, b)
    } else {
        (b, a)
    }
}

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

#[derive(Widget)]
pub struct TextInput<M, Mode: TextMode = SingleLine> {
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,
    padding: Vec4<i32>,
    border: i32,

    value: Cow<'static, str>,
    placeholder: Option<Cow<'static, str>>,
    bg: Option<Color>,
    text_color: Option<Color>,
    caret: Option<Color>,

    child: Text,

    on_change: Option<Box<Handler<M>>>,
    on_submit: Option<Box<Handler<M>>>,

    _mode: PhantomData<Mode>,
}
impl<M, Mode: TextMode + 'static> TextInput<M, Mode> {
    fn new_impl<S: Into<Cow<'static, str>>>(value: S, size: Size<Length>) -> Self {
        Self {
            size,
            min: Size::splat(28),
            max: Size::splat(i32::MAX),
            padding: Vec4::new(8, 6, 8, 6),
            border: 0,
            value: value.into(),
            placeholder: None,
            bg: None,
            text_color: None,
            caret: None,
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
    pub fn style(mut self, style: FontStyle) -> Self {
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
        F: Fn(&str) -> M + 'static,
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
    fn text_origin(&self, r: Rect) -> (i32, i32) {
        (
            r.x + self.padding.x + self.border,
            r.y + self.padding.y + self.border,
        )
    }

    fn ensure_state<'b>(
        &self,
        view_state: &'b mut ViewState,
        id: Id,
    ) -> &'b mut TextInputViewState {
        view_state.ensure(id, TextInputViewState::new)
    }
    fn state<'b>(&self, view_state: &'b ViewState, id: Id) -> Option<&'b TextInputViewState> {
        view_state.get::<TextInputViewState>(&id)
    }
    fn state_mut<'b>(
        &self,
        view_state: &'b mut ViewState,
        id: Id,
    ) -> Option<&'b mut TextInputViewState> {
        view_state.get_mut::<TextInputViewState>(&id)
    }

    fn apply_motion(
        &self,
        view_state: &mut ViewState,
        id: Id,
        motion: Motion,
        extend: bool,
    ) -> bool {
        if self.value.is_empty() {
            return false;
        }
        let Some((cursor, sel)) = self
            .state(view_state, id)
            .map(|s| (s.cursor, s.selection_range(&self.value)))
        else {
            return false;
        };
        let mut changed = false;

        if !extend {
            if let Some((s, e)) = sel {
                // Plain Left/Right collapses the selection to an edge.
                let edge = match motion {
                    Motion::Left => Some(s),
                    Motion::Right => Some(e),
                    _ => None,
                };
                if let Some(off) = edge {
                    if let Some(st) = self.state_mut(view_state, id) {
                        st.cursor = TextInputViewState::byte_offset_to_cursor(&self.value, off);
                        st.selection_anchor = None;
                    }
                    return true;
                }
                // Other motions: drop the selection, then move from the cursor.
                if let Some(st) = self.state_mut(view_state, id) {
                    st.selection_anchor = None;
                }
                changed = true;
            }
        } else if let Some(st) = self.state_mut(view_state, id)
            && st.selection_anchor.is_none()
        {
            st.selection_anchor = Some(cursor);
            changed = true;
        }

        let new = view_state
            .get_mut::<text::TextViewState>(&mix64(id, 1))
            .and_then(|tv| tv.buffer.cursor_motion(cursor, motion));

        if let Some(nc) = new
            && let Some(st) = self.state_mut(view_state, id)
        {
            if st.cursor != nc {
                changed = true;
            }
            st.cursor = nc;
        }
        changed
    }

    /// Insert text at the current cursor position, advancing the cursor.
    /// If there is an active selection it is replaced.
    fn insert_at_cursor(st: &mut TextInputViewState, value: &mut String, text: &str) {
        st.delete_selection(value);
        let offset = st.cursor_to_byte_offset(value);
        value.insert_str(offset, text);
        let new_offset = offset + text.len();
        st.cursor = TextInputViewState::byte_offset_to_cursor(value, new_offset);
        st.selection_anchor = None;
    }

    /// Delete the grapheme before the cursor (backspace behaviour).
    /// If there is an active selection it is removed instead.
    /// Returns true if anything was deleted.
    fn delete_before_cursor(st: &mut TextInputViewState, value: &mut String) -> bool {
        if st.delete_selection(value) {
            return true;
        }
        let offset = st.cursor_to_byte_offset(value);
        if offset == 0 || value.is_empty() {
            return false;
        }
        let mut new_offset = offset - 1;
        while new_offset > 0 && !value.is_char_boundary(new_offset) {
            new_offset -= 1;
        }
        value.replace_range(new_offset..offset, "");
        st.cursor = TextInputViewState::byte_offset_to_cursor(value, new_offset);
        true
    }

    /// Delete the grapheme after the cursor (delete key behaviour).
    /// If there is an active selection it is removed instead.
    /// Returns true if anything was deleted.
    fn delete_after_cursor(st: &mut TextInputViewState, value: &mut String) -> bool {
        if st.delete_selection(value) {
            return true;
        }
        let offset = st.cursor_to_byte_offset(value);
        if offset >= value.len() {
            return false;
        }
        let mut end = offset + 1;
        while end < value.len() && !value.is_char_boundary(end) {
            end += 1;
        }
        value.drain(offset..end);
        // Cursor byte offset stays the same, but line/index may have changed
        // (e.g. deleting a \n merges two lines).
        st.cursor = TextInputViewState::byte_offset_to_cursor(value, offset);
        true
    }

    /// Apply an in-place edit to a fresh working copy of the current value.
    fn edit_value(
        &self,
        view_state: &mut ViewState,
        id: Id,
        edit: impl FnOnce(&mut TextInputViewState, &mut String) -> bool,
    ) -> Option<String> {
        let mut value = self.value.as_ref().to_owned();
        let st = self.state_mut(view_state, id)?;
        edit(st, &mut value).then_some(value)
    }

    /// Hit-test the current mouse position to a text cursor.
    fn hit_cursor(&self, ctx: &EventCtx) -> Option<TextCursor> {
        let id = ctx.id();
        if self.value.is_empty() {
            return Some(TextCursor::new(0, 0));
        }
        let (l, t) = self.text_origin(ctx.rect());
        let cx = ctx.ui.mouse_pos.x - l as f32;
        let cy = ctx.ui.mouse_pos.y - t as f32;
        ctx.ui
            .view_state
            .get::<text::TextViewState>(&mix64(id, 1))
            .and_then(|tv| tv.buffer.hit(cx, cy))
    }
}

impl<M> TextInput<M, SingleLine> {
    pub fn new<S: Into<Cow<'static, str>>>(value: S, size: Size<Length>) -> Self {
        Self::new_impl(value, size)
    }
}
impl<M> TextInput<M, MultiLine> {
    pub fn new<S: Into<Cow<'static, str>>>(value: S, size: Size<Length>) -> Self {
        let mut s = Self::new_impl(value, size);
        s.min = Size::new(60, 60);
        s
    }
}

impl<M: 'static, Mode: TextMode + 'static> Widget for TextInput<M, Mode> {
    fn layout<'b>(&mut self, ctx: &mut LayoutCtx<'b>) -> Node {
        let id = ctx.id();
        self.ensure_state(ctx.view_state, id);
        let placeholder = self.value.is_empty();

        let text = if placeholder {
            self.placeholder.clone().unwrap_or_default()
        } else {
            self.value.clone()
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
    fn child_count(&self) -> usize {
        1
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget {
        &mut self.child
    }

    fn focusable(&self) -> bool {
        true
    }
    fn paint_focus_ring(&self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        const FOCUS_BORDER: i32 = 2;
        let r = ctx.rect();
        let theme = ctx.theme;
        out.push(Instance::ui_rounded(
            Position::new(r.x as f32, r.y as f32),
            Size::new(r.w as f32, r.h as f32),
            Color::TRANSPARENT,
            theme.corner_radius,
            FOCUS_BORDER,
            theme.focus_outline,
        ));
    }

    fn prepare(&mut self, ctx: &mut PrepareCtx) {
        let id = ctx.id();
        let Some((cursor, focused)) = self
            .state(ctx.view_state, id)
            .map(|s| (s.cursor, s.focused))
        else {
            return;
        };
        let r = ctx.rect();
        let (l, t) = self.text_origin(r);
        let key = CaretKey {
            cursor,
            l,
            t,
            w: r.w,
            focused,
        };
        if self
            .state(ctx.view_state, id)
            .is_some_and(|s| s.caret_cache.key == Some(key))
        {
            return; // nothing relevant changed -> skip layout_runs scan
        }
        let rect = if focused {
            ctx.view_state
                .get::<text::TextViewState>(&mix64(id, 1))
                .and_then(|tv| tv.buffer.cursor_rect(cursor))
                .map(|r| (l as f32 + r.x, t as f32 + r.y, r.height))
        } else {
            None
        };
        if let Some(st) = self.state_mut(ctx.view_state, id) {
            st.caret_cache.key = Some(key);
            st.caret_cache.rect = rect;
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        let r = ctx.rect();
        let theme = ctx.theme;
        let fill = self.bg.unwrap_or(theme.surface_variant);
        out.push(Instance::ui_rounded(
            Position::new(r.x as f32, r.y as f32),
            Size::new(r.w as f32, r.h as f32),
            fill,
            theme.corner_radius,
            theme.border_width,
            theme.outline,
        ));
    }

    fn paint_overlay(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        const SELECTION_ALPHA: u8 = 96;
        const SELECTION_PAD_X: f32 = 1.5;
        const SELECTION_PAD_Y: f32 = 1.0;

        let id = ctx.id();

        let rect = ctx.rect();
        let Some(st) = self.state(ctx.view_state, id) else {
            return;
        };
        if !st.focused {
            return;
        }

        let clip_l = rect.x as f32;
        let clip_r = (rect.x + rect.w) as f32;

        if !self.value.is_empty()
            && let Some(anchor) = st.selection_anchor
            && anchor != st.cursor
        {
            let (start, end) = order_cursors(anchor, st.cursor);
            let (l, t) = self.text_origin(rect);
            let p = ctx.theme.primary_container;
            let sel_color = Color::rgba(p.r(), p.g(), p.b(), SELECTION_ALPHA);
            if let Some(buf) = ctx
                .view_state
                .get::<text::TextViewState>(&mix64(id, 1))
                .map(|tv| &tv.buffer)
            {
                for r in buf.selection_rects(start, end) {
                    let x0 = (l as f32 + r.x - SELECTION_PAD_X).max(clip_l);
                    let x1 = (l as f32 + r.x + r.width + SELECTION_PAD_X).min(clip_r);
                    let w = x1 - x0;
                    if w <= 0.0 {
                        continue;
                    }
                    out.push(Instance::ui(
                        Position::new(x0, t as f32 + r.top - SELECTION_PAD_Y),
                        Size::new(w, r.height + 2.0 * SELECTION_PAD_Y),
                        sel_color,
                    ));
                }
            }
        }

        if let Some((cx, cy, ch)) = st.caret_cache.rect {
            // no h-scroll yet: hide caret once it leaves the field
            if cx < rect.x as f32 || cx > (rect.x + rect.w) as f32 {
                return;
            }
            let caret = self.caret.unwrap_or(ctx.theme.on_surface);
            out.push(Instance::ui(
                Position::new(cx, cy),
                Size::new(1.0, ch),
                caret,
            ));
        }
    }

    fn handle(&mut self, ctx: &mut EventCtx) {
        let r = ctx.rect();
        let id = ctx.id();
        let (was_hovered, hovered, focused) = {
            let focused = ctx.is_focused();
            let is_pointer = ctx.pointer_available();
            let st = self.ensure_state(&mut ctx.ui.view_state, id);
            let inside = is_pointer && r.contains(ctx.ui.mouse_pos);
            let was_hovered = st.hovered;
            st.hovered = inside;
            st.focused = focused;
            (was_hovered, st.hovered, st.focused)
        };

        let shift = ctx.ui.modifiers.shift;

        // Mouse press inside: focus, then either extend the selection (Shift) or
        // place a fresh caret and arm a potential drag-select.
        if hovered && ctx.is_mouse_pressed(MouseButton::Left) {
            let hit = self.hit_cursor(ctx);
            ctx.request_focus();
            if let Some(st) = self.state_mut(&mut ctx.ui.view_state, id) {
                st.focused = true;
                let c = hit.unwrap_or_else(|| TextCursor::new(0, self.value.len()));
                if shift {
                    if st.selection_anchor.is_none() {
                        st.selection_anchor = Some(st.cursor);
                    }
                } else {
                    st.selection_anchor = Some(c);
                }
                st.cursor = c;
                st.dragging = true;
            }
            ctx.ui.request_redraw();
        }

        // Mouse drag: extend the selection while the button is held down.
        if focused
            && ctx.ui.is_button_down(MouseButton::Left)
            && matches!(ctx.event, Some(UiEventRef::CursorMoved { .. }))
        {
            let dragging = self
                .state(&ctx.ui.view_state, id)
                .is_some_and(|s| s.dragging);
            if dragging && let Some(c) = self.hit_cursor(ctx) {
                let moved = self
                    .state_mut(&mut ctx.ui.view_state, id)
                    .is_some_and(|st| {
                        let m = st.cursor != c;
                        st.cursor = c;
                        m
                    });
                if moved {
                    ctx.ui.request_redraw();
                }
            }
        }

        // Mouse release: end any drag. A plain release without a preceding press
        // (e.g. a synthetic click) still places the caret.
        if hovered && ctx.is_mouse_released(MouseButton::Left) {
            let hit = self.hit_cursor(ctx);
            ctx.request_focus();
            if let Some(st) = self.state_mut(&mut ctx.ui.view_state, id) {
                st.focused = true;
                let was_dragging = st.dragging;
                st.dragging = false;
                if !was_dragging {
                    let c = hit.unwrap_or_else(|| TextCursor::new(0, self.value.len()));
                    if shift {
                        if st.selection_anchor.is_none() {
                            st.selection_anchor = Some(st.cursor);
                        }
                    } else {
                        st.selection_anchor = Some(c);
                    }
                    st.cursor = c;
                }
            }
            ctx.ui.request_redraw();
        }

        // Click outside while focused: unfocus.
        if ctx.is_mouse_pressed(MouseButton::Left) && !hovered && focused {
            if let Some(st) = self.state_mut(&mut ctx.ui.view_state, id) {
                st.focused = false;
                st.dragging = false;
            }
            ctx.clear_focus();
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
                    if let Some(v) = self.edit_value(&mut ctx.ui.view_state, id, |st, value| {
                        Self::insert_at_cursor(st, value, &t.text);
                        true
                    }) {
                        if let Some(f) = &self.on_change {
                            queued_emit = Some(f(&v));
                        }
                        ctx.ui.request_redraw();
                    }
                }
                UiEventRef::Key(k) if k.state == KeyState::Pressed => {
                    use LogicalKey::*;
                    let cmd = ctx.ui.modifiers.control || ctx.ui.modifiers.super_;

                    if cmd && let Character(s) = &k.logical_key {
                        match s.to_lowercase().as_str() {
                            "a" => {
                                if let Some(st) = self.state_mut(&mut ctx.ui.view_state, id)
                                    && st.select_all(&self.value)
                                {
                                    needs_redraw = true;
                                }
                            }
                            "c" => { // TODO: Copy
                            }
                            "x" => { // TODO: Cut
                            }
                            "v" => { // TODO: Paste
                            }
                            // Any other Ctrl/Cmd+letter is swallowed (not typed).
                            _ => {}
                        }
                        if let Some(msg) = queued_emit.take() {
                            ctx.emit(msg);
                        }
                        if needs_redraw {
                            ctx.ui.request_redraw();
                        }
                        return;
                    }

                    let extend = shift;
                    match k.logical_key {
                        Backspace => {
                            if let Some(v) =
                                self.edit_value(&mut ctx.ui.view_state, id, |st, value| {
                                    Self::delete_before_cursor(st, value)
                                })
                            {
                                if let Some(f) = &self.on_change {
                                    queued_emit = Some(f(&v));
                                }
                                needs_redraw = true;
                            }
                        }
                        Delete => {
                            if let Some(v) =
                                self.edit_value(&mut ctx.ui.view_state, id, |st, value| {
                                    Self::delete_after_cursor(st, value)
                                })
                            {
                                if let Some(f) = &self.on_change {
                                    queued_emit = Some(f(&v));
                                }
                                needs_redraw = true;
                            }
                        }
                        Escape => {
                            // Clear a selection if present, otherwise unfocus.
                            let had_sel = self
                                .state(&ctx.ui.view_state, id)
                                .is_some_and(|s| s.has_selection(&self.value));
                            if had_sel {
                                if let Some(st) = self.state_mut(&mut ctx.ui.view_state, id) {
                                    st.selection_anchor = None;
                                }
                            } else {
                                if let Some(st) = self.state_mut(&mut ctx.ui.view_state, id) {
                                    st.focused = false;
                                }
                                ctx.clear_focus();
                            }
                            needs_redraw = true;
                        }
                        ArrowLeft => {
                            needs_redraw |=
                                self.apply_motion(&mut ctx.ui.view_state, id, Motion::Left, extend);
                        }
                        ArrowRight => {
                            needs_redraw |= self.apply_motion(
                                &mut ctx.ui.view_state,
                                id,
                                Motion::Right,
                                extend,
                            );
                        }
                        ArrowUp => {
                            if TypeId::of::<Mode>() == TypeId::of::<MultiLine>() {
                                needs_redraw |= self.apply_motion(
                                    &mut ctx.ui.view_state,
                                    id,
                                    Motion::Up,
                                    extend,
                                );
                            }
                        }
                        ArrowDown => {
                            if TypeId::of::<Mode>() == TypeId::of::<MultiLine>() {
                                needs_redraw |= self.apply_motion(
                                    &mut ctx.ui.view_state,
                                    id,
                                    Motion::Down,
                                    extend,
                                );
                            }
                        }
                        Home => {
                            needs_redraw |=
                                self.apply_motion(&mut ctx.ui.view_state, id, Motion::Home, extend);
                        }
                        End => {
                            needs_redraw |=
                                self.apply_motion(&mut ctx.ui.view_state, id, Motion::End, extend);
                        }
                        Enter => {
                            if TypeId::of::<Mode>() == TypeId::of::<SingleLine>() {
                                // Submit
                                if let Some(f) = &self.on_submit {
                                    queued_emit = Some(f(&self.value));
                                }
                            } else {
                                // MultiLine: insert newline
                                if let Some(v) =
                                    self.edit_value(&mut ctx.ui.view_state, id, |st, value| {
                                        Self::insert_at_cursor(st, value, "\n");
                                        true
                                    })
                                {
                                    if let Some(f) = &self.on_change {
                                        queued_emit = Some(f(&v));
                                    }
                                    needs_redraw = true;
                                }
                            }
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
                            if let Some(v) =
                                self.edit_value(&mut ctx.ui.view_state, id, |st, value| {
                                    Self::insert_at_cursor(st, value, s);
                                    true
                                })
                            {
                                if let Some(f) = &self.on_change {
                                    queued_emit = Some(f(&v));
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
            ctx.emit(msg);
        }
        if needs_redraw || was_hovered != hovered {
            ctx.ui.request_redraw();
        }
    }
}

pub type TextField<M> = TextInput<M, SingleLine>;
pub type TextArea<M> = TextInput<M, MultiLine>;
