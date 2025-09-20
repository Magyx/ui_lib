pub mod pipeline;
pub(crate) mod renderer;

pub type PipelineFactoryFn =
    fn(&crate::graphics::Config, &[wgpu::PushConstantRange]) -> Box<dyn pipeline::Pipeline>;
