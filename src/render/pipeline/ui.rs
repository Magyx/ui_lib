use crate::{
    graphics::{Config, Globals},
    render::pipeline::Pipeline,
};
use wgpu::RenderPipeline;

pub(super) struct UiPipeline {
    render_pipeline: Option<RenderPipeline>,
    layout: Option<wgpu::PipelineLayout>,
}

impl Pipeline for UiPipeline {
    fn new(
        config: &Config,
        buffers: &[wgpu::VertexBufferLayout],
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) -> Self {
        let mut pipeline = Self {
            render_pipeline: None,
            layout: None,
        };
        pipeline.reload(config, buffers, texture_bgl, push_constant_ranges);

        pipeline
    }

    fn reload(
        &mut self,
        config: &Config,
        buffers: &[wgpu::VertexBufferLayout],
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) {
        let shader_module = config
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("UI Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../shaders/ui_shader.wgsl").into(),
                ),
            });

        let layout = config
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("UI Render Pipeline Layout"),
                push_constant_ranges,
                bind_group_layouts: &[texture_bgl],
            });
        self.layout = Some(layout);

        self.render_pipeline = Some(config.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("UI Render Pipeline"),
                layout: self.layout.as_ref(),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: Some("vs_main"),
                    buffers,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.config.format,
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

    fn apply_pipeline(
        &self,
        globals: &Globals,
        texture_bindgroup: &wgpu::BindGroup,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) {
        render_pass.set_bind_group(0, texture_bindgroup, &[]);
        render_pass.set_pipeline(self.render_pipeline.as_ref().unwrap());
        render_pass.set_push_constants(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::bytes_of(globals),
        );
    }
}
