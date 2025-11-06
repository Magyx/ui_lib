use std::{any::Any, collections::HashMap};

use super::*;
use crate::{
    event::{MouseButton, ScrollUnits, UiEventRef},
    primitive::Instance,
};

#[derive(Debug)]
pub enum ScrollBarBehavior {
    Auto,
    Show,
    Hide,
}

struct ScrollViewState {
    y: i32,
    grab: Option<i32>,
}

pub struct Scrollable<M> {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,
    child: Element<M>,

    id: Id,
    content_h: i32,

    scrollbar_behavior: ScrollBarBehavior,

    bar_color: Color,
    thumb_color: Color,
    bg: Color,
}

impl<M: 'static> Scrollable<M> {
    pub fn new<E: Into<Element<M>>>(child: E) -> Self {
        Self {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            size: Size::new(Length::Grow, Length::Fit),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            id: 0,
            child: child.into(),
            content_h: 0,
            scrollbar_behavior: ScrollBarBehavior::Auto,
            bar_color: Color::rgba(70, 70, 80, 128),
            thumb_color: Color::rgb(200, 200, 210),
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
    pub fn with_scrollbar(mut self, behavior: ScrollBarBehavior) -> Self {
        self.scrollbar_behavior = behavior;
        self
    }
    pub fn scrollbar_color(mut self, bar: Color, thumb: Color) -> Self {
        self.bar_color = bar;
        self.thumb_color = thumb;
        self
    }

    #[inline]
    fn track_rect(&self) -> (i32, i32, i32, i32) {
        let margin = 2;
        let track_w = 6;
        let tx = self.x + self.w - margin - track_w;
        let ty = self.y + margin;
        let th = self.h - 2 * margin;
        (tx, ty, track_w, th)
    }

    #[inline]
    fn thumb_rect(&self, state: &ScrollViewState) -> Option<(i32, i32, i32, i32)> {
        if let ScrollBarBehavior::Hide = self.scrollbar_behavior {
            return None;
        }
        let max = (self.content_h - self.h).max(0);
        if max <= 0 && matches!(self.scrollbar_behavior, ScrollBarBehavior::Auto) {
            return None;
        }

        let (tx, ty, tw, th) = self.track_rect();

        let ch = self.content_h.max(self.h);
        let ratio = (self.h as f32 / ch as f32).clamp(0.0, 1.0);
        let thumb_h = (ratio * th as f32).round() as i32;
        let thumb_h = thumb_h.clamp(20, th); // min thumb size

        let t = if max > 0 {
            state.y as f32 / max as f32
        } else {
            0.0
        };
        let thumb_y = ty + ((th - thumb_h) as f32 * t).round() as i32;

        Some((tx, thumb_y, tw, thumb_h))
    }

    fn ensure_state<'b>(
        &self,
        view_state: &'b mut HashMap<Id, Box<dyn Any>>,
    ) -> &'b mut ScrollViewState {
        view_state
            .entry(self.id)
            .or_insert_with(|| Box::new(ScrollViewState { y: 0, grab: None }))
            .downcast_mut::<ScrollViewState>()
            .expect("View state was wrong type")
    }
}

impl<M> IntoElement for Scrollable<M> {}

