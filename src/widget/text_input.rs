use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::{borrow::Cow, collections::HashMap};

use super::*;
use crate::event::{KeyState, LogicalKey, MouseButton, UiEventRef};
use cosmic_text::{Attrs, Buffer, Metrics, Shaping, Wrap};

pub struct TextInputViewState {
    buffer: Buffer,
    value: String,
    width: i32,
    caret: usize,
}

impl TextInputViewState {
    fn new(fs: &mut cosmic_text::FontSystem, font_size: f32, line_height: f32) -> Self {
        Self {
            buffer: Buffer::new(fs, Metrics::relative(font_size, line_height)),
            value: String::new(),
            width: 0,
            caret: 0,
        }
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
    fn handle_enter<M>(
        state: &mut TextInputViewState,
        on_change: &Option<Box<Handler<M>>>,
        on_submit: &Option<Box<Handler<M>>>,
        ctx: &mut EventCtx<M>,
    );
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
    fn handle_enter<M>(
        state: &mut TextInputViewState,
        _on_change: &Option<Box<dyn Fn(&str) -> M + Send + Sync + 'static>>,
        on_submit: &Option<Box<dyn Fn(&str) -> M + Send + Sync + 'static>>,
        ctx: &mut EventCtx<M>,
    ) {
        if let Some(f) = on_submit {
            ctx.ui.emit(f(&state.value));
        }
    }
}

impl TextMode for MultiLine {
    #[inline]
    fn wrap() -> Wrap {
        Wrap::Word
    }
    #[inline]
    fn handle_enter<M>(
        state: &mut TextInputViewState,
        on_change: &Option<Box<dyn Fn(&str) -> M + Send + Sync + 'static>>,
        _on_submit: &Option<Box<dyn Fn(&str) -> M + Send + Sync + 'static>>,
        ctx: &mut EventCtx<M>,
    ) {
        let caret = state.caret;
        state.value.insert(caret, '\n');
        state.caret += 1;
        if let Some(f) = on_change {
            ctx.ui.emit(f(&state.value));
            ctx.ui.request_redraw();
        }
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

    hovered: bool,
    focused: bool,

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
            hovered: false,
            focused: false,
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
        view_state: &'b mut HashMap<Id, Box<dyn Any>>,
        fs: &mut cosmic_text::FontSystem,
    ) -> &'b mut TextInputViewState {
        dbg!(&self.id);
        view_state
            .entry(self.id)
            .or_insert_with(|| {
                Box::new(TextInputViewState::new(
                    fs,
                    self.font_size,
                    self.line_height,
                ))
            })
            .downcast_mut::<TextInputViewState>()
            .expect("View state was wrong type")
    }

