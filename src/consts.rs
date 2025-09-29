pub const DEFAULT_MAX_TEXTURES: u32 = 128;
pub const DEFAULT_MAX_INSTANCES: u64 = 10_000;

pub(crate) fn feature_backends() -> wgpu::Backends {
    if cfg!(any(feature = "metal", feature = "vulkan")) {
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

    #[cfg(feature = "env_logging")]
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
