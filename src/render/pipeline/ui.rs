use crate::{
    graphics::{Config, Globals},
    primitive::{Primitive, Vertex},
    render::pipeline::Pipeline,
};
use wgpu::{PushConstantRange, RenderPipeline};

pub(super) struct UiPipeline {
    render_pipeline: Option<RenderPipeline>,
}

impl Pipeline for UiPipeline {
    fn new(config: &Config, push_constant_ranges: &[PushConstantRange]) -> Self {
        let mut pipeline = Self {
            render_pipeline: None,
        };

        pipeline.reload(config, push_constant_ranges);

        pipeline
    }

    fn reload(&mut self, config: &Config, push_constant_ranges: &[PushConstantRange]) {
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
                bind_group_layouts: &[],
            });

        self.render_pipeline = Some(config.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("UI Render Pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc(), Primitive::desc()],
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
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            },
        ));
    }

    fn apply_pipeline(&self, globals: &Globals, render_pass: &mut wgpu::RenderPass<'_>) {
        if let Some(pipeline) = &self.render_pipeline {
            render_pass.set_pipeline(pipeline);
            render_pass.set_push_constants(
                wgpu::ShaderStages::VERTEX_FRAGMENT,
                0,
                bytemuck::bytes_of(globals),
            );
        } else {
            panic!("UI Render Pipeline not initialized!");
        }
    }
}
