use super::*;

pub struct Spinner {
    size: i32,
    dots: usize,
    speed: f32,
    color: Option<Color>,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            size: 24,
            dots: 8,
            speed: 1.1,
            color: None,
        }
    }

    pub fn size(mut self, size: i32) -> Self {
        self.size = size.max(4);
        self
    }
    pub fn dots(mut self, n: usize) -> Self {
        self.dots = n;
        self
    }
    pub fn speed(mut self, v: f32) -> Self {
        self.speed = v;
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}
impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoElement for Spinner {}

impl Widget for Spinner {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> Node {
        Node {
            size: Size::splat(Length::Fixed(self.size)),
            ..Default::default()
        }
    }

    fn child_count(&self) -> usize {
        0
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget {
        unreachable!()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        let r = ctx.rect();
        let base = self.color.unwrap_or(ctx.theme.primary);

        let s = self.size as f32;
        let cx = r.x as f32 + s / 2.0;
        let cy = r.y as f32 + s / 2.0;
        let dot = (s * 0.16).max(2.0);
        let ring = s / 2.0 - dot / 2.0;

        // Phase of the "head" dot, advancing over time.
        let phase = (ctx.globals.time * self.speed).rem_euclid(1.0);

        for i in 0..self.dots {
            let f = i as f32 / self.dots as f32;
            let angle = f * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            let px = cx + ring * angle.cos() - dot / 2.0;
            let py = cy + ring * angle.sin() - dot / 2.0;

            // Trailing fade: brightest at the head, dimmest just behind it.
            let rel = (phase - f).rem_euclid(1.0);
            let alpha = (1.0 - rel) * base.a() as f32;
            let color = Color::rgba(base.r(), base.g(), base.b(), alpha.clamp(0.0, 255.0) as u8);

            out.push(Instance::ui_rounded(
                Position::new(px, py),
                Size::new(dot, dot),
                color,
                dot / 2.0,
                0,
                Color::TRANSPARENT,
            ));
        }
    }

    fn handle(&mut self, ctx: &mut EventCtx) {
        ctx.ui.request_redraw();
    }
}
