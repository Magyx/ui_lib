pub mod wgsl {
    pub fn load_wgsl(
        device: &wgpu::Device,
        path: &std::path::Path,
        label: &str,
    ) -> wgpu::ShaderModule {
        use std::{borrow::Cow, fs};
        let src = fs::read_to_string(path).expect("wgsl not found");
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(Cow::Owned(src)),
        })
    }
}
