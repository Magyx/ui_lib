use super::*;
use crate::render::texture::TextureHandle;

use resvg::{tiny_skia, usvg};

use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Svg {
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,

    tint: Color,
    fit: ContentFit,

    path: PathBuf,
    id: Id,
}

impl Svg {
    pub fn new(size: Size<Length>, path: impl Into<PathBuf>) -> Self {
        Self {
            size,
            min: Size::new(0, 0),
            max: Size::splat(i32::MAX),
            tint: Color::WHITE,
            fit: ContentFit::Fill,
            path: path.into(),
            id: 0,
        }
    }

    pub fn tint(mut self, tint: Color) -> Self {
        self.tint = tint;
        self
    }

    pub fn fit(mut self, fit: ContentFit) -> Self {
        self.fit = fit;
        self
    }

    pub fn min_size(mut self, min: Size<i32>) -> Self {
        self.min = min;
        self
    }

    pub fn max_size(mut self, max: Size<i32>) -> Self {
        self.max = max;
        self
    }
}

#[derive(Default)]
struct SvgState {
    // Source tracking
    path: Option<PathBuf>,
    tree: Option<usvg::Tree>,

    // GPU cache
    handle: Option<TextureHandle>,
    raster_px: Size<u32>,

    draw_rect: Option<(f32, f32, f32, f32)>,
}

impl OnSweep for SvgState {
    fn on_sweep(&mut self, cx: &mut SweepCtx) {
        if let Some(handle) = self.handle.take() {
            cx.texture.unload(cx.gpu, handle);
        }
    }
}

impl SvgState {
    fn ensure_tree(
        &mut self,
        path: &Path,
        texture: &mut crate::render::texture::TextureRegistry,
        gpu: &crate::graphics::Gpu,
    ) {
        if self.path.as_deref() == Some(path) && self.tree.is_some() {
            return;
        }

        if let Some(handle) = self.handle.take() {
            texture.unload(gpu, handle);
        }

        self.path = Some(path.to_path_buf());
        self.tree = None;

        self.handle = None;
        self.raster_px = Size::new(0, 0);

        let Ok(bytes) = fs::read(path) else {
            return;
        };
        let Ok(text) = std::str::from_utf8(&bytes) else {
            return;
        };

        let mut opt = usvg::Options {
            resources_dir: path.parent().map(|p| p.to_path_buf()),
            ..Default::default()
        };
        opt.fontdb_mut().load_system_fonts();

        match usvg::Tree::from_str(text, &opt) {
            Ok(t) => self.tree = Some(t),
            Err(_) => self.tree = None,
        }
    }
}

fn fit_rect(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    svg_w: f32,
    svg_h: f32,
    fit: &ContentFit,
) -> (f32, f32, f32, f32) {
    if w <= 0 || h <= 0 {
        return (x as f32, y as f32, 0.0, 0.0);
    }

    match fit {
        ContentFit::Fill => (x as f32, y as f32, w as f32, h as f32),
        ContentFit::Contain | ContentFit::Cover => {
            // Maintain aspect
            let w_f = w as f32;
            let h_f = h as f32;

            let sx = w_f / svg_w;
            let sy = h_f / svg_h;

            let s = if matches!(fit, ContentFit::Contain) {
                sx.min(sy)
            } else {
                sx.max(sy)
            };

            let dw = (svg_w * s).max(1.0);
            let dh = (svg_h * s).max(1.0);

            let dx = x as f32 + (w as f32 - dw) / 2.0;
            let dy = y as f32 + (h as f32 - dh) / 2.0;
            (dx, dy, dw, dh)
        }
    }
}

impl IntoElement for Svg {}

impl<M> Widget<M> for Svg {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            clip_children: matches!(self.fit, ContentFit::Cover),
            ..Default::default()
        }
    }

    fn set_id(&mut self, id: Id) {
        self.id = id;
    }

    fn child_count(&self) -> usize {
        0
    }

    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        unreachable!()
    }

    fn prepare(&mut self, ctx: &mut PrepareCtx) {
        let r = ctx.rect();
        {
            let state = ctx.view_state.ensure_swept(self.id, SvgState::default);
            state.ensure_tree(&self.path, ctx.texture, ctx.gpu);
        }

        if r.w <= 0 || r.h <= 0 || self.tint.a() == 0 {
            return;
        }

        let Some(state) = ctx.view_state.get::<SvgState>(&self.id) else {
            return;
        };
        let Some(tree) = state.tree.as_ref() else {
            return;
        };

        let svg_size = tree.size();
        let (svg_w, svg_h) = (svg_size.width(), svg_size.height());
        let (dx, dy, dw, dh) = fit_rect(r.x, r.y, r.w, r.h, svg_w, svg_h, &self.fit);
        if dw <= 0.0 || dh <= 0.0 {
            return;
        }

        let raster = ctx.physical_size(Size::new(dw as u32, dh as u32));

        let state = ctx.view_state.get_mut::<SvgState>(&self.id).unwrap();
        if state.handle.is_some() && state.raster_px == raster {
            return;
        }

        let Some(mut pixmap) = tiny_skia::Pixmap::new(raster.width, raster.height) else {
            return;
        };
        let transform = tiny_skia::Transform::from_scale(
            raster.width as f32 / svg_w,
            raster.height as f32 / svg_h,
        );
        resvg::render(
            state.tree.as_ref().unwrap(),
            transform,
            &mut pixmap.as_mut(),
        );

        let pixels = pixmap.data();
        match state.handle {
            Some(handle) if handle.size_px == raster => {
                ctx.texture.update_rgba8(ctx.gpu, handle, pixels);
            }
            other => {
                if let Some(handle) = other {
                    ctx.texture.unload(ctx.gpu, handle);
                }
                state.handle =
                    Some(
                        ctx.texture
                            .load_rgba8(ctx.gpu, raster.width, raster.height, pixels),
                    );
            }
        }
        state.raster_px = raster;
        state.draw_rect = Some((dx, dy, dw, dh));
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        let Some(state) = ctx.view_state.get::<SvgState>(&self.id) else {
            return;
        };
        let Some(handle) = state.handle else {
            return;
        };
        let Some((dx, dy, dw, dh)) = state.draw_rect else {
            return;
        };

        out.push(Instance::ui_tex(
            Position::new(dx, dy),
            Size::new(dw, dh),
            self.tint,
            handle,
        ));
    }
}
