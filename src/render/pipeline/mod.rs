use crate::graphics::{Globals, Gpu};
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

pub mod ui;
pub use ui_macros::Pipeline;

pub type SlotAlloc = extern "C" fn() -> u32;

static NEXT_INDEX: AtomicU32 = AtomicU32::new(1);

// State encoding:
// 0           = Uninitialized (neither foreign nor local allocator has been locked in)
// 1           = Locked to LOCAL allocator
// ptr (> 1)   = Configured to FOREIGN allocator function pointer
static ALLOCATOR_STATE: AtomicUsize = AtomicUsize::new(0);
const STATE_LOCAL: usize = 1;

/// Configures a foreign slot allocator.
/// Returns `true` if registered successfully, or `false` if an allocation has already occurred.
#[must_use]
pub fn set_slot_alloc(f: SlotAlloc) -> bool {
    let f_ptr = f as usize;

    ALLOCATOR_STATE
        .compare_exchange(0, f_ptr, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

extern "C" fn local_alloc() -> u32 {
    NEXT_INDEX.fetch_add(1, Ordering::Relaxed)
}

pub fn slot_alloc() -> SlotAlloc {
    local_alloc
}

#[cold]
#[inline(never)]
fn assign(memo: &AtomicU32) -> u32 {
    let candidate = loop {
        match ALLOCATOR_STATE.load(Ordering::Acquire) {
            // Uninitialized: attempt to lock in the LOCAL allocator atomically
            0 => {
                if ALLOCATOR_STATE
                    .compare_exchange(0, STATE_LOCAL, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    break local_alloc();
                }
                // If CAS failed, another thread set state to Foreign or Local simultaneously; loop to re-read.
            }
            // State is locked to Local
            STATE_LOCAL => break local_alloc(),
            // State is set to Foreign pointer
            foreign_ptr => {
                let f: SlotAlloc = unsafe { core::mem::transmute(foreign_ptr) };
                break f();
            }
        }
    };
    assert!(candidate != 0, "pipeline index space exhausted");

    // Two threads racing on the same type both take a candidate; the loser's
    // is simply never claimed, leaving one permanent hole in every registry.
    match memo.compare_exchange(0, candidate, Ordering::Relaxed, Ordering::Relaxed) {
        Ok(_) => candidate - 1,
        Err(actual) => actual - 1,
    }
}

/// Do not implement this by hand — use
/// [`impl_pipeline_slot!`](crate::impl_pipeline_slot). Every implementation
/// MUST return a distinct `static`; two types sharing one would collide on the
/// same registry slot and silently draw with the wrong pipeline.
#[doc(hidden)]
pub trait PipelineSlot {
    fn slot() -> &'static AtomicU32
    where
        Self: Sized;

    fn as_any(&self) -> &dyn core::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any;
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
        &wgpu::BindGroupLayout,
        &[wgpu::PushConstantRange],
    ),
);

impl PipelineRegistration {
    pub fn of<P: Pipeline>() -> Self {
        Self(|registry, gpu, surface_format, texture_bgl, ranges| {
            registry.insert(
                PipelineId::of::<P>(),
                Box::new(P::new(gpu, surface_format, texture_bgl, ranges)),
            );
        })
    }
}

/// Shared per-frame resources a pipeline may bind.
pub struct DrawCtx<'a> {
    pub globals: &'a Globals,
    pub textures: &'a wgpu::BindGroup,
    pub instances: &'a wgpu::Buffer,
}

pub trait Pipeline: PipelineSlot + 'static {
    fn new(
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) -> Self
    where
        Self: Sized;

    fn reload(
        &mut self,
        gpu: &Gpu,
        surface_format: &wgpu::TextureFormat,
        texture_bgl: &wgpu::BindGroupLayout,
        push_constant_ranges: &[wgpu::PushConstantRange],
    );

    fn bind(&mut self, ctx: &DrawCtx, pass: &mut wgpu::RenderPass<'_>);

    fn draw(
        &mut self,
        ctx: &DrawCtx,
        pass: &mut wgpu::RenderPass<'_>,
        byte_offset: u64,
        count: u32,
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
        (reg.0)(self, gpu, surface_format, texture_bgl, push_constant_ranges);
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

#[cfg(test)]
macro_rules! impl_stub_pipeline {
    ($ty:ident) => {
        impl $crate::render::pipeline::Pipeline for $ty {
            fn new(
                _: &$crate::graphics::Gpu,
                _: &wgpu::TextureFormat,
                _: &wgpu::BindGroupLayout,
                _: &[wgpu::PushConstantRange],
            ) -> Self {
                unreachable!("test stub is never built")
            }
            fn reload(
                &mut self,
                _: &$crate::graphics::Gpu,
                _: &wgpu::TextureFormat,
                _: &wgpu::BindGroupLayout,
                _: &[wgpu::PushConstantRange],
            ) {
            }
            fn bind(
                &mut self,
                _: &$crate::render::pipeline::DrawCtx,
                _: &mut wgpu::RenderPass<'_>,
            ) {
            }
            fn draw(
                &mut self,
                _: &$crate::render::pipeline::DrawCtx,
                _: &mut wgpu::RenderPass<'_>,
                _: u64,
                _: u32,
            ) {
            }
        }
    };
}
#[cfg(test)]
pub(crate) use impl_stub_pipeline;

#[cfg(test)]
mod tests {
    use super::*;

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
