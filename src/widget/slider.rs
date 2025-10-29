use super::*;

pub struct Slider<M> {
    x: i32,
    y: i32,
    w: i32,
    h: i32,

    id: Id,
    size: Size<Length>,
    min_px: Size<i32>,
    max_px: Size<i32>,

    lo: f32,
    hi: f32,
    value: f32,

    track_h: i32,
    track_color: Color,
    fill_color: Color,
    knob_color: Color,
    bg_color: Color,

    hovered: bool,
    dragging: bool,

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
            id: next_id(),
            size,
            min_px: Size::splat(0),
            max_px: Size::splat(i32::MAX),

            lo,
            hi,
            value,

            track_h: 6,
            track_color: Color::rgb(70, 70, 80),
            fill_color: Color::rgb(45, 150, 245),
            knob_color: Color::rgb(220, 220, 230),
            bg_color: Color::TRANSPARENT,

            hovered: false,
            dragging: false,

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
        self.min_px = s;
        self
    }
    pub fn max(mut self, s: Size<i32>) -> Self {
        self.max_px = s;
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

    fn set_from_cursor(&mut self, mx: f32) -> bool {
        if self.w <= 0 {
            return false;
        }
        let t = ((mx - self.x as f32) / self.w as f32).clamp(0.0, 1.0);
        let new_v = self.lo + t * (self.hi - self.lo);
        let changed = (new_v - self.value).abs() > f32::EPSILON;
        if changed {
            self.value = new_v;
        }
        changed
    }
}

impl<M> IntoElement for Slider<M> {}

impl<M: 'static> Widget<M> for Slider<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
        Node {
            width: self.size.width,
            height: self.size.height,
            min_width: self.min_px.width,
            min_height: self.min_px.height,
            max_width: self.max_px.width,
            max_height: self.max_px.height,
            ..Default::default()
        }
    }

    fn set_layout(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.x = x;
        self.y = y;
        self.w = w;
        self.h = h;
    }

    fn child_count(&self) -> usize {
        0
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        unreachable!()
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        // optional background
        if self.bg_color.a() != 0 {
            out.push(Instance::ui(
                Position::new(self.x, self.y),
                Size::new(self.w, self.h),
                self.bg_color,
            ));
        }

        // track
        let th = self.track_h.clamp(2, self.h.max(2));
        let ty = self.y + (self.h - th) / 2;
        out.push(Instance::ui(
            Position::new(self.x, ty),
            Size::new(self.w, th),
            self.track_color,
        ));

        // fill
        let ratio = if self.hi > self.lo {
            (self.value - self.lo) / (self.hi - self.lo)
        } else {
            0.0
        };
        let fw = (ratio * self.w as f32).round() as i32;
        out.push(Instance::ui(
            Position::new(self.x, ty),
            Size::new(fw, th),
            self.fill_color,
        ));

        // knob
        let kw = (th * 2).clamp(10, (self.h * 3) / 4);
        let kx = self.x + (fw - kw / 2).clamp(0, self.w - kw);
        let ky = self.y + (self.h - kw) / 2;
        out.push(Instance::ui(
            Position::new(kx, ky),
            Size::new(kw, kw),
            self.knob_color,
        ));
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        // children first (none), then self — same pattern as Button. :contentReference[oaicite:5]{index=5}
        let was_hovered = self.hovered;
        let was_dragging = self.dragging;

        let inside = self.contains(ctx.ui.mouse_pos);
        self.hovered = inside;
        if inside {
            ctx.ui.hot_item = Some(self.id);
        }

        if inside && ctx.ui.mouse_pressed {
            ctx.ui.active_item = Some(self.id);
        }
        self.dragging = ctx.ui.active_item == Some(self.id) && ctx.ui.mouse_down;

        let mut changed = false;
        if self.dragging {
            changed |= self.set_from_cursor(ctx.ui.mouse_pos.x);
        }

        if ctx.ui.mouse_released && ctx.ui.active_item == Some(self.id) {
            // snap on release too
            changed |= self.set_from_cursor(ctx.ui.mouse_pos.x);
            ctx.ui.active_item = None;
        }

        if changed {
            if let Some(ref cb) = self.on_change {
                ctx.ui.emit(cb(self.value));
            }
            ctx.ui.request_redraw();
        } else if self.hovered != was_hovered || self.dragging != was_dragging {
            ctx.ui.request_redraw();
        }
    }
}
