use crate::graphics::{Globals, Gpu};

pub mod ui;
pub use ui_macros::Pipeline;

pub mod slot;
pub use slot::*;

pub mod registry;
pub use registry::*;

/// Shared per-frame resources a pipeline may bind.
pub struct DrawCtx<'a> {
    pub globals: &'a Globals,
    pub textures: &'a wgpu::BindGroup,
    pub instances: &'a wgpu::Buffer,
}

pub trait Pipeline: PipelineSlot + 'static {
    fn new(
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) -> Self
    where
        Self: Sized;

    fn reload(
        &mut self,
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    );

    fn bind(&mut self, ctx: &DrawCtx, pass: &mut wgpu::RenderPass<'_>);

    fn draw(
        &mut self,
        ctx: &DrawCtx,
        pass: &mut wgpu::RenderPass<'_>,
        byte_offset: u64,
        count: u32,
    );
}

#[cfg(test)]
macro_rules! impl_stub_pipeline {
    ($ty:ident) => {
        impl $crate::render::pipeline::Pipeline for $ty {
            fn new(
                _: &$crate::graphics::Gpu,
                _: &wgpu::TextureFormat,
                _: &wgpu::BindGroupLayout,
                _: &[wgpu::PushConstantRange],
            ) -> Self {
                unreachable!("test stub is never built")
            }
            fn reload(
                &mut self,
                _: &$crate::graphics::Gpu,
                _: &wgpu::TextureFormat,
                _: &wgpu::BindGroupLayout,
                _: &[wgpu::PushConstantRange],
            ) {
            }
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
                _: u64,
                _: u32,
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
}
