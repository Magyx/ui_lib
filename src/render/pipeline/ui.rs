use crate::{
    gpu::Gpu,
    primitive::{InstanceData, Primitive},
    render::{
        pipeline::*,
        quad::{QuadGeometry, Vertex},
    },
};
use wgpu::RenderPipeline;

/// The built-in pipeline every stock widget draws through.
#[derive(Pipeline)]
pub struct UiPipeline {
    render_pipeline: Option<RenderPipeline>,
    layout: Option<wgpu::PipelineLayout>,
    geometry: QuadGeometry,
}

impl Pipeline for UiPipeline {
    fn new(
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) -> Self {
        let mut pipeline = Self {
            render_pipeline: None,
            layout: None,
            geometry: QuadGeometry::new(&gpu.device),
        };
        pipeline.reload(gpu, surface_format, texture_bgl, push_constant_ranges);

        pipeline
    }

    fn reload(
        &mut self,
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) {
        let shader_module = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("UI Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../shaders/ui_shader.wgsl").into(),
                ),
            });

        let layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("UI Render Pipeline Layout"),
                push_constant_ranges,
                bind_group_layouts: &[texture_bgl],
            });
        self.layout = Some(layout);

        self.render_pipeline = Some(gpu.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("UI Render Pipeline"),
                layout: self.layout.as_ref(),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::layout(), Primitive::layout()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: *surface_format,
                        blend: Some(wgpu::BlendState {
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
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            },
        ));
    }

    fn bind(&mut self, ctx: &DrawCtx, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_bind_group(0, ctx.textures, &[]);
        pass.set_pipeline(self.render_pipeline.as_ref().unwrap());
        pass.set_push_constants(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::bytes_of(ctx.globals),
        );
        self.geometry.bind(pass);
    }

    fn draw(
        &mut self,
        ctx: &DrawCtx,
        pass: &mut wgpu::RenderPass<'_>,
        byte_offset: u64,
        count: u32,
    ) {
        self.geometry.draw(pass, ctx.instances, byte_offset, count);
    }
}
