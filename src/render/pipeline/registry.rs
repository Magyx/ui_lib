use super::{DrawCtx, PassConfig, PassRequirements, Pipeline, PipelineCtx, PipelineId, ui};
use crate::{gpu::Gpu, primitive::Batch};

/// The pass-independent inputs to building a pipeline. Bundled so that adding
/// one later is not a breaking change at every call site.
#[derive(Copy, Clone)]
pub(crate) struct RegistryEnv<'a> {
    pub gpu: &'a Gpu,
    pub color_format: wgpu::TextureFormat,
    pub texture_bgl: &'a wgpu::BindGroupLayout,
    pub immediate_size: u32,
}

#[derive(Copy, Clone)]
pub struct PipelineRegistration {
    pub id: PipelineId,
    build: fn(&mut PipelineRegistry, RegistryEnv),
}

impl PipelineRegistration {
    pub fn of<P: Pipeline>() -> Self {
        Self {
            id: PipelineId::of::<P>(),
            build: |registry, env| {
                let _ = registry.get_or_insert::<P>(env);
            },
        }
    }
}

struct Entry {
    pipeline: Box<dyn Pipeline>,
    requirements: PassRequirements,
    name: &'static str,
}

fn checked<T>(gpu: &Gpu, name: &str, config: PassConfig, build: impl FnOnce() -> T) -> T {
    let scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
    let value = build();
    if let Some(err) = pollster::block_on(scope.pop()) {
        panic!(
            "pipeline `{name}` is not compatible with the shared pass.\n\
             pass: {config:?}\n\
             {err}\n\
             \n\
             Pass state must come from `PipelineCtx` — use \
             `ctx.create_render_pipeline(..)`, or at minimum \
             `ctx.depth_state_passthrough()` and `ctx.multisample_state()`. \
             Hard-coded `depth_stencil: None` breaks as soon as another \
             pipeline pulls depth into the pass."
        );
    }
    value
}

pub(crate) struct PipelineRegistry {
    slots: Vec<Option<Entry>>,
    merged: PassRequirements,
    config: Option<PassConfig>,
    /// Bumped whenever `config` changes. Attachments compare against this to
    /// know when to recreate.
    generation: u32,
}

impl PipelineRegistry {
    pub(crate) fn new() -> Self {
        Self {
            slots: Vec::new(),
            merged: PassRequirements::NONE,
            config: None,
            generation: 0,
        }
    }

    /// The resolved pass every pipeline was built against. `None` before the
    /// first registration.
    pub(crate) fn pass_config(&self) -> Option<PassConfig> {
        self.config
    }

    pub(crate) fn generation(&self) -> u32 {
        self.generation
    }

    pub(crate) fn requirements_of(&self, id: PipelineId) -> Option<PassRequirements> {
        Some(self.slots.get(id.index())?.as_ref()?.requirements)
    }

    pub(crate) fn register_default_pipelines(&mut self, env: RegistryEnv) {
        let _ = self.get_or_insert::<ui::UiPipeline>(env);
    }

    pub(crate) fn has_default_pipelines(&self) -> bool {
        self.slots
            .get(PipelineId::of::<ui::UiPipeline>().index())
            .is_some_and(Option::is_some)
    }

    pub(crate) fn register(&mut self, reg: PipelineRegistration, env: RegistryEnv) {
        (reg.build)(self, env);
    }

