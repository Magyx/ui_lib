use super::*;
use crate::render::texture::TextureHandle;

use resvg::{tiny_skia, usvg};

use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Svg {
    x: i32,
    y: i32,
    w: i32,
    h: i32,

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
            x: 0,
            y: 0,
            w: 0,
            h: 0,
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
) -> (i32, i32, i32, i32) {
    if w <= 0 || h <= 0 {
        return (x, y, 0, 0);
    }

    match fit {
        ContentFit::Fill => (x, y, w, h),
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

            let dw = (svg_w * s).round().max(1.0) as i32;
            let dh = (svg_h * s).round().max(1.0) as i32;

            let dx = x + (w - dw) / 2;
            let dy = y + (h - dh) / 2;
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
        0
    }

    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
        panic!("Svg has no children")
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut Vec<Instance>) {
        // Grab / init view state
        let entry = ctx
            .view_state
            .entry(self.id)
            .or_insert_with(|| Box::new(SvgState::default()));

        let state = entry
            .downcast_mut::<SvgState>()
            .expect("SvgState type mismatch in view_state");

        // Nothing to draw
        if self.w <= 0 || self.h <= 0 || self.tint.a() == 0 {
            return;
        }

        // Ensure we have a parsed tree
        state.ensure_tree(&self.path, ctx.texture, ctx.gpu);
        let Some(tree) = state.tree.as_ref() else {
            return;
        };

        let svg_size = tree.size();
        let svg_w = svg_size.width();
        let svg_h = svg_size.height();

        // Compute draw rect based on ContentFit (like Image)
        let (dx, dy, dw, dh) = fit_rect(self.x, self.y, self.w, self.h, svg_w, svg_h, &self.fit);
        if dw <= 0 || dh <= 0 {
            return;
        }

        let raster = Size::new(dw as u32, dh as u32);

        // (Re)rasterize if needed
        let need_rerender = state.handle.is_none() || state.raster_px != raster;
        if need_rerender {
            let Some(mut pixmap) = tiny_skia::Pixmap::new(raster.width, raster.height) else {
                return;
            };

            // Scale tree into pixmap
            let sx = raster.width as f32 / svg_w;
            let sy = raster.height as f32 / svg_h;
            let transform = tiny_skia::Transform::from_scale(sx, sy);

            let mut pixmap_mut = pixmap.as_mut();
            // resvg::render outputs sRGB pixels
            resvg::render(tree, transform, &mut pixmap_mut);

            let pixels: &[u8] = pixmap.data();

            // Upload/update GPU texture
            match state.handle {
                Some(handle) if handle.size_px == raster => {
                    ctx.texture.update_rgba8(ctx.gpu, handle, pixels);
                    state.handle = Some(handle);
                }
                Some(handle) => {
                    ctx.texture.unload(ctx.gpu, handle);
                    let new_h =
                        ctx.texture
                            .load_rgba8(ctx.gpu, raster.width, raster.height, pixels);
                    state.handle = Some(new_h);
                }
                None => {
                    let new_h =
                        ctx.texture
                            .load_rgba8(ctx.gpu, raster.width, raster.height, pixels);
                    state.handle = Some(new_h);
                }
            }

            state.raster_px = raster;
        }

        // Emit draw instance
        if let Some(handle) = state.handle {
            out.push(Instance::ui_tex(
                Position::new(dx, dy),
                Size::new(dw, dh),
                self.tint,
                handle,
            ));
        }
    }
}
