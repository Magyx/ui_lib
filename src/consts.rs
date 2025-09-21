pub const DEFAULT_MAX_TEXTURES: u32 = 128;
pub const DEFAULT_MAX_INSTANCES: u64 = 10_000;

pub(crate) fn feature_backends() -> wgpu::Backends {
    #[cfg(feature = "metal")]
    #[cfg(feature = "vulkan")]
    if cfg!(all(feature = "metal", feature = "vulkan")) {
        let mut b = wgpu::Backends::empty();
        #[cfg(feature = "vulkan")]
        {
            b |= wgpu::Backends::VULKAN;
        }
        #[cfg(feature = "metal")]
        {
            b |= wgpu::Backends::METAL;
        }
        return b;
    }

    wgpu::Backends::PRIMARY
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
