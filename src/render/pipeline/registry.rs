use super::{Pipeline, PipelineId, ui};
use crate::gpu::Gpu;

#[derive(Copy, Clone)]
pub struct PipelineRegistration(
    fn(
        &mut PipelineRegistry,
        PipelineId,
        &Gpu,
        &wgpu::TextureFormat,
        &wgpu::BindGroupLayout,
        &[wgpu::PushConstantRange],
    ),
    pub PipelineId,
);
impl PipelineRegistration {
    pub fn of<P: Pipeline>() -> Self {
        let id = PipelineId::of::<P>();
        Self(
            |registry, id, gpu, surface_format, texture_bgl, ranges| {
                registry.insert(
                    id,
                    Box::new(P::new(gpu, surface_format, texture_bgl, ranges)),
                );
            },
            id,
        )
    }
}

pub(crate) struct PipelineRegistry {
    slots: Vec<Option<Box<dyn Pipeline>>>,
}
impl PipelineRegistry {
    pub(crate) fn new() -> Self {
        Self { slots: Vec::new() }
    }

    pub(crate) fn register_default_pipelines(
        &mut self,
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) {
        self.insert(
            PipelineId::of::<ui::UiPipeline>(),
            Box::new(ui::UiPipeline::new(
                gpu,
                surface_format,
                texture_bgl,
                push_constant_ranges,
            )),
        );
    }

    pub(crate) fn has_default_pipelines(&self) -> bool {
        self.get(PipelineId::of::<ui::UiPipeline>()).is_some()
    }

    pub(crate) fn register(
        &mut self,
        reg: PipelineRegistration,
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) {
        (reg.0)(
            self,
            reg.1,
            gpu,
            surface_format,
            texture_bgl,
            push_constant_ranges,
        );
    }

    fn insert(&mut self, id: PipelineId, pipeline: Box<dyn Pipeline>) {
        let idx = id.index();
        if self.slots.len() <= idx {
            self.slots.resize_with(idx + 1, || None);
        }
        if let Some(existing) = self.slots[idx].as_deref_mut() {
            assert_eq!(
                existing.as_any().type_id(),
                pipeline.as_any().type_id(),
                "two pipeline types claimed slot {idx}; if you dlopen plugins, \
             each copy of ui_lib has its own slot counter — see <docs link>",
            );
        }
        self.slots[idx] = Some(pipeline);
    }

    pub(crate) fn remove(&mut self, id: PipelineId) -> Option<Box<dyn Pipeline>> {
        self.slots.get_mut(id.index()).and_then(Option::take)
    }

    pub(crate) fn reload(
        &mut self,
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) {
        for pipeline in self.slots.iter_mut().flatten() {
            pipeline.reload(gpu, surface_format, texture_bgl, push_constant_ranges);
        }
    }

    #[inline]
    fn get(&self, id: PipelineId) -> Option<&dyn Pipeline> {
        self.slots.get(id.index())?.as_deref()
    }

    #[inline]
    pub(crate) fn get_mut(&mut self, id: PipelineId) -> Option<&mut dyn Pipeline> {
        self.slots.get_mut(id.index())?.as_deref_mut()
    }

    pub(crate) fn get_or_insert<P: Pipeline>(
        &mut self,
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) -> &mut P {
        let idx = PipelineId::of::<P>().index();
        if self.slots.len() <= idx {
            self.slots.resize_with(idx + 1, || None);
        }
        if self.slots[idx].is_none() {
            self.slots[idx] = Some(Box::new(P::new(
                gpu,
                surface_format,
                texture_bgl,
                push_constant_ranges,
            )));
        }

        self.slots[idx]
            .as_deref_mut()
            .expect("just inserted")
            .as_any_mut()
            .downcast_mut::<P>()
            // Unreachable: PipelineId::of::<P>() is unique to P.
            .expect("pipeline slot holds a different type than its PipelineId")
    }
}