impl<M: 'static> Widget<M> for Scrollable<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            clip_children: true,
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
        1
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        self.child.as_mut()
    }
    fn children_offset<'a>(&self, view_state: &mut HashMap<Id, Box<dyn Any>>) -> (i32, i32) {
        (0, -self.ensure_state(view_state).y)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        if self.bg.a() > 0 {
            out.push(Instance::ui(
                Position::new(self.x, self.y),
                Size::new(self.w, self.h),
                self.bg,
            ));
        }
        self.content_h = ctx.child_content_height();
    }

    fn paint_overlay(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        let state = self.ensure_state(ctx.view_state);
        if let Some((tx, ty, tw, th)) = self.thumb_rect(state) {
            let (track_x, track_y, track_w, track_h) = self.track_rect();
            out.push(Instance::ui(
                Position::new(track_x, track_y),
                Size::new(track_w, track_h),
                self.bar_color,
            ));
            out.push(Instance::ui(
                Position::new(tx, ty),
                Size::new(tw, th),
                self.thumb_color,
            ));
        }
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        const HIT_SLOP: i32 = 4;

        let mx = ctx.ui.mouse_pos.x as i32;
        let my = ctx.ui.mouse_pos.y as i32;
        let inside = mx >= self.x && mx < self.x + self.w && my >= self.y && my < self.y + self.h;

        let max = (self.content_h - self.h).max(0);

        let pressed = ctx.ui.is_button_pressed(MouseButton::Left);
        let down = ctx.ui.is_button_down(MouseButton::Left);
        let released = ctx.ui.is_button_released(MouseButton::Left);

        if inside
            && max > 0
            && let Some(UiEventRef::MouseWheel(delta)) = ctx.event
        {
            let step = match delta.units {
                ScrollUnits::Lines => 40.0,
                ScrollUnits::Pixels => 1.0,
            };
            if delta.dy != 0.0 {
                let st = self.ensure_state(&mut ctx.ui.view_state);
                let ny = (st.y as f32 - delta.dy * step).round() as i32; // +dy up → smaller y
                st.y = ny.clamp(0, max);
                ctx.ui.request_redraw();
            }
        }

        let st = self.ensure_state(&mut ctx.ui.view_state);
        let thumb = self.thumb_rect(st);
        let track = self.track_rect();

        if let Some((tx, ty, tw, th)) = thumb {
            let (track_x, track_y, track_w, track_h) = track;

            let over_thumb = {
                let x0 = tx - HIT_SLOP;
                let x1 = tx + tw + HIT_SLOP;
                let y0 = ty - HIT_SLOP;
                let y1 = ty + th + HIT_SLOP;
                mx >= x0 && mx < x1 && my >= y0 && my < y1
            };
            let over_track = {
                let x0 = track_x - HIT_SLOP;
                let x1 = track_x + track_w + HIT_SLOP;
                let y0 = track_y - HIT_SLOP;
                let y1 = track_y + track_h + HIT_SLOP;
                mx >= x0 && mx < x1 && my >= y0 && my < y1
            };

            if over_thumb || over_track {
                ctx.ui.hot_item = Some(self.id);
            }

            if (over_thumb || over_track) && pressed {
                ctx.ui.active_item = Some(self.id);
                let grab = if over_thumb {
                    (my - ty).clamp(0, th)
                } else {
                    th / 2
                };
                let st = self.ensure_state(&mut ctx.ui.view_state);
                st.grab = Some(grab);

                if over_track && !over_thumb {
                    let desired = (my - grab).clamp(track_y, track_y + track_h - th);
                    let denom = (track_h - th).max(1);
                    let t = (desired - track_y) as f32 / denom as f32;
                    st.y = (t * max as f32).round() as i32;
                    ctx.ui.request_redraw();
                }
            }

            if ctx.ui.active_item == Some(self.id) && down {
                let st = self.ensure_state(&mut ctx.ui.view_state);
                let mut pos = my - st.grab.unwrap_or(th / 2);
                pos = pos.clamp(track_y, track_y + track_h - th);

                let denom = (track_h - th).max(1);
                let t = (pos - track_y) as f32 / denom as f32;
                st.y = (t * max as f32).round() as i32;

                ctx.ui.request_redraw();
            }

            if released && ctx.ui.active_item == Some(self.id) {
                ctx.ui.active_item = None;
                let st = self.ensure_state(&mut ctx.ui.view_state);
                st.grab = None;
                ctx.ui.request_redraw();
            }
        } else if released && ctx.ui.active_item == Some(self.id) {
            ctx.ui.active_item = None;
            let st = self.ensure_state(&mut ctx.ui.view_state);
            st.grab = None;
        }

        let st = self.ensure_state(&mut ctx.ui.view_state);
        let saved_mouse = ctx.ui.mouse_pos;
        ctx.ui.mouse_pos.y += st.y as f32;

        let mut updated_globals = *ctx.globals;
        updated_globals.mouse_pos[1] += st.y as f32;

        self.child.as_mut().handle(&mut EventCtx {
            globals: &updated_globals,
            ui: ctx.ui,
            event: ctx.event,
        });

        ctx.ui.mouse_pos = saved_mouse;
    }
}
