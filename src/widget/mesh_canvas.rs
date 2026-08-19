use std::sync::Arc;

use ui::{
    render::pipeline::mesh::{
        Mesh, MeshHandle, MeshInstance, MeshPipeline, fnv1a,
        math::{Camera, Mat4},
    },
    render::texture::TextureHandle,
    widget::prelude::*,
};

/// One thing to draw in the canvas.
pub struct MeshItem {
    key: u64,
    version: u64,
    build: Option<Box<dyn FnOnce() -> Mesh + Send>>,
    model: Mat4,
    tint: [f32; 4],
    texture: Option<TextureHandle>,
    handle: MeshHandle,
}
impl MeshItem {
    /// `key` identifies the mesh across frames; `version` its contents. `build`
    /// runs only when the version changes, so passing a closure that generates
    /// or loads geometry is cheap to call every frame.
    pub fn new(key: u64, version: u64, build: impl FnOnce() -> Mesh + Send + 'static) -> Self {
        Self {
            key,
            version,
            build: Some(Box::new(build)),
            model: Mat4::IDENTITY,
            tint: [1.0, 1.0, 1.0, 1.0],
            texture: None,
            handle: MeshHandle::INVALID,
        }
    }

    pub fn named(name: &str, version: u64, build: impl FnOnce() -> Mesh + Send + 'static) -> Self {
        Self::new(fnv1a(name), version, build)
    }

    /// Static geometry that never changes after first upload.
    pub fn shared(name: &str, mesh: Arc<Mesh>) -> Self {
        Self::named(name, 1, move || (*mesh).clone())
    }

    pub fn model(mut self, model: Mat4) -> Self {
        self.model = model;
        self
    }
    pub fn tint(mut self, tint: [f32; 4]) -> Self {
        self.tint = tint;
        self
    }
    pub fn texture(mut self, handle: TextureHandle) -> Self {
        self.texture = Some(handle);
        self
    }
}

// TODO: light sources
#[derive(Widget)]
pub struct MeshCanvas {
    size: Size<Length>,
    min: Size<i32>,
    max: Size<i32>,
    items: Vec<MeshItem>,
    camera: Camera,
    /// Radians per second of turntable spin applied to the camera. Non-zero
    /// requests a redraw every frame.
    spin: f32,
    orbit_distance: f32,
    pitch: f32,
    clip: bool,
}
impl MeshCanvas {
    pub fn new(size: Size<Length>) -> Self {
        Self {
            size,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            items: Vec::new(),
            camera: Camera::default(),
            spin: 0.0,
            orbit_distance: 3.0,
            pitch: 0.35,
            clip: true,
        }
    }

    pub fn push(mut self, item: MeshItem) -> Self {
        self.items.push(item);
        self
    }

    /// Convenience for the single-mesh case.
    pub fn mesh(
        self,
        name: &str,
        version: u64,
        build: impl FnOnce() -> Mesh + Send + 'static,
    ) -> Self {
        self.push(MeshItem::named(name, version, build))
    }

    pub fn camera(mut self, camera: Camera) -> Self {
        self.camera = camera;
        self
    }
    pub fn spin(mut self, rad_per_sec: f32) -> Self {
        self.spin = rad_per_sec;
        self
    }
    pub fn orbit(mut self, distance: f32, pitch: f32) -> Self {
        self.orbit_distance = distance;
        self.pitch = pitch;
        self
    }
    pub fn clip(mut self, enable: bool) -> Self {
        self.clip = enable;
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

    fn camera_at(&self, time: f32) -> Camera {
        if self.spin == 0.0 {
            self.camera
        } else {
            self.camera
                .orbit(time * self.spin, self.pitch, self.orbit_distance)
        }
    }
}
impl Widget for MeshCanvas {
    fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a>) -> Node {
        Node {
            size: self.size,
            min: self.min,
            max: self.max,
            clip_children: self.clip,
            ..Default::default()
        }
    }

    fn child_count(&self) -> usize {
        0
    }
    fn child_mut(&mut self, _i: usize) -> &mut dyn Widget {
        unreachable!()
    }

    fn prepare(&mut self, ctx: &mut PrepareCtx) {
        // Declaration is CPU-only; the pipeline's `frame` hook does the upload
        // once, after every widget has had its say.
        let pipeline = ctx.pipeline::<MeshPipeline>();

        for item in &mut self.items {
            let Some(build) = item.build.take() else {
                continue;
            };
            // Handles stay valid even if a later widget declares more meshes:
            // the arena assigns slot indices once per key.
            item.handle = pipeline.meshes.declare(item.key, item.version, build);
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, out: &mut InstanceStore) {
        let r = ctx.rect();
        if r.w <= 0 || r.h <= 0 || self.items.is_empty() {
            return;
        }

        let rect = [r.x as f32, r.y as f32, r.w as f32, r.h as f32];
        let camera = self.camera_at(ctx.globals.time);

        // Instances sharing a mesh are drawn in one call, so emitting in mesh
        // order costs nothing extra and collapses the draw count. Items are
        // usually few; a stable sort keeps the caller's ordering within a mesh.
        self.items.sort_by_key(|i| i.handle.slot());

        for item in &self.items {
            if !item.handle.is_valid() {
                continue;
            }
            let mut instance =
                MeshInstance::from_camera(item.handle, rect, &camera, item.model).tint(item.tint);
            if let Some(tex) = item.texture {
                instance = instance.texture(tex);
            }
            out.push(Instance::of::<MeshPipeline>(instance));
        }
    }

    fn handle(&mut self, ctx: &mut EventCtx) {
        if self.spin != 0.0 {
            ctx.ui.request_redraw();
        }
    }
}
