use std::{collections::HashMap, ops::Mul};

use ui::render::pipeline::prelude::*;

use math::Mat4;
pub mod math;

/* ------------------------------ geometry -------------------------------- */

/// The vertex format this pipeline understands.
///
/// A different format means a different pipeline type — the `Pipeline` derive
/// rejects generics on purpose, since a `static` slot inside a generic impl
/// would be shared across instantiations. Copying this file and changing the
/// struct is the intended way to get a second format.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl MeshVertex {
    pub fn new(position: [f32; 3], normal: [f32; 3]) -> Self {
        Self {
            position,
            normal,
            uv: [0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
    pub fn uv(mut self, uv: [f32; 2]) -> Self {
        self.uv = uv;
        self
    }
    pub fn color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 12,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 13,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Whatever you want to draw. Triangle list, CCW front faces.
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn new(vertices: Vec<MeshVertex>, indices: Vec<u32>) -> Self {
        debug_assert!(
            indices.len().is_multiple_of(3),
            "triangle list needs a multiple of 3 indices"
        );
        debug_assert!(
            indices.iter().all(|&i| (i as usize) < vertices.len()),
            "index out of range"
        );
        Self { vertices, indices }
    }

    /// Unindexed triangle soup with flat normals computed per face.
    pub fn from_triangles(positions: &[[f32; 3]]) -> Self {
        let mut vertices = Vec::with_capacity(positions.len());
        for tri in positions.chunks_exact(3) {
            let (a, b, c) = (tri[0], tri[1], tri[2]);
            let ab = math::sub(b, a);
            let ac = math::sub(c, a);
            let n = math::normalize(math::cross(ab, ac));
            for p in tri {
                vertices.push(MeshVertex::new(*p, n));
            }
        }
        let indices = (0..vertices.len() as u32).collect();
        Self { vertices, indices }
    }

    /// Recompute smooth normals by area-weighted face averaging. Useful after
    /// loading a mesh whose normals are missing.
    pub fn with_smooth_normals(mut self) -> Self {
        use math::{cross, normalize, sub};

        for v in &mut self.vertices {
            v.normal = [0.0; 3];
        }
        for tri in self.indices.chunks_exact(3) {
            let [i, j, k] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
            let n = cross(
                sub(self.vertices[j].position, self.vertices[i].position),
                sub(self.vertices[k].position, self.vertices[i].position),
            );
            for idx in [i, j, k] {
                let dst = &mut self.vertices[idx].normal;
                dst[0] += n[0];
                dst[1] += n[1];
                dst[2] += n[2];
            }
        }
        for v in &mut self.vertices {
            v.normal = normalize(v.normal);
        }
        self
    }

    pub fn tinted(mut self, color: [f32; 4]) -> Self {
        for v in &mut self.vertices {
            v.color = color;
        }
        self
    }

    /// Unit cube. Convex, so it also looks correct on a build with no depth.
    pub fn cube() -> Self {
        const FACES: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
            ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
            ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
            ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
            ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ];

        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);
        for (n, u, v) in FACES {
            let base = vertices.len() as u32;
            for (su, sv, uv) in [
                (-1.0, -1.0, [0.0, 1.0]),
                (1.0, -1.0, [1.0, 1.0]),
                (1.0, 1.0, [1.0, 0.0]),
                (-1.0, 1.0, [0.0, 0.0]),
            ] {
                vertices.push(
                    MeshVertex::new(
                        [
                            (n[0] + u[0] * su + v[0] * sv) * 0.5,
                            (n[1] + u[1] * su + v[1] * sv) * 0.5,
                            (n[2] + u[2] * su + v[2] * sv) * 0.5,
                        ],
                        n,
                    )
                    .uv(uv),
                );
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        Self { vertices, indices }
    }

    /// Non-convex, so it is the honest test of whether depth is actually
    /// working: without a depth buffer the far half of the ring paints over
    /// the near half.
    pub fn torus(major: f32, minor: f32, major_segs: u32, minor_segs: u32) -> Self {
        use std::f32::consts::TAU;

        let major_segs = major_segs.max(3);
        let minor_segs = minor_segs.max(3);
        let mut vertices = Vec::with_capacity((major_segs * minor_segs) as usize);
        let mut indices = Vec::with_capacity((major_segs * minor_segs * 6) as usize);

        for i in 0..major_segs {
            let phi = i as f32 / major_segs as f32 * TAU;
            let (sp, cp) = phi.sin_cos();
            for j in 0..minor_segs {
                let theta = j as f32 / minor_segs as f32 * TAU;
                let (st, ct) = theta.sin_cos();

                vertices.push(
                    MeshVertex::new(
                        [
                            cp * (major + minor * ct),
                            minor * st,
                            sp * (major + minor * ct),
                        ],
                        [cp * ct, st, sp * ct],
                    )
                    .uv([i as f32 / major_segs as f32, j as f32 / minor_segs as f32]),
                );

                let a = i * minor_segs + j;
                let b = ((i + 1) % major_segs) * minor_segs + j;
                let c = ((i + 1) % major_segs) * minor_segs + (j + 1) % minor_segs;
                let d = i * minor_segs + (j + 1) % minor_segs;
                indices.extend_from_slice(&[a, c, b, a, d, c]);
            }
        }
        Self { vertices, indices }
    }

    pub fn uv_sphere(radius: f32, rings: u32, sectors: u32) -> Self {
        use std::f32::consts::{PI, TAU};

        let rings = rings.max(2);
        let sectors = sectors.max(3);
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for r in 0..=rings {
            let v = r as f32 / rings as f32;
            let phi = v * PI;
            let (sp, cp) = phi.sin_cos();
            for s in 0..=sectors {
                let u = s as f32 / sectors as f32;
                let theta = u * TAU;
                let (st, ct) = theta.sin_cos();
                let n = [sp * ct, cp, sp * st];
                vertices.push(
                    MeshVertex::new([n[0] * radius, n[1] * radius, n[2] * radius], n).uv([u, v]),
                );
            }
        }

        let stride = sectors + 1;
        for r in 0..rings {
            for s in 0..sectors {
                let a = r * stride + s;
                let b = a + stride;
                indices.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
            }
        }
        Self { vertices, indices }
    }
}

/* ------------------------------ mesh arena ------------------------------ */

/// Stable reference to an uploaded mesh. Survives arena rebuilds — the slot
/// index is assigned once per key and never reassigned, so a handle taken in
/// one widget's `prepare` is still valid after a later widget uploads.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct MeshHandle(u32);

impl MeshHandle {
    pub const INVALID: Self = Self(u32::MAX);
    pub fn is_valid(self) -> bool {
        self != Self::INVALID
    }
    /// Arena slot. Sorting instances by this groups them into single draws.
    pub fn slot(self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Default, Debug)]
struct Span {
    index_start: u32,
    index_count: u32,
    base_vertex: i32,
}

struct Stored {
    version: u64,
    mesh: Mesh,
    span: Span,
    last_seen: u64,
}

/// Meshes are declared during `prepare` (CPU only) and uploaded once during
/// `frame`. Re-uploading rebuilds the whole arena rather than appending, so
/// changing a mesh cannot leak arena space.
pub struct MeshArena {
    keys: HashMap<u64, u32>,
    entries: Vec<Option<Stored>>,
    vertices: Option<wgpu::Buffer>,
    indices: Option<wgpu::Buffer>,
    vertex_capacity: u64,
    index_capacity: u64,
    dirty: bool,
    frame: u64,
    /// Frames a mesh may go untouched before it is dropped from the arena.
    pub retire_after: u64,
}

impl MeshArena {
    fn new() -> Self {
        Self {
            keys: HashMap::new(),
            entries: Vec::new(),
            vertices: None,
            indices: None,
            vertex_capacity: 0,
            index_capacity: 0,
            dirty: false,
            frame: 0,
            retire_after: 240,
        }
    }

    /// Declare a mesh. `build` runs only when `version` differs from the last
    /// upload for `key`, so calling this every frame is free.
    ///
    /// `key` identifies the mesh (hash a name, use an entity id, anything
    /// stable); `version` identifies its contents.
    pub fn declare(&mut self, key: u64, version: u64, build: impl FnOnce() -> Mesh) -> MeshHandle {
        let next = self.entries.len() as u32;
        let slot = *self.keys.entry(key).or_insert(next);
        if slot as usize >= self.entries.len() {
            self.entries.resize_with(slot as usize + 1, || None);
        }

        let needs_upload = match &self.entries[slot as usize] {
            Some(stored) => stored.version != version,
            None => true,
        };

        if needs_upload {
            let mesh = build();
            self.entries[slot as usize] = Some(Stored {
                version,
                mesh,
                span: Span::default(),
                last_seen: self.frame,
            });
            self.dirty = true;
        } else if let Some(stored) = &mut self.entries[slot as usize] {
            stored.last_seen = self.frame;
        }

        MeshHandle(slot)
    }

    /// Convenience for static geometry named by a string literal.
    pub fn declare_named(
        &mut self,
        name: &str,
        version: u64,
        build: impl FnOnce() -> Mesh,
    ) -> MeshHandle {
        self.declare(fnv1a(name), version, build)
    }

    pub fn drop_mesh(&mut self, key: u64) {
        if let Some(slot) = self.keys.remove(&key)
            && let Some(entry) = self.entries.get_mut(slot as usize)
        {
            *entry = None;
            self.dirty = true;
        }
    }

    fn span(&self, handle: MeshHandle) -> Option<Span> {
        let stored = self.entries.get(handle.0 as usize)?.as_ref()?;
        (stored.span.index_count > 0).then_some(stored.span)
    }

    /// Upload everything that changed. Called from `Pipeline::frame`.
    fn flush(&mut self, gpu: &Gpu) {
        self.frame = self.frame.wrapping_add(1);
        self.retire();

        if !self.dirty {
            return;
        }
        self.dirty = false;

        let vertex_bytes: u64 = self
            .entries
            .iter()
            .flatten()
            .map(|s| (s.mesh.vertices.len() * std::mem::size_of::<MeshVertex>()) as u64)
            .sum();
        let index_bytes: u64 = self
            .entries
            .iter()
            .flatten()
            .map(|s| (s.mesh.indices.len() * 4) as u64)
            .sum();

        if vertex_bytes == 0 || index_bytes == 0 {
            for stored in self.entries.iter_mut().flatten() {
                stored.span = Span::default();
            }
            return;
        }

        if vertex_bytes > self.vertex_capacity || self.vertices.is_none() {
            self.vertex_capacity = vertex_bytes.next_power_of_two();
            self.vertices = Some(alloc(
                &gpu.device,
                "Mesh Vertex Arena",
                wgpu::BufferUsages::VERTEX,
                self.vertex_capacity,
            ));
        }
        if index_bytes > self.index_capacity || self.indices.is_none() {
            self.index_capacity = index_bytes.next_power_of_two();
            self.indices = Some(alloc(
                &gpu.device,
                "Mesh Index Arena",
                wgpu::BufferUsages::INDEX,
                self.index_capacity,
            ));
        }

        let vbuf = self.vertices.as_ref().expect("just allocated");
        let ibuf = self.indices.as_ref().expect("just allocated");

        let mut vertex_offset = 0u64;
        let mut index_offset = 0u64;
        for stored in self.entries.iter_mut().flatten() {
            let v: &[u8] = bytemuck::cast_slice(&stored.mesh.vertices);
            let i: &[u8] = bytemuck::cast_slice(&stored.mesh.indices);

            gpu.queue.write_buffer(vbuf, vertex_offset, v);
            gpu.queue.write_buffer(ibuf, index_offset, i);

            stored.span = Span {
                index_start: (index_offset / 4) as u32,
                index_count: stored.mesh.indices.len() as u32,
                // `base_vertex` is added to every index at draw time, which is
                // what lets one index buffer address many meshes.
                base_vertex: (vertex_offset / std::mem::size_of::<MeshVertex>() as u64) as i32,
            };

            vertex_offset += v.len() as u64;
            index_offset += i.len() as u64;
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(vertex_bytes, index_bytes, "mesh arena uploaded");
    }

    fn retire(&mut self) {
        let cutoff = self.retire_after;
        let now = self.frame;
        let mut removed = Vec::new();

        for (slot, entry) in self.entries.iter_mut().enumerate() {
            if let Some(stored) = entry
                && now.saturating_sub(stored.last_seen) > cutoff
            {
                *entry = None;
                removed.push(slot as u32);
                self.dirty = true;
            }
        }
        if !removed.is_empty() {
            self.keys.retain(|_, slot| !removed.contains(slot));
        }
    }
}

fn alloc(device: &wgpu::Device, label: &str, usage: wgpu::BufferUsages, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(4),
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub const fn fnv1a(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
        i += 1;
    }
    hash
}

/* ------------------------------- instance ------------------------------- */

/// One mesh draw.
///
/// Carries the complete clip-space transform rather than a reference to a
/// shared camera, which is what lets two canvases in the same frame use
/// different cameras with no extra bind groups.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshInstance {
    /// `projection * view * model`, column-major.
    pub mvp: [[f32; 4]; 4],
    /// Inverse-transpose of the model's upper-left 3x3, padded to `vec4`s.
    pub normal_matrix: [[f32; 4]; 3],
    /// Widget rect in logical pixels: `[x, y, w, h]`. The vertex stage remaps
    /// the mesh's own NDC cube into this sub-rect of the window.
    pub rect: [f32; 4],
    /// Linear RGBA, multiplied with the per-vertex colour.
    pub tint: [f32; 4],
    /// `[mesh slot, texture slot_gen, uv scale, uv offset]`. Slot 0 of this
    /// array is read on the CPU in `draw` to group instances by mesh; the
    /// shader ignores it.
    pub params: [u32; 4],
}

impl MeshInstance {
    pub fn new(mesh: MeshHandle, rect: [f32; 4], mvp: Mat4, normal_matrix: [[f32; 4]; 3]) -> Self {
        Self {
            mvp: mvp.0,
            normal_matrix,
            rect,
            tint: [1.0, 1.0, 1.0, 1.0],
            params: [mesh.0, 0, 0, 0],
        }
    }

    /// The common case: a camera, a model transform, and a widget rect.
    pub fn from_camera(
        mesh: MeshHandle,
        rect: [f32; 4],
        camera: &math::Camera,
        model: Mat4,
    ) -> Self {
        let aspect = if rect[3] > 0.0 {
            rect[2] / rect[3]
        } else {
            1.0
        };
        let mvp = camera.view_projection(aspect).mul(model);
        Self::new(mesh, rect, mvp, model.normal_matrix())
    }

    pub fn tint(mut self, tint: [f32; 4]) -> Self {
        self.tint = tint;
        self
    }

    /// Sample an engine texture across the mesh's UVs. Uses the same packed
    /// handle encoding as `Instance::ui_tex`.
    pub fn texture(mut self, handle: TextureHandle) -> Self {
        self.params[1] = handle.slot_gen;
        self.params[2] = handle.scale_packed;
        self.params[3] = handle.offset_packed;
        self
    }

    fn mesh_slot(&self) -> u32 {
        self.params[0]
    }
}

impl InstanceData for MeshInstance {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // mvp columns
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // normal matrix columns
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 80,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 96,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // rect / tint / params
                wgpu::VertexAttribute {
                    offset: 112,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 128,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 144,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Uint32x4,
                },
            ],
        }
    }
}

