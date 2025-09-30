pub mod pipeline;
pub(crate) mod renderer;
pub mod text;
pub mod texture;

pub type PipelineFactoryFn = fn(
    &crate::graphics::Config,
    &[wgpu::VertexBufferLayout],
    &wgpu::BindGroupLayout,
    &[wgpu::PushConstantRange],
) -> Box<dyn pipeline::Pipeline>;
