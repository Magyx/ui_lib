use crate::event::MouseButton;

use super::*;

struct SliderViewState {
    grab: Option<f32>,
}

pub struct Slider<M> {
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,

    lo: f32,
    hi: f32,
    value: f32,

    track_h: i32,
    track_color: Option<Color>,
    fill_color: Option<Color>,
    knob_color: Option<Color>,
    bg_color: Option<Color>,

    on_change: Option<Box<dyn Fn(f32) -> M + Send + Sync + 'static>>,
}

impl<M> Slider<M> {
    pub fn new(size: Size<Length>, range: (f32, f32), value: f32) -> Self {
        let (lo, hi) = range;
        let value = value.clamp(lo, hi);
        Self {
            size,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),

            lo,
            hi,
            value,

            track_h: 6,
            track_color: None,
            fill_color: None,
            knob_color: None,
            bg_color: None,

            on_change: None,
        }
    }
    pub fn on_change<F>(mut self, f: F) -> Self
    where
        F: Fn(f32) -> M + Send + Sync + 'static,
    {
        self.on_change = Some(Box::new(f));
        self
    }
    pub fn track_color(mut self, c: Color) -> Self {
        self.track_color = Some(c);
        self
    }
    pub fn fill_color(mut self, c: Color) -> Self {
        self.fill_color = Some(c);
        self
    }
    pub fn knob_color(mut self, c: Color) -> Self {
        self.knob_color = Some(c);
        self
    }
    pub fn background(mut self, c: Color) -> Self {
        self.bg_color = Some(c);
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

    #[inline]
    fn knob_size(&self, r: Rect) -> f32 {
        if r.h <= 0 {
            return 0.0;
        }
        let th = self.track_h.clamp(2, r.h.max(2)) as f32;
        let h = r.h as f32;
        let lower = (10.0_f32).min(h);
        let upper = h;
        (th * 2.0).clamp(lower, upper)
    }

    #[inline]
    fn value_track(&self, r: Rect) -> (f32, f32) {
        let kw = self.knob_size(r);
        let left = r.x as f32 + kw / 2.0;
        let right = r.x as f32 + r.w as f32 - kw / 2.0;
        (left, right.max(left))
    }

    #[inline]
    fn ratio(&self) -> f32 {
        if self.hi > self.lo {
            (self.value - self.lo) / (self.hi - self.lo)
        } else {
            0.0
        }
    }

    #[inline]
    fn knob_center_x(&self, r: Rect) -> f32 {
        let (l, rr) = self.value_track(r);
        l + self.ratio() * (rr - l)
    }

    #[inline]
    fn value_per_pixel(&self, r: Rect) -> f32 {
        let (l, rr) = self.value_track(r);
        let w = (rr - l).max(1.0);
        (self.hi - self.lo).abs() / w
    }

    fn set_from_cursor(&mut self, r: Rect, mx: f32) -> bool {
        if r.w <= 0 || self.hi <= self.lo {
            return false;
        }
        let (l, rr) = self.value_track(r);
        let denom = (rr - l).max(1.0);
        let t = ((mx - l) / denom).clamp(0.0, 1.0);
        let new_v = self.lo + t * (self.hi - self.lo);

        let threshold = self.value_per_pixel(r) * 0.5;
        let changed = (new_v - self.value).abs() >= threshold
            || (new_v == self.lo && self.value != self.lo)
            || (new_v == self.hi && self.value != self.hi);

        if changed {
            self.value = new_v;
        }
        changed
    }

    fn ensure_state<'b>(&self, view_state: &'b mut ViewState, id: Id) -> &'b mut SliderViewState {
        view_state.ensure(id, || SliderViewState { grab: None })
    }
}

impl<M> IntoElement for Slider<M> {}

impl<M: 'static> Widget<M> for Slider<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            ..Default::default()
        }
    }

    fn child_count(&self) -> usize {
        0
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        unreachable!()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        let theme = ctx.theme;
        let r = ctx.rect();
        let track = self.track_color.unwrap_or(theme.surface_variant);
        let fill_c = self.fill_color.unwrap_or(theme.primary);
        let knob = self.knob_color.unwrap_or(theme.on_surface);
        if let Some(bg) = self.bg_color {
            ctx.fill(out, r.xywh(), bg);
        }

        let th = self.track_h.clamp(2, r.h.max(2)) as f32;
        let ty = r.y as f32 + (r.h as f32 - th) / 2.0;
        out.push(Instance::ui_rounded(
            Position::new(r.x as f32, ty),
            Size::new(r.w as f32, th),
            track,
            theme.corner_radius,
            0,
            Color::TRANSPARENT,
        ));

        let kcx = self.knob_center_x(r);
        let kw = self.knob_size(r);
        let kx = kcx - kw / 2.0;
        let fw = (kx - r.x as f32).max(0.0);
        out.push(Instance::ui_rounded(
            Position::new(r.x as f32, ty),
            Size::new(fw, th),
            fill_c,
            theme.corner_radius,
            0,
            Color::TRANSPARENT,
        ));
        // TODO: should probably add per stroke control for rounding
        out.push(Instance::ui(
            Position::new((r.x + 6) as f32, ty),
            Size::new(fw - 6.0, th),
            fill_c,
        ));

        let ky = r.y as f32 + (r.h as f32 - kw) / 2.0;
        out.push(Instance::ui_rounded(
            Position::new(kx, ky),
            Size::new(kw, kw),
            knob,
            theme.corner_radius,
            theme.border_width / 2,
            theme.outline,
        ));
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        let r = ctx.rect();
        let id = ctx.id();
        let inside = r.contains(ctx.ui.mouse_pos);
        if inside {
            ctx.ui.hot_item = Some(id);
        }

        let kcx = self.knob_center_x(r);
        let kw = self.knob_size(r);
        let over_knob =
            inside && ctx.ui.mouse_pos.x >= kcx - kw / 2.0 && ctx.ui.mouse_pos.x < kcx + kw / 2.0;

        let mut changed = false;

        if inside && ctx.is_mouse_pressed(MouseButton::Left) {
            ctx.ui.active_item = Some(id);

            if over_knob {
                let offset = ctx.ui.mouse_pos.x - kcx;
                self.ensure_state(&mut ctx.ui.view_state, id).grab = Some(offset);
            } else {
                self.ensure_state(&mut ctx.ui.view_state, id).grab = Some(0.0);
                changed |= self.set_from_cursor(r, ctx.ui.mouse_pos.x);
            }
        }

        if ctx.ui.active_item == Some(id) && ctx.ui.is_button_down(MouseButton::Left) {
            let offset = self
                .ensure_state(&mut ctx.ui.view_state, id)
                .grab
                .unwrap_or(0.0);
            changed |= self.set_from_cursor(r, ctx.ui.mouse_pos.x - offset);
        }

        if ctx.is_mouse_released(MouseButton::Left) && ctx.ui.active_item == Some(id) {
            let offset = self
                .ensure_state(&mut ctx.ui.view_state, id)
                .grab
                .unwrap_or(0.0);
            changed |= self.set_from_cursor(r, ctx.ui.mouse_pos.x - offset);
            self.ensure_state(&mut ctx.ui.view_state, id).grab = None;
            ctx.ui.active_item = None;
        }

        if changed {
            if let Some(ref cb) = self.on_change {
                ctx.ui.emit(cb(self.value));
            }
            ctx.ui.request_redraw();
        }
    }
}
