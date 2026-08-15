use std::marker::PhantomData;

use super::Engine;

use crate::{
    context::MessageSink,
    defaults::DEFAULT_MAX_INSTANCES,
    render::{AllocatorKind, pipeline::PipelineRegistration},
    text::TextBackend,
    theme::Theme,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FeatureProfile {
    /// Current set: texture binding array + non-uniform indexing + partially
    /// bound binding array (dropped on Metal) + push constants. Desktop
    /// Vulkan / Metal / DX12.
    #[default]
    Bindless,
    /// Downlevel, binding-array-free path. Not yet implemented.
    Compat,
}

impl FeatureProfile {
    /// The binding-array features this profile adds on top of the always-on
    /// base (`PUSH_CONSTANTS | ADDRESS_MODE_CLAMP_TO_BORDER`). `is_metal`
    /// suppresses `PARTIALLY_BOUND_BINDING_ARRAY`, which Metal does not expose.
    ///
    /// Returns `Err` for profiles that are not yet implementable so `build()`
    /// can convert it into a typed error.
    pub(crate) fn binding_array_features(self, is_metal: bool) -> Result<wgpu::Features, ()> {
        match self {
            FeatureProfile::Bindless => {
                let mut f = wgpu::Features::TEXTURE_BINDING_ARRAY
                    | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING;
                if !is_metal {
                    f |= wgpu::Features::PARTIALLY_BOUND_BINDING_ARRAY;
                }
                Ok(f)
            }
            FeatureProfile::Compat => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TargetConfig {
    /// `None` auto-picks `AutoVsync` when available, else the first supported
    /// mode. `Some(mode)` requests `mode`, falling back to the auto pick if the
    /// surface does not support it.
    pub present_mode: Option<wgpu::PresentMode>,
    /// `None` auto-picks `PreMultiplied`/`Inherit` when available. `Some(mode)`
    /// requests `mode`, with the same auto fallback if unsupported.
    pub alpha_mode: Option<wgpu::CompositeAlphaMode>,
    /// `None` auto-picks an sRGB format. `Some(fmt)` requests `fmt`, falling
    /// back to the auto pick if the surface does not support it.
    pub format: Option<wgpu::TextureFormat>,
    /// `desired_maximum_frame_latency`. Was hard-coded to `1`.
    pub max_frame_latency: u32,
}

impl Default for TargetConfig {
    fn default() -> Self {
        Self {
            present_mode: None,
            alpha_mode: None,
            format: None,
            max_frame_latency: 1,
        }
    }
}

/// Where the engine's `wgpu::Device`/`Queue` come from.
pub(crate) enum GpuSource {
    /// Create an instance/adapter/device internally (the default).
    Create,
    /// Borrow an existing device/queue (embedding). The engine then owns no
    /// `wgpu::Instance`/`Adapter` and cannot create its own surfaces.
    Injected {
        device: wgpu::Device,
        queue: wgpu::Queue,
        adapter_info: wgpu::AdapterInfo,
    },
}
impl GpuSource {}

type LimitsFn = Box<dyn FnOnce(wgpu::Limits) -> wgpu::Limits>;

/// Accumulating builder for [`Engine`]. See the [module docs](crate::engine::builder).
pub struct EngineBuilder<M> {
    pub(crate) power_preference: wgpu::PowerPreference,
    pub(crate) force_fallback_adapter: bool,
    pub(crate) profile: FeatureProfile,
    pub(crate) extra_features: wgpu::Features,
    pub(crate) limits_override: LimitsFn,
    pub(crate) max_instances: u64,
    pub(crate) allocator: AllocatorKind,
    pub(crate) theme: Theme,
    pub(crate) target_defaults: TargetConfig,
    pub(crate) gpu_source: GpuSource,
    pub(crate) pending_pipelines: Vec<PipelineRegistration>,
    pub(crate) text_backend: Option<Box<dyn TextBackend>>,
    pub(crate) message_sink: Option<Box<dyn MessageSink>>,
    pub(crate) task_runner: Option<Box<dyn crate::task::TaskRunner>>,
    pub(crate) _marker: PhantomData<M>,
}

impl<M: 'static> Default for EngineBuilder<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> EngineBuilder<M> {
    pub fn new() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            profile: FeatureProfile::default(),
            extra_features: wgpu::Features::empty(),
            limits_override: Box::new(|l| l),
            max_instances: DEFAULT_MAX_INSTANCES,
            allocator: AllocatorKind::default(),
            theme: Theme::dark(),
            target_defaults: TargetConfig::default(),
            gpu_source: GpuSource::Create,
            pending_pipelines: Vec::new(),
            text_backend: None,
            message_sink: None,
            task_runner: None,
            _marker: PhantomData,
        }
    }

    /// Adapter power preference. Ignored when a device is injected via
    /// [`Self::with_wgpu`].
    pub fn power_preference(mut self, pref: wgpu::PowerPreference) -> Self {
        self.power_preference = pref;
        self
    }

    /// Force the software/fallback adapter. Ignored when a device is injected.
    pub fn force_fallback_adapter(mut self, force: bool) -> Self {
        self.force_fallback_adapter = force;
        self
    }

    /// Capability profile requested at device creation. Ignored when a device
    /// is injected (the embedder already chose its features).
    pub fn features(mut self, profile: FeatureProfile) -> Self {
        self.profile = profile;
        self
    }

    /// Power-user escape hatch: extra `wgpu::Features` OR-ed onto the profile's
    /// set. Ignored when a device is injected.
    pub fn extra_features(mut self, features: wgpu::Features) -> Self {
        self.extra_features = features;
        self
    }

    /// Mutate the `required_limits` before device creation. Ignored when a
    /// device is injected.
    pub fn limits_override(
        mut self,
        f: impl FnOnce(wgpu::Limits) -> wgpu::Limits + 'static,
    ) -> Self {
        self.limits_override = Box::new(f);
        self
    }

    /// Initial capacity of the per-frame instance buffer, in primitives. The
    /// buffer still grows on overflow; this only seeds the starting size.
    pub fn max_instances(mut self, n: u64) -> Self {
        self.max_instances = n;
        self
    }

    /// Glyph-atlas page allocator strategy.
    pub fn allocator(mut self, kind: AllocatorKind) -> Self {
        self.allocator = kind;
        self
    }

    /// Inject a custom text backend. When unset, the engine creates a
    /// [`TextCosmic`](crate::text::cosmic::TextCosmic) using the configured
    /// [`allocator`](Self::allocator) for its glyph atlas.
    pub fn text_backend(mut self, backend: Box<dyn TextBackend>) -> Self {
        self.text_backend = Some(backend);
        self
    }

    /// Initial theme. Defaults to [`Theme::dark`].
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Per-surface defaults used by `attach_target`/`new_for` and as the base
    /// for `attach_target_with`.
    pub fn target_defaults(mut self, cfg: TargetConfig) -> Self {
        self.target_defaults = cfg;
        self
    }

    /// Inject an existing device/queue instead of creating one. This is the
    /// difference between a windowing app and a library you can drop into an
    /// existing renderer.
    ///
    /// The engine will not own a `wgpu::Instance`/`Adapter`, so it cannot
    /// create its own surfaces; `attach_target` on such an engine fails with
    /// [`crate::error::InitError::NoInstance`]. `adapter_info` is used for
    /// feature-quirk decisions (e.g. the Metal binding-array case).
    pub fn with_wgpu(
        mut self,
        device: wgpu::Device,
        queue: wgpu::Queue,
        adapter_info: wgpu::AdapterInfo,
    ) -> Self {
        self.gpu_source = GpuSource::Injected {
            device,
            queue,
            adapter_info,
        };
        self
    }

    /// Register a custom pipeline before the first frame. Default pipelines are
    /// always registered; these are applied right after, once the first target
    /// establishes the surface format.
    /// Chainable: `.pipeline::<Planet>().pipeline::<Stars>()`.
    pub fn pipeline<P: crate::render::pipeline::Pipeline>(mut self) -> Self {
        self.pending_pipelines.push(PipelineRegistration::of::<P>());
        self
    }

    /// Consume the builder and create the engine. Fallible: surfaces
    /// adapter/device acquisition errors and unimplemented feature profiles.
    /// Provide a custom [`MessageSink`] for the engine to own and drain each
    /// `poll`. If unset, the engine uses a default
    /// [`BasicMessageSink`](crate::context::BasicMessageSink).
    ///
    /// A backend that injects messages from outside the widget tree provides a
    /// sink over shared storage and keeps a clone to write into (the engine
    /// drains the copy it owns here); see the sctk backend.
    pub fn with_message_sink(mut self, sink: Box<dyn MessageSink>) -> Self {
        self.message_sink = Some(sink);
        self
    }

    /// Provide a custom [`TaskRunner`](crate::task::TaskRunner) to drive the
    /// async half of [`Task`](crate::task::Task)s. If unset, the engine uses a
    /// thread-per-task [`ThreadRunner`](crate::task::ThreadRunner), which is
    /// fine for frame-polled loops (winit). A backend with a *blocking* event
    /// loop should supply a runner whose delivery wakes the loop.
    pub fn with_task_runner(mut self, runner: Box<dyn crate::task::TaskRunner>) -> Self {
        self.task_runner = Some(runner);
        self
    }

    pub fn build<'a>(self) -> crate::Result<Engine<'a>> {
        Engine::<'a>::from_builder(self)
    }
}
