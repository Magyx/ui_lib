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

#[doc(hidden)]
pub struct ScrollViewState {
    pub y: i32,
    grab: Option<f32>,
    content_h: i32,
    /// Viewport height captured during paint/handle, so `children_offset`
    /// (which only sees `ViewState`) can compute max-scroll without the
    /// removed `self.h` field.
    viewport_h: i32,
}

pub struct Scrollable<M> {
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,
    child: Element<M>,

    scrollbar_behavior: ScrollBarBehavior,

    bar_color: Option<Color>,
    thumb_color: Option<Color>,
    bg: Option<Color>,
}

impl<M> Scrollable<M> {
    pub fn new<E: Into<Element<M>>>(child: E) -> Self {
        Self {
            size: Size::new(Length::Grow, Length::Grow),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            child: child.into(),
            scrollbar_behavior: ScrollBarBehavior::Auto,
            bar_color: None,
            thumb_color: None,
            bg: None,
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
        self.bg = Some(c);
        self
    }
    pub fn with_scrollbar(mut self, behavior: ScrollBarBehavior) -> Self {
        self.scrollbar_behavior = behavior;
        self
    }
    pub fn scrollbar_color(mut self, bar: Color, thumb: Color) -> Self {
        self.bar_color = Some(bar);
        self.thumb_color = Some(thumb);
        self
    }

    #[inline]
    fn track_rect(&self, r: Rect) -> (f32, f32, f32, f32) {
        let margin = 2.0;
        let track_w = 6.0;
        let tx = r.x as f32 + r.w as f32 - margin - track_w;
        let ty = r.y as f32 + margin;
        let th = r.h as f32 - 2.0 * margin;
        (tx, ty, track_w, th)
    }

    #[inline]
    fn thumb_rect(
        &self,
        r: Rect,
        state: &ScrollViewState,
        content_h: i32,
    ) -> Option<(f32, f32, f32, f32)> {
        if let ScrollBarBehavior::Hide = self.scrollbar_behavior {
            return None;
        }
        let max = (content_h - r.h).max(0);
        if max <= 0 && matches!(self.scrollbar_behavior, ScrollBarBehavior::Auto) {
            return None;
        }

        let (tx, ty, tw, th) = self.track_rect(r);

        let ch = content_h.max(r.h);
        let ratio = (r.h as f32 / ch as f32).clamp(0.0, 1.0);
        let thumb_h = ratio * th;
        let thumb_h = thumb_h.clamp(20.0, th); // min thumb size

        let t = if max > 0 {
            state.y as f32 / max as f32
        } else {
            0.0
        };
        let thumb_y = ty + (th - thumb_h) * t;

        Some((tx, thumb_y, tw, thumb_h))
    }

    fn ensure_state<'b>(&self, view_state: &'b mut ViewState, id: Id) -> &'b mut ScrollViewState {
        view_state.ensure(id, || ScrollViewState {
            y: 0,
            grab: None,
            content_h: 0,
            viewport_h: 0,
        })
    }
}

impl<M> IntoElement for Scrollable<M> {}

