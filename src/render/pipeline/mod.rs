use std::collections::HashMap;

use crate::graphics::{Globals, Gpu};

mod ui;

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum PipelineKey {
    Ui,
    Other(&'static str),
}

pub trait Pipeline {
    fn new(
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        buffers: &[wgpu::VertexBufferLayout],
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) -> Self
    where
        Self: Sized;

    fn reload(
        &mut self,
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        buffers: &[wgpu::VertexBufferLayout],
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    );

    fn apply_pipeline(
        &self,
        globals: &Globals,
        texture_bindgroup: &wgpu::BindGroup,
        render_pass: &mut wgpu::RenderPass<'_>,
    );
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
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        buffers: &[wgpu::VertexBufferLayout],
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) {
        self.register_pipeline(
            PipelineKey::Ui,
            Box::new(ui::UiPipeline::new(
                gpu,
                surface_format,
                buffers,
                texture_bgl,
                push_constant_ranges,
            )),
        );
    }

    pub(crate) fn has_default_pipelines(&self) -> bool {
        [PipelineKey::Ui]
            .iter()
            .all(|k| self.pipelines.contains_key(k))
    }

    pub fn register_pipeline(&mut self, key: PipelineKey, pipeline: Box<dyn Pipeline>) {
        self.pipelines.insert(key, pipeline);
    }

    pub(crate) fn reload(
        &mut self,
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        buffers: &[wgpu::VertexBufferLayout],
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) {
        for pipeline in self.pipelines.values_mut() {
            pipeline.reload(
                gpu,
                surface_format,
                buffers,
                texture_bgl,
                push_constant_ranges,
            );
        }
    }

    pub(crate) fn apply_pipeline(
        &self,
        key: &PipelineKey,
        globals: &Globals,
        texture_bindgroup: &wgpu::BindGroup,
        pass: &mut wgpu::RenderPass<'_>,
    ) {
        self.pipelines
            .get(key)
            .expect("Pipeline not registered!")
            .as_ref()
            .apply_pipeline(globals, texture_bindgroup, pass);
    }
}
