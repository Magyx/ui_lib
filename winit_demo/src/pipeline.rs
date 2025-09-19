use ui::graphics::{Config, Globals};
use ui::primitive::{Primitive, Vertex};
use ui::render::pipeline::Pipeline;

pub struct PlanetPipeline {
    render_pipeline: Option<wgpu::RenderPipeline>,
}

impl Pipeline for PlanetPipeline {
    fn new(config: &Config, push_constant_ranges: &[wgpu::PushConstantRange]) -> Self {
        let mut p = Self {
            render_pipeline: None,
        };
        p.reload(config, push_constant_ranges);
        p
    }

    fn reload(&mut self, config: &Config, push_constant_ranges: &[wgpu::PushConstantRange]) {
        let shader_module = config
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Planet Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/planet.wgsl").into()),
            });

        let layout = config
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Planet Layout"),
                bind_group_layouts: &[],
                push_constant_ranges,
            });

        self.render_pipeline = Some(config.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("Planet Render Pipeline"),
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
            panic!("Planet Render Pipeline not initialized!");
        }
    }
}
