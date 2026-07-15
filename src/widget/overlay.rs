use super::*;

struct Absolute {
    inner: Element,
    offx: i32,
    offy: i32,
}

impl Absolute {
    fn new(child: Element, offx: i32, offy: i32) -> Self {
        Self {
            inner: child,
            offx,
            offy,
        }
    }
}

impl IntoElement for Absolute {}

impl Widget for Absolute {
    fn layout<'a>(&mut self, ctx: &mut LayoutCtx<'a>) -> Node {
        let mut n = self.inner.as_mut().layout(ctx);
        n.is_absolute = true;
        n.offset_pos.x += self.offx;
        n.offset_pos.y += self.offy;
        n
    }
    fn key(&self) -> Option<u64> {
        self.inner.as_ref().key()
    }
    fn child_count(&self) -> usize {
        self.inner.as_ref().child_count()
    }
    fn child_mut(&mut self, i: usize) -> &mut dyn Widget {
        self.inner.as_mut().child_mut(i)
    }
    fn child_env(&self, env: Env, theme: &Theme) -> Env {
        self.inner.as_ref().child_env(env, theme)
    }
    fn min_height_for_width<'a>(&mut self, ctx: &mut LayoutCtx<'a>, width: i32) -> Option<i32> {
        self.inner.as_mut().min_height_for_width(ctx, width)
    }
    fn children_offset(&self, view_state: &mut ViewState, id: Id) -> (i32, i32) {
        self.inner.as_ref().children_offset(view_state, id)
    }
    fn prepare(&mut self, ctx: &mut PrepareCtx) {
        self.inner.as_mut().prepare(ctx);
    }
    fn prepare_overlay(&mut self, ctx: &mut PrepareCtx) {
        self.inner.as_mut().prepare_overlay(ctx);
    }
    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        self.inner.as_mut().paint(ctx, out);
    }
    fn paint_overlay(&mut self, ctx: &mut PaintCtx, instancess: &mut Vec<Instance>) {
        self.inner.as_mut().paint_overlay(ctx, instancess);
    }
    fn handle(&mut self, ctx: &mut EventCtx) {
        self.inner.as_mut().handle(ctx);
    }
    fn handle_after(&mut self, ctx: &mut EventCtx) {
        self.inner.as_mut().handle_after(ctx);
    }
}

pub struct Overlay {
    children: Vec<Absolute>,
    size: Size<Length>,
    color: Color,
    padding: Vec4<i32>,
    min: Size<i32>,
    max: Size<i32>,
    modal: bool,
}

impl Overlay {
    pub fn empty() -> Self {
        Self::new::<Vec<_>, Element>(el!())
    }
    pub fn new<I, E>(children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Element>,
    {
        let wrapped = children
            .into_iter()
            .map(|c| Absolute::new(c.into(), 0, 0))
            .collect();
        Self {
            children: wrapped,
            size: Size::splat(Length::Fit),
            color: Color::TRANSPARENT,
            padding: Vec4::splat(0),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            modal: false,
        }
    }
    pub fn size(mut self, size: Size<Length>) -> Self {
        self.size = size;
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
    pub fn modal(mut self) -> Self {
        self.modal = true;
        if self.color.a() == 0 {
            self.color = Color::rgba(0, 0, 0, 120);
        }
        self
    }
    pub fn padding(mut self, amount: Vec4<i32>) -> Self {
        self.padding = amount;
        self
    }
    pub fn min(mut self, size: Size<i32>) -> Self {
        self.min = size;
        self
    }
    pub fn max(mut self, size: Size<i32>) -> Self {
        self.max = size;
        self
    }

    pub fn push<E>(&mut self, element: E, x: i32, y: i32)
    where
        E: Into<Element>,
    {
        self.children.push(Absolute::new(element.into(), x, y));
    }
}

impl IntoElement for Overlay {}

impl Widget for Overlay {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            layout_dir: Axis::Horizontal,
            padding: Padding {
                left: self.padding.x,
                top: self.padding.y,
                right: self.padding.z,
                bottom: self.padding.w,
            },
            ..Default::default()
        }
    }

    fn child_count(&self) -> usize {
        self.children.len()
    }
    fn child_mut(&mut self, i: usize) -> &mut dyn Widget {
        &mut self.children[i]
    }

    fn focus_trap(&self) -> bool {
        self.modal
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        if self.color.a() > 0 {
            let r = ctx.rect();
            out.push(Instance::ui(
                Position::new(r.x as f32, r.y as f32),
                Size::new(r.w as f32, r.h as f32),
                self.color,
            ));
        }
    }
}
