use crate::{
    gpu::{Globals, Gpu},
    primitive::{Batch, InstanceStore},
    render::texture::TextureRegistry,
};

pub use pass::*;
pub use registry::*;
pub use slot::*;
pub use ui_macros::Pipeline;

pub mod mesh;
pub mod pass;
pub mod registry;
pub mod slot;
pub mod ui;

/// Everything needed to implement a custom [`Pipeline`].
///
/// ```ignore
/// use ui::render::pipeline::prelude::*;
///
/// #[derive(Pipeline)]
/// #[instance_data(MeshInstance)]
/// struct MeshPipeline { /* ... */ }
/// ```
pub mod prelude {
    pub use super::{
        DepthUse, DrawCtx, FrameCtx, PassConfig, PassRequirements, Pipeline, PipelineCtx,
    };
    pub use crate::{
        gpu::{Globals, Gpu},
        primitive::{Batch, Instance, InstanceData, Instanced, Primitive},
        render::{
            quad::{QUAD_INDICES, QUAD_VERTICES, QuadGeometry, Vertex},
            texture::TextureHandle,
        },
    };

    // Version-matched to the `ui` build; a custom pipeline cannot compile
    // against a different `wgpu`, and `InstanceData` impls need `bytemuck`.
    pub use crate::{bytemuck, wgpu};
}

/// Handed to [`Pipeline::new`] and [`Pipeline::reload`].
pub struct PipelineCtx<'a> {
    pub gpu: &'a Gpu,
    /// The resolved shared pass. Every pipeline in the registry sees the same
    /// value.
    pub pass: PassConfig,
    /// Bind group layout for the engine's bindless texture array. Bind it at
    /// group 0 if the pipeline samples engine textures; ignore it otherwise.
    pub texture_bgl: &'a wgpu::BindGroupLayout,
    /// Bytes of immediate data the engine reserves — currently exactly
    /// `size_of::<Globals>()`, laid out at offset 0.
    pub immediate_size: u32,
}
impl PipelineCtx<'_> {
    /// Depth state matching the pass, or `None` when the pass has no depth.
    ///
    /// `write` is silently downgraded to `false` when the pass attached depth
    /// read-only, which happens when no registered pipeline declared
    /// [`DepthUse::Write`].
    pub fn depth_state(
        &self,
        write: bool,
        compare: wgpu::CompareFunction,
    ) -> Option<wgpu::DepthStencilState> {
        self.pass
            .depth_format
            .map(|format| wgpu::DepthStencilState {
                format,
                depth_write_enabled: Some(write && !self.pass.depth_read_only),
                depth_compare: Some(compare),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
    }

    /// Depth state for a pipeline that does not care about depth but must
    /// still match a pass that has it — i.e. every 2D pipeline once anything
    /// in the app goes 3D.
    pub fn depth_state_passthrough(&self) -> Option<wgpu::DepthStencilState> {
        self.depth_state(false, wgpu::CompareFunction::Always)
    }

    pub fn multisample_state(&self) -> wgpu::MultisampleState {
        wgpu::MultisampleState {
            count: self.pass.sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        }
    }

    /// The surface colour target with the engine's premultiplied blend. Pass
    /// `Some(blend)` to override.
    pub fn color_target(&self, blend: Option<wgpu::BlendState>) -> wgpu::ColorTargetState {
        wgpu::ColorTargetState {
            format: self.pass.color_format,
            blend: Some(blend.unwrap_or(PREMULTIPLIED)),
            write_mask: wgpu::ColorWrites::ALL,
        }
    }

    /// Create a render pipeline, correcting the fields that must agree with
    /// the shared pass.
    ///
    /// Prefer this over calling `device.create_render_pipeline` directly. Pass
    /// state is not something a pipeline can know on its own: whether depth
    /// exists depends on what *other* pipelines the app registered, and that
    /// can change after this pipeline was first built. Hand-written values
    /// desync silently, and the resulting error arrives much later as an
    /// opaque `set_pipeline` failure.
    ///
    /// Corrections applied:
    ///
    /// * `depth_stencil: None` becomes [`Self::depth_state_passthrough`] — the
    ///   right answer for any pipeline that neither tests nor writes depth. In
    ///   a pass that has no depth it stays `None`.
    /// * `depth_stencil: Some(..)` keeps the pipeline's own write flag and
    ///   compare function, but takes its format from the pass.
    /// * `multisample.count` is taken from the pass; `mask` and
    ///   `alpha_to_coverage_enabled` are left alone.
    pub fn create_render_pipeline(
        &self,
        mut desc: wgpu::RenderPipelineDescriptor<'_>,
    ) -> wgpu::RenderPipeline {
        desc.depth_stencil = match (desc.depth_stencil.take(), self.pass.depth_format) {
            (None, _) => self.depth_state_passthrough(),
            (Some(state), Some(format)) => Some(wgpu::DepthStencilState {
                format,
                depth_write_enabled: Some(
                    state.depth_write_enabled.unwrap_or(false) && !self.pass.depth_read_only,
                ),
                ..state
            }),
            // The pipeline asked for depth in a pass that has none. That means
            // its `requirements()` did not declare `DepthUse`, so the pass was
            // never widened — a bug in the pipeline, not something to paper
            // over silently.
            (Some(_), None) => {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    label = ?desc.label,
                    "pipeline requested depth state but declared no DepthUse in requirements(); \
                     drawing without depth"
                );
                None
            }
        };

        desc.multisample = wgpu::MultisampleState {
            count: self.pass.sample_count,
            ..desc.multisample
        };

        self.gpu.device.create_render_pipeline(&desc)
    }
}