    pub(crate) fn get_or_insert<P: Pipeline>(&mut self, env: RegistryEnv) -> &mut P {
        let idx = PipelineId::of::<P>().index();

        if self.slots.len() <= idx {
            self.slots.resize_with(idx + 1, || None);
        }

        if self.slots[idx].is_none() {
            let requirements = P::requirements();

            // Widening the merged requirements changes the pass, which
            // invalidates every pipeline already built against the old one.
            // Do that *before* building P, so P is built against the final
            // config and is not itself reloaded twice.
            if !self.merged.covers(requirements) {
                self.merged = self.merged.merge(requirements).unwrap_or_else(|e| {
                    panic!(
                        "{} conflicts with the registered pipelines: {e}",
                        core::any::type_name::<P>()
                    )
                });
                self.rebuild_config(env);
                self.reload_all(env);
            } else if self.config.is_none() {
                self.rebuild_config(env);
            }

            let ctx = self.ctx(env);
            let name = std::any::type_name::<P>();
            self.slots[idx] = Some(Entry {
                pipeline: Box::new(P::new(&ctx)),
                requirements,
                name,
            });
        }

        self.slots[idx]
            .as_mut()
            .expect("just inserted")
            .pipeline
            .as_any_mut()
            .downcast_mut::<P>()
            // Unreachable: PipelineId::of::<P>() is unique to P.
            .expect("pipeline slot holds a different type than its PipelineId")
    }

    pub(crate) fn remove(&mut self, id: PipelineId) -> Option<Box<dyn Pipeline>> {
        // The merged requirements are deliberately *not* narrowed here.
        // Shrinking the pass would invalidate every surviving pipeline for no
        // benefit; an over-provisioned depth buffer costs memory, not
        // correctness.
        let entry = self.slots.get_mut(id.index()).and_then(Option::take)?;
        Some(entry.pipeline)
    }

    /// Rebuild every pipeline against the current config. Called on surface
    /// reconfiguration and after the pass widens.
    pub(crate) fn reload_all(&mut self, env: RegistryEnv) {
        if self.config.is_none() {
            self.rebuild_config(env);
        }
        let ctx = self.ctx(env);
        for entry in self.slots.iter_mut().flatten() {
            let name = entry.name;
            checked(env.gpu, name, ctx.pass, || entry.pipeline.reload(&ctx));
        }
    }

    fn rebuild_config(&mut self, env: RegistryEnv) {
        let next = self.merged.resolve(env.gpu, env.color_format);
        if self.config != Some(next) {
            self.config = Some(next);
            self.generation = self.generation.wrapping_add(1);

            #[cfg(feature = "tracing")]
            tracing::debug!(?next, "pass config changed");
        }
    }

    fn ctx<'a>(&self, env: RegistryEnv<'a>) -> PipelineCtx<'a> {
        PipelineCtx {
            gpu: env.gpu,
            pass: self.config.expect("config resolved before building"),
            texture_bgl: env.texture_bgl,
            immediate_size: env.immediate_size,
        }
    }

    #[inline]
    pub(crate) fn get_mut(&mut self, id: PipelineId) -> Option<&mut dyn Pipeline> {
        Some(self.slots.get_mut(id.index())?.as_mut()?.pipeline.as_mut())
    }

    /// Pipeline ids with instances this frame, in first-appearance order.
    /// The renderer drives [`Pipeline::frame`] itself — building a `FrameCtx`
    /// needs the encoder, which only the renderer holds.
    pub(crate) fn active_ids(batches: &[Batch]) -> Vec<PipelineId> {
        let mut ids: Vec<PipelineId> = Vec::new();
        for b in batches {
            if !ids.contains(&b.id) {
                ids.push(b.id);
            }
        }
        ids
    }

    pub(crate) fn draw_batch(
        &mut self,
        id: PipelineId,
        ctx: &DrawCtx,
        pass: &mut wgpu::RenderPass<'_>,
        batch: &Batch,
        rebind: bool,
    ) -> bool {
        let Some(pipeline) = self.get_mut(id) else {
            #[cfg(feature = "tracing")]
            tracing::warn!("batch emitted for an unregistered pipeline: {id:?}");
            debug_assert!(false, "batch emitted for an unregistered pipeline");
            return false;
        };
        if rebind {
            pipeline.bind(ctx, pass);
        }
        pipeline.draw(ctx, pass, batch);
        true
    }
}
