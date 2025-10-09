use crate::model::*;

pub(crate) mod consts;
pub mod context;
pub mod event;
pub mod graphics;
pub mod model;
pub mod primitive;
pub mod render;
#[cfg(feature = "sctk")]
pub mod sctk;
pub mod widget;
#[cfg(feature = "winit")]
pub mod winit;

#[macro_export]
macro_rules! pipeline_factories {
    ( $( $name:literal => $ty:path ),+ $(,)? ) => {{
        [
            $(
                ($name, {
                    fn __factory(
                        gpu: &$crate::graphics::Gpu,
                        surface_format: &wgpu::TextureFormat,
                        buffers: &[wgpu::VertexBufferLayout],
                        texture_bgl: &wgpu::BindGroupLayout,
                        ranges: &[wgpu::PushConstantRange],
                    ) -> ::std::boxed::Box<dyn $crate::render::pipeline::Pipeline> {
                        ::std::boxed::Box::new(<$ty>::new(gpu, surface_format, buffers, texture_bgl, ranges))
                    }
                    __factory as $crate::render::PipelineFactoryFn
                }),
            )+
        ]
    }};
}