/// The blend state the engine composites with. Every pipeline drawing into the
/// shared pass must emit premultiplied colour.
pub const PREMULTIPLIED: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};

/// Handed to [`Pipeline::frame`], before the shared pass opens.
///
/// This is the only place a pipeline has both a queue and a command encoder,
/// so it is where uploads, offscreen passes, and compute belong.
pub struct FrameCtx<'a> {
    pub gpu: &'a Gpu,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub globals: &'a Globals,
    /// All of this frame's instances. Use [`FrameCtx::batches`] to get the
    /// ones belonging to this pipeline.
    pub store: &'a InstanceStore,
    /// Register offscreen output here to composite it back through
    /// `UiPipeline` as an ordinary textured primitive.
    pub textures: &'a mut TextureRegistry,
    pub pass: PassConfig,
    /// Physical size of the frame's colour attachment.
    pub target_size: [u32; 2],
    pub(crate) id: PipelineId,
}

impl FrameCtx<'_> {
    /// This pipeline's batches, in paint order.
    pub fn batches(&self) -> impl Iterator<Item = &Batch> {
        let id = self.id;
        self.store.batches().iter().filter(move |b| b.id == id)
    }

    /// Total instances this pipeline will draw this frame.
    pub fn instance_count(&self) -> u32 {
        self.batches().map(|b| b.count).sum()
    }
}

/// Handed to [`Pipeline::bind`] and [`Pipeline::draw`], inside the pass.
pub struct DrawCtx<'a> {
    pub globals: &'a Globals,
    /// Bindless texture array, matching `PipelineCtx::texture_bgl`.
    pub textures: &'a wgpu::BindGroup,
    /// The GPU-side instance buffer. Slice it at `batch.byte_offset`.
    pub instances: &'a wgpu::Buffer,
    /// CPU-side view of the same data. `store.view::<D>(batch)` reads a batch
    /// back as `&[D]` — the hook for splitting one batch into several draws.
    pub store: &'a InstanceStore,
    /// Read-only depth, when the pass attached it read-only. `None` whenever
    /// any pipeline writes depth: WebGPU forbids sampling an attachment that
    /// is also being written.
    pub depth: Option<&'a wgpu::BindGroup>,
    pub pass: PassConfig,
    pub target_size: [u32; 2],
}

pub trait Pipeline: PipelineSlot + 'static {
    /// What this pipeline needs from the shared pass. Queried once per type,
    /// before construction, and merged with every other registered pipeline.
    ///
    /// Must not depend on instance state — it is called on the type.
    fn requirements() -> PassRequirements
    where
        Self: Sized,
    {
        PassRequirements::NONE
    }

    fn new(ctx: &PipelineCtx) -> Self
    where
        Self: Sized;

    /// Rebuild everything that depends on [`PipelineCtx`] — shader modules,
    /// pipeline layouts, render pipelines.
    ///
    /// Called on surface reconfiguration *and* whenever the merged pass
    /// requirements widen. Resources that do not depend on the pass (vertex
    /// buffers, uploaded geometry, textures) must be built in
    /// [`Pipeline::new`] and left alone here, or they are silently discarded
    /// on the first resize.
    fn reload(&mut self, ctx: &PipelineCtx);

    /// Before the shared pass, once per frame, only when this pipeline has
    /// instances. Default: nothing.
    fn frame(&mut self, _ctx: &mut FrameCtx) {}

    /// Bind pipeline-wide state. Called once per contiguous run of batches
    /// belonging to this pipeline.
    fn bind(&mut self, ctx: &DrawCtx, pass: &mut wgpu::RenderPass<'_>);

    /// Draw one batch. The default issues a single instanced draw; override to
    /// sub-divide, e.g. one draw per mesh.
    fn draw(&mut self, ctx: &DrawCtx, pass: &mut wgpu::RenderPass<'_>, batch: &Batch);
}

#[cfg(test)]
macro_rules! impl_stub_pipeline {
    ($ty:ident) => {
        impl $crate::render::pipeline::Pipeline for $ty {
            fn new(_: &$crate::render::pipeline::PipelineCtx) -> Self {
                unreachable!("test stub is never built")
            }
            fn reload(&mut self, _: &$crate::render::pipeline::PipelineCtx) {}
            fn bind(
                &mut self,
                _: &$crate::render::pipeline::DrawCtx,
                _: &mut wgpu::RenderPass<'_>,
            ) {
            }
            fn draw(
                &mut self,
                _: &$crate::render::pipeline::DrawCtx,
                _: &mut wgpu::RenderPass<'_>,
                _: &$crate::primitive::Batch,
            ) {
            }
        }
    };
}
#[cfg(test)]
pub(crate) use impl_stub_pipeline;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Pipeline)]
    struct A;
    impl_stub_pipeline!(A);
    #[derive(Pipeline)]
    struct B;
    impl_stub_pipeline!(B);

    #[test]
    fn distinct_types_get_distinct_slots() {
        assert!(!core::ptr::eq(A::slot(), B::slot()));
    }

    #[test]
    fn id_is_stable_across_calls() {
        assert_eq!(PipelineId::of::<A>(), PipelineId::of::<A>());
        assert_ne!(PipelineId::of::<A>(), PipelineId::of::<B>());
    }

    #[test]
    fn pipelines_default_to_needing_nothing() {
        assert_eq!(A::requirements(), PassRequirements::NONE);
    }
}
