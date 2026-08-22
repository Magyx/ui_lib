use crate::{
    primitive::LayerShift,
    widget::{Positioned, prelude::*},
};

#[derive(Widget)]
pub struct Stack {
    children: Vec<Positioned>,
    size: Size<Length>,
    color: Color,
    padding: Inset,
    min: Size<i32>,
    max: Size<i32>,
    modal: bool,
    align: Align2,
    layer: LayerShift,
}
impl Stack {
    pub fn empty() -> Self {
        Self::new::<Vec<_>, Element>(el!())
    }
    pub fn new<I, E>(children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Element>,
    {
        let wrapped = children.into_iter().map(|c| Positioned::new(c)).collect();
        Self {
            children: wrapped,
            size: Size::splat(Length::Fit),
            color: Color::TRANSPARENT,
            padding: Inset::ZERO,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            modal: false,
            align: Align2::TOP_LEFT,
            layer: LayerShift::Inherit,
        }
    }

    /// Where children that didn't ask for a placement of their own sit.
    /// `Stack::new(..).align(Align2::CENTER)` centres everything.
    pub fn align(mut self, a: Align2) -> Self {
        self.align = a;
        self
    }
    pub fn size(mut self, size: Size<Length>) -> Self {
        self.size = size;
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
    pub const MODAL_LAYER: u16 = 1000;
    pub fn raise(mut self, n: u16) -> Self {
        self.layer = LayerShift::Above(n);
        self
    }
    pub fn layer(mut self, shift: LayerShift) -> Self {
        self.layer = shift;
        self
    }
    pub fn modal(mut self) -> Self {
        self.modal = true;
        if self.color.a() == 0 {
            self.color = Color::rgba(0, 0, 0, 120);
        }
        self
    }
    pub fn padding(mut self, amount: impl Into<Inset>) -> Self {
        self.padding = amount.into();
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

    /// Position travels with the child now, so this takes no coordinates:
    /// `stack.push(badge.pinned(Align2::TOP_RIGHT))`.
    pub fn push<E>(&mut self, element: E)
    where
        E: Into<Element>,
    {
        self.children.push(Positioned::new(element));
    }
}
impl Widget for Stack {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> Node {
        // Children that didn't ask for a placement get one at `align`.
        // `set_fallback` leaves an explicitly placed child alone, including
        // one wrapped by `push`/`new` after it was already positioned.
        let default = Placement::Absolute {
            anchor: self.align,
            origin: self.align,
            offset: Position::new(0, 0),
            edges: Edges::NONE,
        };
        for c in &mut self.children {
            c.default_to(default);
        }
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            layout_dir: Axis::Horizontal,
            padding: self.padding,
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

    fn layer_shift(&self) -> LayerShift {
        self.layer
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
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
