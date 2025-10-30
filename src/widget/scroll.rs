use std::{cell::RefCell, rc::Rc};

use super::*;
use crate::{
    event::{ScrollUnits, UiEventRef},
    primitive::Instance,
};

#[derive(Clone, Default)]
pub struct ScrollState(Rc<RefCell<i32>>);

impl ScrollState {}

pub struct Scrollable<M> {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,
    child: Element<M>,
    scroll_y: Rc<RefCell<i32>>,
    bg: Color,
}

impl<M> Scrollable<M> {
    pub fn new<E: Into<Element<M>>>(child: E, state: &ScrollState) -> Self {
        Self {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            size: Size::splat(Length::Fit),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            child: child.into(),
            scroll_y: state.0.clone(),
            bg: Color::TRANSPARENT,
        }
    }

    pub fn size(mut self, s: Size<Length>) -> Self {
        self.size = s;
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
    pub fn bg(mut self, c: Color) -> Self {
        self.bg = c;
        self
    }
}

impl<M> IntoElement for Scrollable<M> {}

impl<M: 'static> Widget<M> for Scrollable<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
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

    fn child_count(&self) -> usize {
        1
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        self.child.as_mut()
    }

    fn paint_descendants(&self) -> bool {
        false
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        if self.bg.a() > 0 {
            out.push(Instance::ui(
                Position::new(self.x, self.y),
                Size::new(self.w, self.h),
                self.bg,
            ));
        }

        fn paint_subtree<M>(w: &mut dyn Widget<M>, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
            w.paint(ctx, out);
            let n = w.child_count();
            for i in 0..n {
                paint_subtree(w.child_mut(i), ctx, out);
            }
        }

        let mut inner = Vec::new();
        paint_subtree(self.child.as_mut(), ctx, &mut inner);

        let dy = -*self.scroll_y.borrow();
        for inst in inner.into_iter() {
            out.push(
                inst.translate(0, dy)
                    .with_clip(self.x, self.y, self.w, self.h),
            );
        }
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        let inside = ctx.ui.mouse_pos.x >= self.x as f32
            && ctx.ui.mouse_pos.x < (self.x + self.w) as f32
            && ctx.ui.mouse_pos.y >= self.y as f32
            && ctx.ui.mouse_pos.y < (self.y + self.h) as f32;

        if inside && let Some(UiEventRef::MouseWheel(delta)) = ctx.event {
            let scale = match delta.units {
                ScrollUnits::Lines => 40.0,
                ScrollUnits::Pixels => 1.0,
            };
            let dy = (delta.dy * scale).round() as i32;
            if dy != 0 {
                let mut y = self.scroll_y.borrow_mut();
                *y = (*y - dy).max(0);
                ctx.ui.request_redraw();
            }
        }

        let scroll = *self.scroll_y.borrow();

        let saved_mouse = ctx.ui.mouse_pos;
        ctx.ui.mouse_pos.y += scroll as f32;

        let mut updated_globals = *ctx.globals;
        updated_globals.mouse_pos[1] += scroll as f32;

        self.child.as_mut().handle(&mut EventCtx {
            globals: &updated_globals,
            ui: ctx.ui,
            event: ctx.event,
        });

        ctx.ui.mouse_pos = saved_mouse;
    }
}
