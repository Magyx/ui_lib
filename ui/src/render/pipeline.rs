use crate::{
    graphics::Config,
    primitive::{Primitive, Vertex},
};
use std::collections::HashMap;
use wgpu::{PushConstantRange, RenderPipeline};

#[derive(Eq, Hash, PartialEq)]
pub enum PipelineKey {
    Ui,
    Other(&'static str),
}

pub trait Pipeline {
    fn new(config: &Config, push_constant_ranges: &[PushConstantRange]) -> Self
    where
        Self: Sized;

    fn reload(&mut self, config: &Config, push_constant_ranges: &[PushConstantRange]);

    fn apply_pipeline(&self, render_pass: &mut wgpu::RenderPass<'_>);
}

pub(crate) struct PipelineRegistry {
    pipelines: HashMap<PipelineKey, Box<dyn Pipeline>>,
}

impl PipelineRegistry {
    pub(crate) fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
        }
    }

    pub(crate) fn register_default_pipelines(
        &mut self,
        config: &Config,
        push_constant_ranges: &[PushConstantRange],
    ) {
        self.register_pipeline(
            PipelineKey::Ui,
            Box::new(UiPipeline::new(config, push_constant_ranges)),
        );
    }

    pub fn register_pipeline(&mut self, key: PipelineKey, pipeline: Box<dyn Pipeline>) {
        self.pipelines.insert(key, pipeline);
    }

    pub(crate) fn reload(&mut self, config: &Config, push_constant_ranges: &[PushConstantRange]) {
        for pipeline in self.pipelines.values_mut() {
            pipeline.reload(config, push_constant_ranges);
        }
    }

    pub(crate) fn apply_pipeline(&self, key: &PipelineKey, pass: &mut wgpu::RenderPass<'_>) {
        self.pipelines
            .get(key)
            .expect("Pipeline not registered!")
            .as_ref()
            .apply_pipeline(pass);
    }
}

struct UiPipeline {
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
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ui_shader.wgsl").into()),
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

    fn apply_pipeline(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        if let Some(pipeline) = &self.render_pipeline {
            render_pass.set_pipeline(pipeline);
        } else {
            panic!("UI Render Pipeline not initialized!");
        }
    }
}
