use super::*;
use crate::render::texture::TextureHandle;

pub enum ContentFit {
    Fill,
    Contain,
    Cover,
}

pub struct Image {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,
    handle: TextureHandle,
    fit: ContentFit,
    tint: Color,
}

impl Image {
    pub fn new(size: Size<Length>, handle: TextureHandle) -> Self {
        Self {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            size,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            handle,
            fit: ContentFit::Fill,
            tint: Color::WHITE,
        }
    }
    pub fn tint(mut self, tint: Color) -> Self {
        self.tint = tint;
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
    pub fn fit(mut self, fit: ContentFit) -> Self {
        self.fit = fit;
        self
    }
}

impl IntoElement for Image {}

impl<M> Widget<M> for Image {
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
        0
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        unreachable!()
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        if self.tint.a() == 0 {
            return;
        }

        let sw = self.handle.size_px.width as i32;
        let sh = self.handle.size_px.height as i32;
        if sw <= 0 || sh <= 0 || self.w <= 0 || self.h <= 0 {
            return;
        }

        let dst_w = self.w as f32;
        let dst_h = self.h as f32;
        let src_w = sw as f32;
        let src_h = sh as f32;

        let (draw_w, draw_h) = match self.fit {
            ContentFit::Fill => (dst_w, dst_h),
            ContentFit::Contain => {
                let s = (dst_w / src_w).min(dst_h / src_h);
                (src_w * s, src_h * s)
            }
            ContentFit::Cover => {
                let s = (dst_w / src_w).max(dst_h / src_h);
                (src_w * s, src_h * s)
            }
        };

        let px = self.x + ((dst_w - draw_w) * 0.5).round() as i32;
        let py = self.y + ((dst_h - draw_h) * 0.5).round() as i32;
        let dw = draw_w.round().max(1.0) as i32;
        let dh = draw_h.round().max(1.0) as i32;

        let inst = Instance::ui_tex(
            Position::new(px, py),
            Size::new(dw, dh),
            self.tint,
            self.handle,
        );

        match self.fit {
            ContentFit::Cover => {
                out.push(inst.with_clip(self.x, self.y, self.w, self.h));
            }
            _ => {
                out.push(inst);
            }
        }
    }
}