impl<M> Widget<M> for Scrollable<M> {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            clip_children: true,
            ..Default::default()
        }
    }

    fn child_count(&self) -> usize {
        1
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        self.child.as_mut()
    }
    fn children_offset(&self, view_state: &mut ViewState, id: Id) -> (i32, i32) {
        let st = self.ensure_state(view_state, id);
        // Heights are stashed during paint/handle (same frame values that
        // `write_back` used to bake in), so no `self.h` is needed here.
        let max = (st.content_h - st.viewport_h).max(0);
        st.y = st.y.clamp(0, max);
        (0, -st.y)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        let r = ctx.rect();
        let id = ctx.id();
        self.ensure_state(ctx.view_state, id).viewport_h = r.h;
        if let Some(bg) = self.bg {
            ctx.fill(out, r.xywh(), bg);
        }
    }

    fn paint_overlay(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        let r = ctx.rect();
        let id = ctx.id();
        let content_h = ctx.child_content_height();
        let bar = self.bar_color.unwrap_or_else(|| {
            let s = ctx.theme.surface_variant;
            Color::rgba(s.r(), s.g(), s.b(), 128)
        });
        let thumb = self.thumb_color.unwrap_or(ctx.theme.on_surface_variant);
        let state = self.ensure_state(ctx.view_state, id);
        state.content_h = content_h;
        state.viewport_h = r.h;
        if let Some((tx, ty, tw, th)) = self.thumb_rect(r, state, content_h) {
            let (track_x, track_y, track_w, track_h) = self.track_rect(r);
            out.push(Instance::ui(
                Position::new(track_x, track_y),
                Size::new(track_w, track_h),
                bar,
            ));
            out.push(Instance::ui(
                Position::new(tx, ty),
                Size::new(tw, th),
                thumb,
            ));
        }
    }

    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        const HIT_SLOP: f32 = 4.0;

        let r = ctx.rect();
        let id = ctx.id();
        let mx = ctx.ui.mouse_pos.x;
        let my = ctx.ui.mouse_pos.y;
        let inside = mx >= r.x as f32
            && mx < r.x as f32 + r.w as f32
            && my >= r.y as f32
            && my < r.y as f32 + r.h as f32;

        let content_h = ctx.child_content_height();
        let max = (content_h - r.h).max(0);
        {
            let st = self.ensure_state(&mut ctx.ui.view_state, id);
            st.content_h = content_h;
            st.viewport_h = r.h;
            st.y = st.y.clamp(0, max);
        }

        let pressed = ctx.is_mouse_pressed(MouseButton::Left);
        let down = ctx.ui.is_button_down(MouseButton::Left);
        let released = ctx.is_mouse_released(MouseButton::Left);

        if inside
            && max > 0
            && let Some(UiEventRef::MouseWheel(delta)) = ctx.event
        {
            let step = match delta.units {
                ScrollUnits::Lines => 40.0,
                ScrollUnits::Pixels => 1.0,
            };
            if delta.dy != 0.0 {
                let st = self.ensure_state(&mut ctx.ui.view_state, id);
                let old_y = st.y;
                let ny = (st.y as f32 + delta.dy * step).round() as i32;
                st.y = ny.clamp(0, max);
                if st.y != old_y {
                    ctx.ui.request_redraw();
                }
            }
        }

        let st = self.ensure_state(&mut ctx.ui.view_state, id);
        let thumb = self.thumb_rect(r, st, content_h);
        let track = self.track_rect(r);

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
                ctx.ui.hot_item = Some(id);
            }

            if (over_thumb || over_track) && pressed {
                ctx.ui.active_item = Some(id);
                let grab = if over_thumb {
                    (my - ty).clamp(0.0, th)
                } else {
                    th / 2.0
                };
                let st = self.ensure_state(&mut ctx.ui.view_state, id);
                st.grab = Some(grab);

                if over_track && !over_thumb {
                    let old_y = st.y;
                    let desired = (my - grab).clamp(track_y, track_y + track_h - th);
                    let denom = (track_h - th).max(1.0);
                    let t = (desired - track_y) / denom;
                    st.y = (t * max as f32).round() as i32;
                    if st.y != old_y {
                        ctx.ui.request_redraw();
                    }
                }
            }

            if ctx.ui.active_item == Some(id) && down {
                let st = self.ensure_state(&mut ctx.ui.view_state, id);
                let old_y = st.y;
                let mut pos = my - st.grab.unwrap_or(th / 2.0);
                pos = pos.clamp(track_y, track_y + track_h - th);

                let denom = (track_h - th).max(1.0);
                let t = (pos - track_y) / denom;
                st.y = (t * max as f32).round() as i32;
                if st.y != old_y {
                    ctx.ui.request_redraw();
                }
            }

            if released && ctx.ui.active_item == Some(id) {
                ctx.ui.active_item = None;
                let st = self.ensure_state(&mut ctx.ui.view_state, id);
                st.grab = None;
            }
        } else if released && ctx.ui.active_item == Some(id) {
            ctx.ui.active_item = None;
            let st = self.ensure_state(&mut ctx.ui.view_state, id);
            st.grab = None;
        }
    }
}
