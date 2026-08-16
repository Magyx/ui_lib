use crate::{
    primitive::{Batch, InstanceData, Primitive},
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
    // Declares nothing: the UI is painter-ordered 2D. It still has to *match*
    // whatever the pass turns out to be, which `ctx.depth_state_passthrough()`
    // handles below.
    fn requirements() -> PassRequirements {
        PassRequirements::NONE
    }

    fn new(ctx: &PipelineCtx) -> Self {
        let mut pipeline = Self {
            render_pipeline: None,
            layout: None,
            geometry: QuadGeometry::new(&ctx.gpu.device),
        };
        pipeline.reload(ctx);
        pipeline
    }

    fn reload(&mut self, ctx: &PipelineCtx) {
        let source = format!(
            "enable wgpu_binding_array;\n{}",
            include_str!("../../../shaders/ui_shader.wgsl")
        );
        let shader_module = ctx
            .gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("UI Shader"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });

        let layout = ctx
            .gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("UI Render Pipeline Layout"),
                immediate_size: ctx.immediate_size,
                bind_group_layouts: &[Some(ctx.texture_bgl)],
            });
        self.layout = Some(layout);

        self.render_pipeline = Some(ctx.gpu.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("UI Render Pipeline"),
                layout: self.layout.as_ref(),
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
                primitive: wgpu::PrimitiveState::default(),
                // Never tests, never writes — but must declare the pass's
                // format when the pass has one.
                depth_stencil: ctx.depth_state_passthrough(),
                multisample: ctx.multisample_state(),
                multiview_mask: None,
                cache: None,
            },
        ));
    }

    fn bind(&mut self, ctx: &DrawCtx, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_bind_group(0, ctx.textures, &[]);
        pass.set_pipeline(self.render_pipeline.as_ref().unwrap());
        pass.set_immediates(0, bytemuck::bytes_of(ctx.globals));
        self.geometry.bind(pass);
    }

    fn draw(&mut self, ctx: &DrawCtx, pass: &mut wgpu::RenderPass<'_>, batch: &Batch) {
        self.geometry
            .draw(pass, ctx.instances, batch.byte_offset, batch.count);
    }
}
