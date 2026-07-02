pub mod pipeline;
pub(crate) mod renderer;
pub mod texture;

#[cfg(feature = "text_cosmic")]
pub(crate) mod glyph_atlas;
#[cfg(feature = "text_cosmic")]
pub mod text_cosmic;

pub(crate) mod alloc;
pub use alloc::AllocatorKind;

pub type PipelineFactoryFn = fn(
    &crate::graphics::Gpu,
    &wgpu::TextureFormat,
    &[wgpu::VertexBufferLayout],
    &wgpu::BindGroupLayout,
    &[wgpu::PushConstantRange],
) -> Box<dyn pipeline::Pipeline>;
