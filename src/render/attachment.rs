use crate::render::pipeline::PassConfig;

pub(crate) struct Attachments {
    size: [u32; 2],
    generation: u32,
    config: Option<PassConfig>,

    depth: Option<wgpu::TextureView>,
    /// Bound as `texture_depth_2d` for pipelines that sample depth. Only
    /// created when the pass attaches depth read-only — sampling an
    /// attachment that is also written is forbidden.
    depth_sample: Option<(wgpu::BindGroupLayout, wgpu::BindGroup)>,
    /// Multisampled colour. When present the surface view becomes the resolve
    /// target instead of the render target.
    msaa: Option<wgpu::TextureView>,
}
#[allow(dead_code)]
impl Attachments {
    pub(crate) fn new() -> Self {
        Self {
            size: [0, 0],
            generation: u32::MAX,
            config: None,
            depth: None,
            depth_sample: None,
            msaa: None,
        }
    }

    /// Idempotent. Recreates only when size or pass config actually changed.
    pub(crate) fn ensure(
        &mut self,
        device: &wgpu::Device,
        size: [u32; 2],
        config: PassConfig,
        generation: u32,
    ) {
        let size = [size[0].max(1), size[1].max(1)];
        if self.size == size && self.generation == generation && self.config == Some(config) {
            return;
        }

        self.size = size;
        self.generation = generation;
        self.config = Some(config);
        self.depth = None;
        self.depth_sample = None;
        self.msaa = None;

        let extent = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };

        if let Some(format) = config.depth_format {
            // TEXTURE_BINDING is requested unconditionally so that flipping a
            // pipeline from DepthUse::Write to DepthUse::Read does not require
            // reallocating the texture with different usage.
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Attachment"),
                size: extent,
                mip_level_count: 1,
                sample_count: config.sample_count,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            // Multisampled depth cannot be bound as a plain texture_depth_2d,
            // and a written attachment cannot be sampled at all.
            if config.depth_read_only && config.sample_count == 1 {
                self.depth_sample = Some(Self::depth_bind_group(device, &view));
            }
            self.depth = Some(view);
        }

        if config.sample_count > 1 {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("MSAA Colour Attachment"),
                size: extent,
                mip_level_count: 1,
                sample_count: config.sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: config.color_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.msaa = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(?size, ?config, "attachments (re)created");
    }

    fn depth_bind_group(
        device: &wgpu::Device,
        view: &wgpu::TextureView,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Depth Sample Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Depth Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Depth Sample Bind Group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        (layout, bind_group)
    }

    pub(crate) fn depth_view(&self) -> Option<&wgpu::TextureView> {
        self.depth.as_ref()
    }
    pub(crate) fn msaa_view(&self) -> Option<&wgpu::TextureView> {
        self.msaa.as_ref()
    }
    pub(crate) fn depth_bind(&self) -> Option<&wgpu::BindGroup> {
        self.depth_sample.as_ref().map(|(_, bg)| bg)
    }
    /// Layout a pipeline needs to declare if it samples depth. Available from
    /// `PipelineCtx` via the engine.
    pub(crate) fn depth_bind_layout(&self) -> Option<&wgpu::BindGroupLayout> {
        self.depth_sample.as_ref().map(|(l, _)| l)
    }
}