    fn state_mut<'b>(
        &self,
        view_state: &'b mut HashMap<Id, Box<dyn Any>>,
    ) -> Option<&'b mut TextInputViewState> {
        view_state
            .get_mut(&self.id)?
            .downcast_mut::<TextInputViewState>()
    }

    fn paint_text_and_caret(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        let border_color = if self.focused {
            self.colors.focus_border
        } else {
            self.colors.border
        };
        instances.push(Instance::ui(
            Position::new(self.x, self.y),
            Size::new(self.w, self.h),
            border_color,
        ));
        instances.push(Instance::ui(
            Position::new(self.x + 1, self.y + 1),
            Size::new(self.w - 2, self.h - 2),
            self.colors.bg,
        ));

        let (l, t, r, _b) = self.inner_bounds();
        let available_w = (r - l).max(1) as f32;

        let st = self.ensure_state(ctx.view_state, ctx.text.font_system_mut());
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

        {
            let fs = ctx.text.font_system_mut();
            let b = &mut st.buffer;
            if st.width != self.w {
                st.width = self.w;
            }
            b.set_wrap(fs, Mode::wrap());
            b.set_text(fs, text, &attrs, Shaping::Basic);
            b.set_size(fs, Some(available_w), None);
            b.shape_until_scroll(fs, false);
        }

        const BASE_COLOR: cosmic_text::Color = cosmic_text::Color::rgba(255, 255, 255, 255);

        for run in st.buffer.layout_runs() {
            for glyph in run.glyphs {
                let (top_left, width, height, cache_key, tint) = {
                    let (Position { x: left, y: top }, Size { width, height }, cache_key) =
                        match ctx.text.get_glyph_data(glyph) {
                            Some(v) => v,
                            None => continue,
                        };

                    let top_left = Position::new(
                        (l as f32 + glyph.x).round() as i32 + left,
                        (t as f32 + glyph.y + run.line_y).round() as i32 - top,
                    );

                    let glyph_color = glyph.color_opt.unwrap_or(BASE_COLOR);
                    let tint = Color::rgba(
                        glyph_color.r(),
                        glyph_color.g(),
                        glyph_color.b(),
                        glyph_color.a(),
                    );

                    (top_left, width, height, cache_key, tint)
                };

                let handle =
                    match ctx
                        .text
                        .upload_glyph(ctx.gpu, ctx.texture, cache_key, width, height)
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

        if self.focused && !show_placeholder {
            let (caret_x, caret_y, caret_h) = {
                let fs = ctx.text.font_system_mut();
                let prefix = &st.value[..st.caret];
                let mut pb = Buffer::new(fs, Metrics::relative(self.font_size, self.line_height));
                pb.set_wrap(fs, Mode::wrap());
                pb.set_text(fs, prefix, &self.attrs, Shaping::Basic);
                pb.set_size(fs, Some(available_w), None);
                pb.shape_until_scroll(fs, false);

                let mut x_advance = 0.0f32;
                let mut baseline = 0.0f32;
                for run in pb.layout_runs() {
                    baseline = run.line_y;
                    for g in run.glyphs {
                        x_advance = g.x + g.w;
                    }
                }
                let caret_x = (l as f32 + x_advance).round() as i32;
                let caret_h = (self.font_size * 1.1) as i32;
                let caret_y = (t as f32 + baseline - self.font_size * 0.9).round() as i32;
                (caret_x, caret_y, caret_h)
            };

            instances.push(Instance::ui(
                Position::new(caret_x, caret_y),
                Size::new(1, caret_h),
                self.colors.caret,
            ));
        }
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
    fn layout<'b>(&mut self, _ctx: &mut LayoutCtx<'b, M>) -> Node {
        Node {
            width: self.size.width,
            height: self.size.height,
            min_width: self.min.width,
            min_height: self.min.height,
            max_width: self.max.width,
            max_height: self.max.height,
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
    fn paint(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        self.paint_text_and_caret(ctx, instances);
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        let inside = ctx.ui.mouse_pos.x >= self.x as f32
            && ctx.ui.mouse_pos.x < (self.x + self.w) as f32
            && ctx.ui.mouse_pos.y >= self.y as f32
            && ctx.ui.mouse_pos.y < (self.y + self.h) as f32;
        let was_hovered = self.hovered;
        self.hovered = inside;
        self.focused = ctx.ui.kbd_focus_item == Some(self.id);

        let mut queued_emit: Option<M> = None;
        let mut needs_redraw = false;

        if inside && ctx.ui.is_button_released(MouseButton::Left) {
            ctx.ui.kbd_focus_item = Some(self.id);
            self.focused = true;
            if let Some(st) = self.state_mut(&mut ctx.ui.view_state) {
                st.caret = st.value.len();
            }
            needs_redraw = true;
        }
        if ctx.ui.is_button_pressed(MouseButton::Left) && !inside && self.focused {
            ctx.ui.kbd_focus_item = None;
            self.focused = false;
        }
        if !self.focused {
            if was_hovered != self.hovered {
                ctx.ui.request_redraw();
            }
            return;
        }

        if let Some(ev) = ctx.event {
            match ev {
                UiEventRef::Text(t) => {
                    if let Some(st) = self.state_mut(&mut ctx.ui.view_state) {
                        let caret = st.caret;
                        st.value.insert_str(caret, &t.text);
                        st.caret += t.text.len();
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
                            let mut changed = false;
                            {
                                if let Some(st) = self.state_mut(&mut ctx.ui.view_state)
                                    && st.caret > 0
                                    && !st.value.is_empty()
                                {
                                    let caret = st.caret;
                                    let mut new_caret = caret - 1;
                                    while new_caret > 0 && !st.value.is_char_boundary(new_caret) {
                                        new_caret -= 1;
                                    }
                                    st.value.replace_range(new_caret..caret, "");
                                    st.caret = new_caret;
                                    if let Some(f) = &self.on_change {
                                        queued_emit = Some(f(&st.value));
                                    }
                                    changed = true;
                                }
                            }
                            if changed {
                                needs_redraw = true;
                            }
                        }
                        Delete => {
                            let mut changed = false;
                            {
                                if let Some(st) = self.state_mut(&mut ctx.ui.view_state) {
                                    let caret = st.caret;
                                    if caret < st.value.len() {
                                        let mut end = caret + 1;
                                        while end < st.value.len()
                                            && !st.value.is_char_boundary(end)
                                        {
                                            end += 1;
                                        }
                                        st.value.drain(caret..end);
                                        if let Some(f) = &self.on_change {
                                            queued_emit = Some(f(&st.value));
                                        }
                                        changed = true;
                                    }
                                }
                            }
                            if changed {
                                needs_redraw = true;
                            }
                        }
                        ArrowLeft => {
                            let mut moved = false;
                            {
                                if let Some(st) = self.state_mut(&mut ctx.ui.view_state)
                                    && st.caret > 0
                                {
                                    let mut idx = st.caret - 1;
                                    while idx > 0 && !st.value.is_char_boundary(idx) {
                                        idx -= 1;
                                    }
                                    st.caret = idx;
                                    moved = true;
                                }
                            }
                            if moved {
                                needs_redraw = true;
                            }
                        }

                        ArrowRight => {
                            let mut moved = false;
                            {
                                if let Some(st) = self.state_mut(&mut ctx.ui.view_state)
                                    && st.caret < st.value.len()
                                {
                                    let mut idx = st.caret + 1;
                                    while idx < st.value.len() && !st.value.is_char_boundary(idx) {
                                        idx += 1;
                                    }
                                    st.caret = idx;
                                    moved = true;
                                }
                            }
                            if moved {
                                needs_redraw = true;
                            }
                        }

                        Home => {
                            let mut moved = false;
                            {
                                if let Some(st) = self.state_mut(&mut ctx.ui.view_state)
                                    && st.caret != 0
                                {
                                    st.caret = 0;
                                    moved = true;
                                }
                            }
                            if moved {
                                needs_redraw = true;
                            }
                        }

                        End => {
                            let mut moved = false;
                            {
                                if let Some(st) = self.state_mut(&mut ctx.ui.view_state) {
                                    let end = st.value.len();
                                    if st.caret != end {
                                        st.caret = end;
                                        moved = true;
                                    }
                                }
                            }
                            if moved {
                                needs_redraw = true;
                            }
                        }

                        Enter => {
                            if TypeId::of::<Mode>() == TypeId::of::<SingleLine>() {
                                // Submit current value
                                let to_emit = {
                                    if let Some(st) = self.state_mut(&mut ctx.ui.view_state) {
                                        if let Some(f) = &self.on_submit {
                                            Some(f(&st.value))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                };
                                if to_emit.is_some() {
                                    queued_emit = to_emit;
                                }
                            } else {
                                // MultiLine: insert '\n' and treat as change
                                let mut changed = false;
                                {
                                    if let Some(st) = self.state_mut(&mut ctx.ui.view_state) {
                                        let caret = st.caret;
                                        st.value.insert(caret, '\n');
                                        st.caret += 1;
                                        if let Some(f) = &self.on_change {
                                            queued_emit = Some(f(&st.value));
                                        }
                                        changed = true;
                                    }
                                }
                                if changed {
                                    needs_redraw = true;
                                }
                            }
                        }

                        Tab => {
                            self.focused = false;
                            if ctx.ui.kbd_focus_item == Some(self.id) {
                                ctx.ui.kbd_focus_item = None;
                            }
                            needs_redraw = true;
                        }

                        Character(ref s) if !s.is_empty() => {
                            let mut changed = false;
                            {
                                if let Some(st) = self.state_mut(&mut ctx.ui.view_state) {
                                    let caret = st.caret;
                                    st.value.insert_str(caret, s);
                                    st.caret += s.len();
                                    if let Some(f) = &self.on_change {
                                        queued_emit = Some(f(&st.value));
                                    }
                                    changed = true;
                                }
                            }
                            if changed {
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
        if needs_redraw || was_hovered != self.hovered {
            ctx.ui.request_redraw();
        }
    }
}

pub type TextField<M> = TextInput<M, SingleLine>;
pub type TextArea<M> = TextInput<M, MultiLine>;
