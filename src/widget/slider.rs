use crate::event::MouseButton;

use super::*;

struct SliderViewState {
    grab: Option<f32>,
}

pub struct Slider<M> {
    x: i32,
    y: i32,
    w: i32,
    h: i32,

    id: Id,
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,

    lo: f32,
    hi: f32,
    value: f32,

    track_h: i32,
    track_color: Color,
    fill_color: Color,
    knob_color: Color,
    bg_color: Color,

    on_change: Option<Box<dyn Fn(f32) -> M + Send + Sync + 'static>>,
}

impl<M> Slider<M> {
    pub fn new(size: Size<Length>, range: (f32, f32), value: f32) -> Self {
        let (lo, hi) = range;
        let value = value.clamp(lo, hi);
        Self {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            id: 0,
            size,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),

            lo,
            hi,
            value,

            track_h: 6,
            track_color: Color::rgb(70, 70, 80),
            fill_color: Color::rgb(45, 150, 245),
            knob_color: Color::rgb(220, 220, 230),
            bg_color: Color::TRANSPARENT,

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
    pub fn colors(mut self, track: Color, fill: Color, knob: Color) -> Self {
        self.track_color = track;
        self.fill_color = fill;
        self.knob_color = knob;
        self
    }
    pub fn background(mut self, c: Color) -> Self {
        self.bg_color = c;
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
    fn contains(&self, p: Position<f32>) -> bool {
        let l = self.x as f32;
        let t = self.y as f32;
        let r = l + self.w as f32;
        let b = t + self.h as f32;
        p.x >= l && p.x < r && p.y >= t && p.y < b
    }

    #[inline]
    fn knob_size(&self) -> f32 {
        if self.h <= 0 {
            return 0.0;
        }
        let th = self.track_h.clamp(2, self.h.max(2)) as f32;
        let h = self.h as f32;
        let lower = (10.0_f32).min(h);
        let upper = h;
        (th * 2.0).clamp(lower, upper)
    }

    #[inline]
    fn value_track(&self) -> (f32, f32) {
        let kw = self.knob_size();
        let left = self.x as f32 + kw / 2.0;
        let right = self.x as f32 + self.w as f32 - kw / 2.0;
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
    fn knob_center_x(&self) -> f32 {
        let (l, r) = self.value_track();
        l + self.ratio() * (r - l)
    }

    #[inline]
    fn value_per_pixel(&self) -> f32 {
        let (l, r) = self.value_track();
        let w = (r - l).max(1.0);
        (self.hi - self.lo).abs() / w
    }

    fn set_from_cursor(&mut self, mx: f32) -> bool {
        if self.w <= 0 || self.hi <= self.lo {
            return false;
        }
        let (l, r) = self.value_track();
        let denom = (r - l).max(1.0);
        let t = ((mx - l) / denom).clamp(0.0, 1.0);
        let new_v = self.lo + t * (self.hi - self.lo);

        let threshold = self.value_per_pixel() * 0.5;
        let changed = (new_v - self.value).abs() >= threshold
            || (new_v == self.lo && self.value != self.lo)
            || (new_v == self.hi && self.value != self.hi);

        if changed {
            self.value = new_v;
        }
        changed
    }

    fn ensure_state<'b>(&self, view_state: &'b mut ViewState) -> &'b mut SliderViewState {
        view_state.ensure(self.id, || SliderViewState { grab: None })
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

    fn paint(&mut self, _ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        if self.bg_color.a() != 0 {
            out.push(Instance::ui(
                Position::new(self.x as f32, self.y as f32),
                Size::new(self.w as f32, self.h as f32),
                self.bg_color,
            ));
        }

        let th = self.track_h.clamp(2, self.h.max(2)) as f32;
        let ty = self.y as f32 + (self.h as f32 - th) / 2.0;

        out.push(Instance::ui(
            Position::new(self.x as f32, ty),
            Size::new(self.w as f32, th),
            self.track_color,
        ));

        let kcx = self.knob_center_x();
        let kw = self.knob_size();
        let kx = kcx - kw / 2.0;
        let fw = (kx - self.x as f32).max(0.0);
        out.push(Instance::ui(
            Position::new(self.x as f32, ty),
            Size::new(fw, th),
            self.fill_color,
        ));
        let ky = self.y as f32 + (self.h as f32 - kw) / 2.0;
        out.push(Instance::ui(
            Position::new(kx, ky),
            Size::new(kw, kw),
            self.knob_color,
        ));
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        let inside = self.contains(ctx.ui.mouse_pos);
        if inside {
            ctx.ui.hot_item = Some(self.id);
        }

        let kcx = self.knob_center_x();
        let kw = self.knob_size();
        let over_knob =
            inside && ctx.ui.mouse_pos.x >= kcx - kw / 2.0 && ctx.ui.mouse_pos.x < kcx + kw / 2.0;

        let mut changed = false;

        if inside && ctx.is_mouse_pressed(MouseButton::Left) {
            ctx.ui.active_item = Some(self.id);

            if over_knob {
                let offset = ctx.ui.mouse_pos.x - kcx;
                self.ensure_state(&mut ctx.ui.view_state).grab = Some(offset);
            } else {
                self.ensure_state(&mut ctx.ui.view_state).grab = Some(0.0);
                changed |= self.set_from_cursor(ctx.ui.mouse_pos.x);
            }
        }

        if ctx.ui.active_item == Some(self.id) && ctx.ui.is_button_down(MouseButton::Left) {
            let offset = self
                .ensure_state(&mut ctx.ui.view_state)
                .grab
                .unwrap_or(0.0);
            changed |= self.set_from_cursor(ctx.ui.mouse_pos.x - offset);
        }

        if ctx.is_mouse_released(MouseButton::Left) && ctx.ui.active_item == Some(self.id) {
            let offset = self
                .ensure_state(&mut ctx.ui.view_state)
                .grab
                .unwrap_or(0.0);
            changed |= self.set_from_cursor(ctx.ui.mouse_pos.x - offset);
            self.ensure_state(&mut ctx.ui.view_state).grab = None;
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
