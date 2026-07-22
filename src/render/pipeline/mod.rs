use crate::graphics::{Globals, Gpu};

mod ui;

#[derive(Eq, Copy, Clone, Hash, PartialEq, Debug)]
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
        &mut self,
        globals: &Globals,
        texture_bindgroup: &wgpu::BindGroup,
        render_pass: &mut wgpu::RenderPass<'_>,
    );
}

pub(crate) struct RegisteredPipeline {
    key: PipelineKey,
    pipeline: Box<dyn Pipeline>,
}
impl AsMut<Box<dyn Pipeline>> for RegisteredPipeline {
    fn as_mut(&mut self) -> &mut Box<dyn Pipeline> {
        &mut self.pipeline
    }
}
impl AsRef<Box<dyn Pipeline>> for RegisteredPipeline {
    fn as_ref(&self) -> &Box<dyn Pipeline> {
        &self.pipeline
    }
}

const CACHE_SIZE: usize = 64;

#[derive(Copy, Clone, Default)]
struct CacheEntry {
    ptr: usize,
    pipeline_id: u16,
}

pub(crate) struct PipelineRegistry {
    pipelines: Vec<RegisteredPipeline>,
    cache: [CacheEntry; CACHE_SIZE],
}

impl PipelineRegistry {
    pub(crate) fn new() -> Self {
        Self {
            pipelines: Vec::new(),
            cache: [Default::default(); CACHE_SIZE],
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
        !self.pipelines.is_empty()
    }

    pub fn register_pipeline(&mut self, key: PipelineKey, pipeline: Box<dyn Pipeline>) {
        let pipeline_id = self.pipelines.len() as u16;
        self.pipelines.push(RegisteredPipeline { key, pipeline });

        if let PipelineKey::Other(name) = key {
            let ptr = name.as_ptr() as usize;
            let slot = (ptr >> 3) & (CACHE_SIZE - 1);
            self.cache[slot] = CacheEntry { ptr, pipeline_id };
        }
    }

    pub(crate) fn reload(
        &mut self,
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        buffers: &[wgpu::VertexBufferLayout],
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) {
        for pipeline in &mut self.pipelines {
            pipeline.as_mut().reload(
                gpu,
                surface_format,
                buffers,
                texture_bgl,
                push_constant_ranges,
            );
        }
    }

    #[inline(always)]
    pub(crate) fn get_id(&mut self, key: &PipelineKey) -> u16 {
        match key {
            PipelineKey::Ui => 0,
            PipelineKey::Other(name) => {
                let ptr = name.as_ptr() as usize;
                let slot = (ptr >> 3) & (CACHE_SIZE - 1);
                let entry = self.cache[slot];

                if entry.ptr == ptr {
                    return entry.pipeline_id;
                }

                self.resolve_cold(ptr, name, slot)
            }
        }
    }

    #[cold]
    #[inline(never)]
    fn resolve_cold(&mut self, ptr: usize, name: &'static str, slot: usize) -> u16 {
        let (id, _) = self
            .pipelines
            .iter()
            .enumerate()
            .find(|(_, reg)| matches!(reg.key, PipelineKey::Other(n) if n == name))
            .expect("Pipeline not registered!");

        let pipeline_id = id as u16;
        self.cache[slot] = CacheEntry { ptr, pipeline_id };
        pipeline_id
    }

    pub(crate) fn apply_pipeline(
        &mut self,
        id: u16,
        globals: &Globals,
        texture_bindgroup: &wgpu::BindGroup,
        pass: &mut wgpu::RenderPass<'_>,
    ) {
        self.pipelines[id as usize]
            .as_mut()
            .apply_pipeline(globals, texture_bindgroup, pass);
    }
}
