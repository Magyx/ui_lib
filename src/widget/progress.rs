use super::*;

#[derive(Widget)]
pub struct ProgressBar {
    value: f32,
    height: i32,
    indeterminate: bool,
    segment: f32,
    speed: f32,
    track_color: Option<Color>,
    fill_color: Option<Color>,
}
impl ProgressBar {
    /// A determinate bar; `value` is clamped to `[0, 1]`.
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            height: 8,
            indeterminate: false,
            segment: 0.0,
            speed: 0.0,
            track_color: None,
            fill_color: None,
        }
    }

    /// An animated bar with no known completion fraction.
    pub fn indeterminate() -> Self {
        Self {
            value: 0.0,
            height: 8,
            indeterminate: true,
            segment: 0.3,
            speed: 0.9,
            track_color: None,
            fill_color: None,
        }
    }

    pub fn segment(mut self, size: f32) -> Self {
        self.segment = size;
        self
    }
    pub fn speed(mut self, v: f32) -> Self {
        self.speed = v;
        self
    }
    pub fn height(mut self, height: i32) -> Self {
        self.height = height.max(1);
        self
    }
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = Some(color);
        self
    }
    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill_color = Some(color);
        self
    }
}
impl Widget for ProgressBar {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> Node {
        Node {
            size: Size::new(Length::Grow, Length::Fixed(self.height)),
            ..Default::default()
        }
    }

    fn child_count(&self) -> usize {
        0
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget {
        unreachable!()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        let r = ctx.rect();
        let theme = ctx.theme;
        let radius = self.height as f32 / 2.0;
        let track = self.track_color.unwrap_or(theme.surface_variant);
        let fill = self.fill_color.unwrap_or(theme.primary);

        out.push(Instance::ui_rounded(
            Position::new(r.x as f32, r.y as f32),
            Size::new(r.w as f32, r.h as f32),
            track,
            radius,
            0,
            Color::TRANSPARENT,
        ));

        let w = r.w as f32;
        let (fx, fw) = if self.indeterminate {
            let seg = w * self.segment;
            let span = w + seg;
            let t = (ctx.globals.time * self.speed).rem_euclid(1.0);
            let x = -seg + t * span;
            let left = x.clamp(0.0, w);
            let right = (x + seg).clamp(0.0, w);
            (left, (right - left).max(0.0))
        } else {
            (0.0, w * self.value)
        };

        if fw > 0.5 {
            out.push(Instance::ui_rounded(
                Position::new(r.x as f32 + fx, r.y as f32),
                Size::new(fw, r.h as f32),
                fill,
                radius,
                0,
                Color::TRANSPARENT,
            ));
        }
    }

    fn handle(&mut self, ctx: &mut EventCtx) {
        if self.indeterminate {
            ctx.ui.request_redraw();
        }
    }
}
