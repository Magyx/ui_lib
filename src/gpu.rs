use crate::defaults::DEFAULT_MAX_TEXTURES;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    pub window_size: [f32; 2], // logical pixels
    pub mouse_pos: [f32; 2],   // logical pixels
    pub mouse_buttons: u32,    // bit 0: left, bit 1: right (etc.)
    pub time: f32,             // seconds since start
    pub delta_time: f32,       // seconds since last frame
    pub frame: u32,            // frame counter
    pub scale: f32,            // device-pixel ratio
    pub _pad: f32,
}

pub struct Gpu {
    pub instance: Option<wgpu::Instance>,
    pub adapter: Option<wgpu::Adapter>,
    pub adapter_info: wgpu::AdapterInfo,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    pub fn required_features() -> wgpu::Features {
        wgpu::Features::IMMEDIATES
            | wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER
            | wgpu::Features::TEXTURE_BINDING_ARRAY
            | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
            | wgpu::Features::PARTIALLY_BOUND_BINDING_ARRAY
    }

    pub fn required_limits(adapter: &wgpu::Adapter) -> wgpu::Limits {
        wgpu::Limits {
            max_immediate_size: 64,
            max_binding_array_elements_per_shader_stage: DEFAULT_MAX_TEXTURES
                .min(adapter.limits().max_binding_array_elements_per_shader_stage),
            ..Default::default()
        }
    }

    pub fn headless() -> Option<Self> {
        let backends = wgpu::Instance::enabled_backend_features();
        if backends.is_empty() {
            eprintln!(
                "ui: no wgpu backend compiled in — build with `--features vulkan` \
                 (or metal/dx12/gl)"
            );
            return None;
        }

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            flags: crate::gpu::default_instance_flags(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .ok()?;

        let adapter_info = adapter.get_info();
        let features = Self::required_features();

        if !adapter.features().contains(features) {
            eprintln!(
                "ui: adapter {:?} is missing required features: {:?}",
                adapter_info.name,
                features - adapter.features()
            );
            return None;
        }

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("ui headless"),
            required_features: features,
            required_limits: Self::required_limits(&adapter),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        }))
        .inspect_err(|e| eprintln!("ui: request_device failed: {e}"))
        .ok()?;

        Some(Self {
            instance: Some(instance),
            adapter: Some(adapter),
            adapter_info,
            device,
            queue,
        })
    }
}

pub(crate) fn feature_backends() -> wgpu::Backends {
    if cfg!(any(feature = "metal", feature = "vulkan")) {
        #[allow(unused)]
        let mut b = wgpu::Backends::empty();
        #[cfg(feature = "vulkan")]
        {
            b |= wgpu::Backends::VULKAN;
        }
        #[cfg(feature = "metal")]
        {
            b |= wgpu::Backends::METAL;
        }
        b
    } else {
        wgpu::Backends::PRIMARY
    }
}

pub(crate) fn env_override_backends(current: wgpu::Backends) -> wgpu::Backends {
    match std::env::var("UI_BACKEND") {
        Ok(s) => match s.to_ascii_lowercase().as_str() {
            "vulkan" => wgpu::Backends::VULKAN,
            "metal" => wgpu::Backends::METAL,
            "dx12" => wgpu::Backends::DX12,
            "gl" => wgpu::Backends::GL,
            "primary" => wgpu::Backends::PRIMARY,
            "all" => wgpu::Backends::all(),
            _ => current,
        },
        Err(_) => current,
    }
}

pub(crate) fn default_backends() -> wgpu::Backends {
    env_override_backends(feature_backends())
}

pub(crate) fn default_instance_flags() -> wgpu::InstanceFlags {
    let mut flags = wgpu::InstanceFlags::empty();

    #[cfg(feature = "tracing")]
    {
        flags.insert(wgpu::InstanceFlags::DEBUG);
    }

    if let Ok(v) = std::env::var("UI_WGPU_DEBUG") {
        let on = matches!(v.to_ascii_lowercase().as_str(), "1" | "on" | "true" | "yes");
        if on {
            flags.insert(wgpu::InstanceFlags::DEBUG);
        } else {
            flags.remove(wgpu::InstanceFlags::DEBUG);
        }
    }

    if let Ok(v) = std::env::var("UI_WGPU_VALIDATION") {
        let on = matches!(v.to_ascii_lowercase().as_str(), "1" | "on" | "true" | "yes");
        if on {
            flags.insert(wgpu::InstanceFlags::VALIDATION);
        }
    }

    flags
}
