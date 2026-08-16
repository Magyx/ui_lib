use ui::render::pipeline::prelude::*;

#[derive(Pipeline)]
pub struct PlanetPipeline {
    render_pipeline: Option<wgpu::RenderPipeline>,
    geometry: QuadGeometry,
}

impl Pipeline for PlanetPipeline {
    fn new(ctx: &PipelineCtx) -> Self {
        let mut p = Self {
            render_pipeline: None,
            geometry: QuadGeometry::new(&ctx.gpu.device),
        };
        p.reload(ctx);
        p
    }

    fn reload(&mut self, ctx: &PipelineCtx) {
        let &PipelineCtx {
            gpu,
            immediate_size,
            ..
        } = ctx;
        let shader_module = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Planet Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/planet.wgsl").into()),
            });

        let layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Planet Layout"),
                bind_group_layouts: &[],
                immediate_size,
            });

        self.render_pipeline = Some(ctx.create_render_pipeline(wgpu::RenderPipelineDescriptor {
            label: Some("Planet Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::layout()), Some(Primitive::layout())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(ctx.color_target(None))],
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
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        }));
    }

    fn bind(&mut self, ctx: &DrawCtx, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(self.render_pipeline.as_ref().unwrap());
        pass.set_immediates(0, bytemuck::bytes_of(ctx.globals));
        self.geometry.bind(pass);
    }

    fn draw(&mut self, ctx: &DrawCtx, pass: &mut wgpu::RenderPass<'_>, batch: &Batch) {
        self.geometry
            .draw(pass, ctx.instances, batch.byte_offset, batch.count);
    }
}
