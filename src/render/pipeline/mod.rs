use core::sync::atomic::{AtomicU32, Ordering};

use crate::graphics::{Globals, Gpu};

pub mod ui;

pub use ui_macros::Pipeline;

static NEXT_INDEX: AtomicU32 = AtomicU32::new(1);

/// Do not implement this by hand — use
/// [`impl_pipeline_slot!`](crate::impl_pipeline_slot). Every implementation
/// MUST return a distinct `static`; two types sharing one would collide on the
/// same registry slot and silently draw with the wrong pipeline.
#[doc(hidden)]
pub trait PipelineSlot {
    fn slot() -> &'static AtomicU32
    where
        Self: Sized;
}

/// Identifies the pipeline that draws an instance.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PipelineId(u32);

impl PipelineId {
    #[inline(always)]
    pub fn of<P: Pipeline>() -> Self {
        let memo = P::slot();
        let v = memo.load(Ordering::Relaxed);
        if v != 0 {
            return Self(v - 1);
        }
        Self(assign(memo))
    }

    #[inline(always)]
    fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Copy, Clone)]
pub struct PipelineRegistration(
    fn(
        &mut PipelineRegistry,
        &Gpu,
        &wgpu::TextureFormat,
        &[wgpu::VertexBufferLayout],
        &wgpu::BindGroupLayout,
        &[wgpu::PushConstantRange],
    ),
);

impl PipelineRegistration {
    pub fn of<P: Pipeline>() -> Self {
        Self(
            |registry, gpu, surface_format, buffers, texture_bgl, ranges| {
                registry.insert(
                    PipelineId::of::<P>(),
                    Box::new(P::new(gpu, surface_format, buffers, texture_bgl, ranges)),
                );
            },
        )
    }
}

#[cold]
#[inline(never)]
fn assign(memo: &AtomicU32) -> u32 {
    let candidate = NEXT_INDEX.fetch_add(1, Ordering::Relaxed);
    assert!(candidate != 0, "pipeline index space exhausted");

    // Two threads racing on the same type both take a candidate; the loser's
    // is simply never claimed, leaving one permanent hole in every registry.
    match memo.compare_exchange(0, candidate, Ordering::Relaxed, Ordering::Relaxed) {
        Ok(_) => candidate - 1,
        Err(actual) => actual - 1,
    }
}

pub trait Pipeline: PipelineSlot + 'static {
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
        buffers: &[wgpu::VertexBufferLayout],
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) {
        self.insert(
            PipelineId::of::<ui::UiPipeline>(),
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
        self.get(PipelineId::of::<ui::UiPipeline>()).is_some()
    }

    pub(crate) fn register(
        &mut self,
        reg: PipelineRegistration,
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        buffers: &[wgpu::VertexBufferLayout],
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) {
        (reg.0)(
            self,
            gpu,
            surface_format,
            buffers,
            texture_bgl,
            push_constant_ranges,
        );
    }

    fn insert(&mut self, id: PipelineId, pipeline: Box<dyn Pipeline>) {
        let idx = id.index();
        if self.slots.len() <= idx {
            self.slots.resize_with(idx + 1, || None);
        }
        self.slots[idx] = Some(pipeline);
    }

    pub(crate) fn reload(
        &mut self,
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        buffers: &[wgpu::VertexBufferLayout],
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) {
        for pipeline in self.slots.iter_mut().flatten() {
            pipeline.reload(
                gpu,
                surface_format,
                buffers,
                texture_bgl,
                push_constant_ranges,
            );
        }
    }

    #[inline]
    fn get(&self, id: PipelineId) -> Option<&dyn Pipeline> {
        self.slots.get(id.index())?.as_deref()
    }

    pub(crate) fn apply_pipeline(
        &mut self,
        id: PipelineId,
        globals: &Globals,
        texture_bindgroup: &wgpu::BindGroup,
        pass: &mut wgpu::RenderPass<'_>,
    ) {
        let pipeline = self
            .slots
            .get_mut(id.index())
            .and_then(Option::as_deref_mut)
            .expect("pipeline index was emitted but never registered");
        pipeline.apply_pipeline(globals, texture_bindgroup, pass);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! impl_stub_pipeline {
        ($ty:ident) => {
            impl Pipeline for $ty {
                fn new(
                    _: &Gpu,
                    _: &wgpu::TextureFormat,
                    _: &[wgpu::VertexBufferLayout],
                    _: &wgpu::BindGroupLayout,
                    _: &[wgpu::PushConstantRange],
                ) -> Self {
                    unreachable!("test stub is never built")
                }
                fn reload(
                    &mut self,
                    _: &Gpu,
                    _: &wgpu::TextureFormat,
                    _: &[wgpu::VertexBufferLayout],
                    _: &wgpu::BindGroupLayout,
                    _: &[wgpu::PushConstantRange],
                ) {
                }
                fn apply_pipeline(
                    &mut self,
                    _: &Globals,
                    _: &wgpu::BindGroup,
                    _: &mut wgpu::RenderPass<'_>,
                ) {
                }
            }
        };
    }

    #[derive(Pipeline)]
    struct A;
    impl_stub_pipeline!(A);
    #[derive(Pipeline)]
    struct B;
    impl_stub_pipeline!(B);

    #[test]
    fn distinct_types_get_distinct_slots() {
        assert!(!core::ptr::eq(A::slot(), B::slot()));
    }

    #[test]
    fn id_is_stable_across_calls() {
        assert_eq!(PipelineId::of::<A>(), PipelineId::of::<A>());
        assert_ne!(PipelineId::of::<A>(), PipelineId::of::<B>());
    }
}
