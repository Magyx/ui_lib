use super::*;

struct Absolute<M> {
    inner: Element<M>,
    offx: i32,
    offy: i32,
}

impl<M> Absolute<M> {
    fn new(child: Element<M>, offx: i32, offy: i32) -> Self {
        Self {
            inner: child,
            offx,
            offy,
        }
    }
}

impl<M> IntoElement for Absolute<M> {}

impl<M> Widget<M> for Absolute<M> {
    fn layout<'a>(&mut self, ctx: &mut LayoutCtx<'a, M>) -> Node {
        let mut n = self.inner.as_mut().layout(ctx);
        n.is_absolute = true;
        n.offset_x += self.offx;
        n.offset_y += self.offy;
        n
    }
    fn set_layout(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.inner.as_mut().set_layout(x, y, w, h);
    }
    fn child_count(&self) -> usize {
        self.inner.as_ref().child_count()
    }
    fn child_mut(&mut self, i: usize) -> &mut dyn Widget<M> {
        self.inner.as_mut().child_mut(i)
    }
    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        self.inner.as_mut().paint(ctx, out);
    }
    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        self.inner.as_mut().handle(ctx);
    }
}

pub struct Overlay<M> {
    x: i32,
    y: i32,
    w: i32,
    h: i32,

    children: Vec<Absolute<M>>,
    size: Size<Length>,
    color: Color,
    padding: Vec4<i32>,
    min: Size<i32>,
    max: Size<i32>,
}

impl<M> Overlay<M> {
    pub fn new<I, E>(children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Element<M>>,
    {
        let wrapped = children
            .into_iter()
            .map(|c| Absolute::new(c.into(), 0, 0))
            .collect();
        Self {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            children: wrapped,
            size: Size::splat(Length::Fit),
            color: Color::TRANSPARENT,
            padding: Vec4::splat(0),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
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
        E: Into<Element<M>>,
    {
        self.children.push(Absolute::new(element.into(), x, y));
    }
}

impl<M> IntoElement for Overlay<M> {}

impl<M: 'static> Widget<M> for Overlay<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
        Node {
            width: self.size.width,
            height: self.size.height,
            min_width: self.min.width,
            min_height: self.min.height,
            max_width: self.max.width,
            max_height: self.max.height,
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

    fn set_layout(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.x = x;
        self.y = y;
        self.w = w;
        self.h = h;
    }

    fn child_count(&self) -> usize {
        self.children.len()
    }
    fn child_mut(&mut self, i: usize) -> &mut dyn Widget<M> {
        &mut self.children[i]
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        if self.color.a() > 0 {
            out.push(Instance::ui(
                Position::new(self.x, self.y),
                Size::new(self.w, self.h),
                self.color,
            ));
        }
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        for c in &mut self.children {
            c.handle(ctx);
        }
    }
}
