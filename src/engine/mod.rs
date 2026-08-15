use std::{collections::HashMap, sync::Arc};

use crate::{
    context::{BasicMessageSink, MessageSink},
    engine::builder::{EngineBuilder, GpuSource, TargetConfig},
    event::Modifiers,
    gpu::{Globals, Gpu, default_backends, default_instance_flags},
    layout::LayoutEngine,
    model::*,
    primitive::InstanceStore,
    render::{
        AllocatorKind,
        pipeline::{Pipeline, PipelineId, PipelineRegistration, PipelineRegistry},
        renderer::Renderer,
        texture::{Atlas, TextureHandle},
    },
    task::{Payload, TaskId, TaskRunner, ThreadRunner},
    text::TextBackend,
    theme::Theme,
};

pub use target::*;

pub mod builder;
pub mod frame;
mod target;

pub struct Engine<'a> {
    instance_buf: InstanceStore,
    layout_engine: LayoutEngine,
    theme: Theme,

    gpu: Arc<Gpu>,
    target_alloc: TargetIdAlloc,
    primary_target: Option<TargetId>,
    targets: HashMap<TargetId, Target<'a>>,
    immediate_size: u32,
    pipeline_registry: PipelineRegistry,
    renderer: Renderer,
    text: Box<dyn TextBackend>,
    message_sink: Box<dyn MessageSink>,

    runner: Box<dyn TaskRunner>,
    landings: Vec<(TargetId, TaskId, Payload)>,

    target_defaults: TargetConfig,
    pending_pipelines: Vec<PipelineRegistration>,
}
impl<'a> Engine<'a> {
    /// Start an [`EngineBuilder`]. This is the configurable entry point; see
    /// [`crate::engine::builder`] for the full set of knobs.
    pub fn builder<M: 'static>() -> EngineBuilder<M> {
        EngineBuilder::default()
    }

    pub(crate) fn from_builder<M: 'static>(builder: EngineBuilder<M>) -> crate::Result<Self> {
        let EngineBuilder {
            power_preference,
            force_fallback_adapter,
            profile,
            extra_features,
            limits_override,
            max_instances,
            allocator,
            theme,
            target_defaults,
            gpu_source,
            pending_pipelines,
            text_backend,
            message_sink,
            task_runner,
            _marker,
        } = builder;

        let gpu = match gpu_source {
            GpuSource::Create => {
                // Env overrides (UI_BACKEND / UI_WGPU_*) are applied here and so
                // win last over any builder intent.
                let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                    backends: default_backends(),
                    flags: default_instance_flags(),
                    ..wgpu::InstanceDescriptor::new_without_display_handle()
                });

                let adapter =
                    pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference,
                        compatible_surface: None,
                        force_fallback_adapter,
                        apply_limit_buckets: false,
                    }))
                    .map_err(|_| crate::error::InitError::NoAdapter)?;

                let adapter_info = adapter.get_info();
                let is_metal = adapter_info.backend == wgpu::Backend::Metal;

                // Validates the profile; `Compat` is not yet implementable.
                profile
                    .binding_array_features(is_metal)
                    .map_err(|_| crate::error::InitError::UnsupportedFeatureProfile)?;

                let required_limits = limits_override(Gpu::required_limits());

                let (device, queue) =
                    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                        label: None,
                        required_features: Gpu::required_features(is_metal) | extra_features,
                        required_limits,
                        experimental_features: wgpu::ExperimentalFeatures::disabled(),
                        memory_hints: wgpu::MemoryHints::MemoryUsage,
                        trace: wgpu::Trace::Off,
                    }))
                    .map_err(crate::error::InitError::RequestDevice)?;

                #[cfg(feature = "tracing")]
                device.on_uncaptured_error(Arc::new(|err| {
                    tracing::warn!("wgpu uncaptured error: {err}");
                }));

                Gpu {
                    instance: Some(instance),
                    adapter: Some(adapter),
                    adapter_info,
                    device,
                    queue,
                }
            }
            GpuSource::Injected {
                device,
                queue,
                adapter_info,
            } => {
                // The embedder already created the device with whatever features
                // it needs; profile/limits/power-preference are not ours to apply.
                // We still validate the profile so `Compat` fails consistently.
                let is_metal = adapter_info.backend == wgpu::Backend::Metal;
                profile
                    .binding_array_features(is_metal)
                    .map_err(|_| crate::error::InitError::UnsupportedFeatureProfile)?;

                Gpu {
                    instance: None,
                    adapter: None,
                    adapter_info: *adapter_info,
                    device,
                    queue,
                }
            }
        };

        let immediate_size = std::mem::size_of::<Globals>() as u32;

        let pipeline_registry = PipelineRegistry::new();
        let renderer = Renderer::with_capacity(&gpu.device, max_instances);

        let text = text_backend.unwrap_or_else(|| {
            #[cfg(feature = "text_cosmic")]
            {
                Box::new(crate::text::cosmic::TextCosmic::new(allocator))
            }
            #[cfg(not(feature = "text_cosmic"))]
            {
                _ = allocator;
                tracing::error!("No text backend enabled or selected!");
                unimplemented!()
            }
        });
        let message_sink = message_sink.unwrap_or_else(|| Box::new(BasicMessageSink::new()));
        let runner = task_runner.unwrap_or_else(|| Box::new(ThreadRunner::new()));

        Ok(Self {
            layout_engine: LayoutEngine::new(),
            instance_buf: InstanceStore::new(),
            theme,

            gpu: Arc::new(gpu),
            target_alloc: TargetIdAlloc::default(),
            primary_target: None,
            targets: HashMap::with_capacity(1),
            immediate_size,
            pipeline_registry,
            renderer,
            text,
            message_sink,
            runner,
            landings: Vec::new(),

            target_defaults,
            pending_pipelines,
        })
    }

    pub fn new_for<M: 'static, T>(
        target: Arc<T>,
        physical_size: Size<u32>,
        scale_factor: f64,
    ) -> (TargetId, Self)
    where
        T: wgpu::rwh::HasWindowHandle
            + wgpu::rwh::HasDisplayHandle
            + Sized
            + std::marker::Sync
            + std::marker::Send
            + 'a,
    {
        let mut engine = Self::builder::<M>()
            .build()
            .expect("wgpu: failed to initialize default Engine");

        let cfg = engine.target_defaults;
        let target = engine
            .create_target(target, physical_size, scale_factor, &cfg)
            .expect("wgpu: failed to create target surface");

        (target, engine)
    }

    pub fn reload_all(&mut self) {
        let fmt = if let Some(t) = self.primary_target() {
            t.config.format
        } else {
            return;
        };

        self.pipeline_registry.reload(
            &self.gpu,
            &fmt,
            self.renderer.textures.layout(),
            self.immediate_size,
        );
    }

    pub fn toggle_debug(&mut self) {
        self.layout_engine.toggle_debug();
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        for t in self.targets.values_mut() {
            t.ctx.request_redraw();
        }
    }

    pub fn globals(&self, tid: &TargetId) -> Option<Globals> {
        self.targets.get(tid).map(|t| t.globals)
    }
    pub fn modifiers(&self, tid: &TargetId) -> Option<Modifiers> {
        self.targets.get(tid).map(|t| t.ctx.modifiers)
    }

    /// Register a pipeline for `P` after startup.
    ///
    /// No-ops if no target exists yet, since the surface format isn't known
    /// until then; use [`EngineBuilder::pipeline`](crate::engine::builder::EngineBuilder::pipeline)
    /// for pipelines known up front.
    pub fn register_pipeline<P: crate::render::pipeline::Pipeline>(&mut self) {
        self.register(PipelineRegistration::of::<P>());
    }

    /// Same, for callers that already hold a [`PipelineRegistration`] (the
    /// backends collect them from their builders).
    pub fn register(&mut self, reg: PipelineRegistration) {
        let fmt = if let Some(t) = self.primary_target() {
            t.config.format
        } else {
            self.pending_pipelines.push(reg);
            return;
        };

        self.pipeline_registry.register(
            reg,
            &self.gpu,
            &fmt,
            self.renderer.textures.layout(),
            self.immediate_size,
        );
    }

    pub fn unregister(&mut self, id: PipelineId) -> Option<Box<dyn Pipeline>> {
        self.pipeline_registry.remove(id)
    }

    pub fn unregister_pipeline<P: Pipeline>(&mut self) -> Option<Box<dyn Pipeline>> {
        self.unregister(PipelineId::of::<P>())
    }

    pub fn load_texture_rgba8(&mut self, width: u32, height: u32, pixels: &[u8]) -> TextureHandle {
        self.renderer
            .textures
            .load_rgba8(&self.gpu, width, height, pixels)
    }
    pub fn update_texture_rgba8(&mut self, handle: TextureHandle, pixels: &[u8]) -> bool {
        self.renderer
            .textures
            .update_rgba8(&self.gpu, handle, pixels)
    }
    pub fn unload_texture(&mut self, handle: TextureHandle) -> bool {
        self.renderer.textures.unload(&self.gpu, handle)
    }
    pub fn create_atlas(&mut self, width: u32, height: u32, kind: AllocatorKind) -> Atlas {
        self.renderer
            .textures
            .create_atlas(&self.gpu, width, height, kind)
    }
    pub fn load_texture_into_atlas(
        &mut self,
        atlas: &mut Atlas,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Option<TextureHandle> {
        self.renderer
            .textures
            .load_into_atlas(&self.gpu, atlas, width, height, pixels)
    }
    pub fn free_from_atlas(&mut self, atlas: &mut Atlas, handle: TextureHandle) {
        atlas.free(handle);
    }
    pub fn destroy_atlas(&mut self, atlas: &mut Atlas) {
        self.renderer.textures.destroy_atlas(&self.gpu, atlas)
    }
}
