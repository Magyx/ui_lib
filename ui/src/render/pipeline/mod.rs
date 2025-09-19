use std::collections::HashMap;

use crate::graphics::{Config, Globals};

mod ui;

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum PipelineKey {
    Ui,
    Other(&'static str),
}

pub trait Pipeline {
    fn new(config: &Config, push_constant_ranges: &[wgpu::PushConstantRange]) -> Self
    where
        Self: Sized;

    fn reload(&mut self, config: &Config, push_constant_ranges: &[wgpu::PushConstantRange]);

    fn apply_pipeline(&self, globals: &Globals, render_pass: &mut wgpu::RenderPass<'_>);
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
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) {
        self.register_pipeline(
            PipelineKey::Ui,
            Box::new(ui::UiPipeline::new(config, push_constant_ranges)),
        );
    }

    pub fn register_pipeline(&mut self, key: PipelineKey, pipeline: Box<dyn Pipeline>) {
        self.pipelines.insert(key, pipeline);
    }

    pub(crate) fn reload(
        &mut self,
        config: &Config,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) {
        for pipeline in self.pipelines.values_mut() {
            pipeline.reload(config, push_constant_ranges);
        }
    }

    pub(crate) fn apply_pipeline(
        &self,
        key: &PipelineKey,
        globals: &Globals,
        pass: &mut wgpu::RenderPass<'_>,
    ) {
        self.pipelines
            .get(key)
            .expect("Pipeline not registered!")
            .as_ref()
            .apply_pipeline(globals, pass);
    }
}