/* ------------------------------- pipeline ------------------------------- */

#[derive(Pipeline)]
#[instance_data(MeshInstance)]
pub struct MeshPipeline {
    render_pipeline: Option<wgpu::RenderPipeline>,
    pub meshes: MeshArena,
}

impl Pipeline for MeshPipeline {
    /// Depth, isolated per batch. That second flag is what stops two mesh
    /// canvases in the same window from occluding each other while still
    /// giving each one correct internal occlusion.
    fn requirements() -> PassRequirements {
        PassRequirements::DEPTH
    }

    fn new(ctx: &PipelineCtx) -> Self {
        // Nothing that survives a resize is built in `reload`.
        let mut p = Self {
            render_pipeline: None,
            meshes: MeshArena::new(),
        };
        p.reload(ctx);
        p
    }

    fn reload(&mut self, ctx: &PipelineCtx) {
        let source = format!(
            "enable wgpu_binding_array;\n{}",
            include_str!("../../../../shaders/mesh.wgsl")
        );
        let shader_module = ctx
            .gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Mesh Shader"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });

        let layout = ctx
            .gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Mesh Layout"),
                // Group 0 is the engine's bindless texture array, so meshes can
                // sample anything loaded through `TextureRegistry`.
                bind_group_layouts: &[Some(ctx.texture_bgl)],
                immediate_size: ctx.immediate_size,
            });

        self.render_pipeline = Some(ctx.gpu.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("Mesh Render Pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: Some("vs_main"),
                    // Slot 0 = arena geometry, slot 1 = engine instance buffer.
                    buffers: &[Some(MeshVertex::layout()), Some(MeshInstance::layout())],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: Some("fs_main"),
                    targets: &[Some(ctx.color_target(None))],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                // Resolved by the engine. Returns None on a build where no
                // pipeline asked for depth, which keeps this compiling and
                // drawing (convex meshes only) rather than failing validation.
                depth_stencil: ctx.depth_state(true, wgpu::CompareFunction::Less),
                multisample: ctx.multisample_state(),
                multiview_mask: None,
                cache: None,
            },
        ));
    }

    /// The only place with a queue *and* an encoder. Geometry declared by
    /// widgets during `prepare` is uploaded here, once, before the pass.
    fn frame(&mut self, ctx: &mut FrameCtx) {
        self.meshes.flush(ctx.gpu);
    }

    fn bind(&mut self, ctx: &DrawCtx, pass: &mut wgpu::RenderPass<'_>) {
        let (Some(pipeline), Some(vertices), Some(indices)) = (
            self.render_pipeline.as_ref(),
            self.meshes.vertices.as_ref(),
            self.meshes.indices.as_ref(),
        ) else {
            return;
        };

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, ctx.textures, &[]);
        pass.set_immediates(0, bytemuck::bytes_of(ctx.globals));
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
    }

    /// One batch can reference many meshes, so read the instances back and
    /// emit one indexed draw per run of identical mesh slots.
    ///
    /// Instances that share a mesh should be pushed consecutively — a run of
    /// `n` becomes one draw call, an alternating sequence becomes `n`.
    fn draw(&mut self, ctx: &DrawCtx, pass: &mut wgpu::RenderPass<'_>, batch: &Batch) {
        if self.render_pipeline.is_none() || self.meshes.vertices.is_none() {
            return;
        }

        let instances = ctx.store.view::<MeshInstance>(batch);
        let stride = std::mem::size_of::<MeshInstance>() as u64;

        let mut start = 0usize;
        while start < instances.len() {
            let slot = instances[start].mesh_slot();
            let mut end = start + 1;
            while end < instances.len() && instances[end].mesh_slot() == slot {
                end += 1;
            }

            if let Some(span) = self.meshes.span(MeshHandle(slot)) {
                // Slicing the instance buffer makes this run's first instance
                // index 0, so the range is always 0..len.
                let offset = batch.byte_offset + start as u64 * stride;
                pass.set_vertex_buffer(1, ctx.instances.slice(offset..));
                pass.draw_indexed(
                    span.index_start..span.index_start + span.index_count,
                    span.base_vertex,
                    0..(end - start) as u32,
                );
            }

            start = end;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_stride_matches_the_declared_layout() {
        assert_eq!(std::mem::size_of::<MeshInstance>(), 160);
        assert_eq!(MeshInstance::layout().array_stride, 160);
        assert_eq!(std::mem::size_of::<MeshVertex>(), 48);
        assert_eq!(MeshVertex::layout().array_stride, 48);
    }

    #[test]
    fn attribute_offsets_do_not_overlap() {
        let layout = MeshInstance::layout();
        let mut end = 0u64;
        for a in layout.attributes {
            assert!(a.offset >= end, "attribute {a:?} overlaps the previous one");
            assert_eq!(
                a.offset % 4,
                0,
                "vertex attribute offsets must be 4-aligned"
            );
            end = a.offset + a.format.size();
        }
        assert!(end <= layout.array_stride);
    }

    /// Handles must survive another widget uploading in the same frame, or a
    /// handle taken in `prepare` is stale by `paint`.
    #[test]
    fn handles_are_stable_across_later_declarations() {
        let mut arena = MeshArena::new();
        let a = arena.declare_named("cube", 1, Mesh::cube);
        let b = arena.declare_named("sphere", 1, || Mesh::uv_sphere(1.0, 8, 12));
        let a_again = arena.declare_named("cube", 1, Mesh::cube);
        assert_eq!(a, a_again);
        assert_ne!(a, b);
    }

    #[test]
    fn declare_rebuilds_only_on_version_change() {
        let mut arena = MeshArena::new();
        arena.declare_named("cube", 1, Mesh::cube);
        arena.dirty = false;

        arena.declare_named("cube", 1, || panic!("must not rebuild at the same version"));
        assert!(!arena.dirty);

        arena.declare_named("cube", 2, Mesh::cube);
        assert!(arena.dirty);
    }

    #[test]
    fn cube_is_closed_and_wound_consistently() {
        let cube = Mesh::cube();
        assert_eq!(cube.indices.len(), 36);
        // Every face normal points away from the centre.
        for v in &cube.vertices {
            let outward = math::dot(v.normal, v.position);
            assert!(outward > 0.0, "normal points inward at {:?}", v.position);
        }
    }

    #[test]
    fn smooth_normals_are_unit_length() {
        let m = Mesh::uv_sphere(1.0, 6, 8).with_smooth_normals();
        for v in &m.vertices {
            let len = math::dot(v.normal, v.normal).sqrt();
            assert!((len - 1.0).abs() < 1e-3 || len == 0.0);
        }
    }
}
