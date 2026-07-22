use super::*;

pub struct Card {
    children: Vec<Element>,
    spacing: i32,
    padding: Vec4<i32>,
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,
}

impl Card {
    pub fn new<I, E>(children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Element>,
    {
        Self {
            children: children.into_iter().map(Into::into).collect(),
            spacing: 0,
            padding: Vec4::splat(0),
            size: Size::splat(Length::Fit),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
        }
    }
    pub fn spacing(mut self, spacing: i32) -> Self {
        self.spacing = spacing;
        self
    }
    pub fn padding(mut self, padding: Vec4<i32>) -> Self {
        self.padding = padding;
        self
    }
    pub fn size(mut self, size: Size<Length>) -> Self {
        self.size = size;
        self
    }
    pub fn min(mut self, min: Size<i32>) -> Self {
        self.min = min;
        self
    }
    pub fn max(mut self, max: Size<i32>) -> Self {
        self.max = max;
        self
    }
}

impl IntoElement for Card {}

impl Widget for Card {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            layout_dir: Axis::Vertical,
            padding: Padding {
                left: self.padding.x,
                top: self.padding.y,
                right: self.padding.z,
                bottom: self.padding.w,
            },
            spacing: self.spacing,
            ..Default::default()
        }
    }

    fn child_count(&self) -> usize {
        self.children.len()
    }
    fn child_mut(&mut self, i: usize) -> &mut dyn Widget {
        self.children[i].as_mut()
    }

    fn child_env(&self, env: Env, theme: &Theme) -> Env {
        let elevation = env.elevation.saturating_add(1);
        Env {
            elevation,
            foreground: theme.on_surface_at(elevation),
            ..env
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        let r = ctx.rect();
        let elevation = ctx.env.elevation.saturating_add(1);
        ctx.surface(
            out,
            r.xywh(),
            ctx.theme.surface_at(elevation),
            Color::TRANSPARENT,
        );
    }
}
