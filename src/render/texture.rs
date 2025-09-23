use crate::{consts::DEFAULT_MAX_TEXTURES, graphics::Config, model::Size};

fn dummy_bind_group(device: &wgpu::Device) -> wgpu::BindGroup {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("dummy"),
        entries: &[],
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("dummy"),
        layout: &layout,
        entries: &[],
    })
}

#[inline]
pub fn pack_unorm2x16(xy: [f32; 2]) -> u32 {
    let q = |v: f32| -> u32 { (v.clamp(0.0, 1.0) * 65535.0 + 0.5).floor() as u32 };
    q(xy[0]) | (q(xy[1]) << 16)
}

#[inline]
pub fn unpack_unorm2x16(p: u32) -> [f32; 2] {
    [(p & 0xFFFF) as f32 / 65535.0, (p >> 16) as f32 / 65535.0]
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct TextureHandle {
    pub index: u32,
    pub generation: u32,
    pub scale_packed: u32,
    pub offset_packed: u32,
    pub size_px: Size<u32>,
}

pub struct TextureRegistry {
    layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,

    views: Vec<Option<wgpu::TextureView>>,
    gens: Vec<u32>,
    gens_buffer: wgpu::Buffer,

    free: Vec<usize>,
    placeholder_view: wgpu::TextureView,
}

impl TextureRegistry {
    pub fn new(config: &Config) -> Self {
        let layout = config
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("UI Texture Array BGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: std::num::NonZeroU32::new(DEFAULT_MAX_TEXTURES),
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let sampler = config.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("UI Texture Sampler"),
            ..Default::default()
        });

        let placeholder = config.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("UI Placeholder Tex"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let placeholder_view = placeholder.create_view(&Default::default());

        let n = DEFAULT_MAX_TEXTURES as usize;
        let views = vec![None; n];
        let gens = vec![0u32; n];

        let gens_buffer = config.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI Texture Generations Buffer"),
            size: (std::mem::size_of::<u32>() * n) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut reg = Self {
            layout,
            bind_group: dummy_bind_group(&config.device),
            sampler,

            views,
            gens,
            gens_buffer,
            free: (0..n).rev().collect(),
            placeholder_view,
        };
        reg.update_bind_group(&config.device);
        reg
    }

    fn update_bind_group(&mut self, device: &wgpu::Device) {
        let mut slice: Vec<&wgpu::TextureView> = Vec::with_capacity(self.views.len());
        for v in &self.views {
            slice.push(v.as_ref().unwrap_or(&self.placeholder_view));
        }

        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UI Texture Array BG"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(&slice),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.gens_buffer.as_entire_binding(),
                },
            ],
        });
    }

    pub fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn load_rgba8(
        &mut self,
        config: &Config,
        width: u32,
        height: u32,
        pixels_rgba8: &[u8],
    ) -> TextureHandle {
        let idx = self
            .free
            .pop()
            .expect("Texture slots exhausted; bump DEFAULT_MAX_TEXTURES");

        let tex = config.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("UI Image"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        config.queue.write_texture(
            tex.as_image_copy(),
            pixels_rgba8,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.views[idx] = Some(tex.create_view(&Default::default()));

        config.queue.write_buffer(
            &self.gens_buffer,
            (std::mem::size_of::<u32>() * idx) as u64,
            bytemuck::cast_slice(&[self.gens[idx]]),
        );
        self.update_bind_group(&config.device);

        TextureHandle {
            index: idx as u32,
            generation: self.gens[idx],
            scale_packed: pack_unorm2x16([1.0, 1.0]),
            offset_packed: pack_unorm2x16([0.0, 0.0]),
            size_px: Size::new(width, height),
        }
    }

    pub fn unload(&mut self, config: &Config, handle: TextureHandle) -> bool {
        let idx = handle.index as usize;
        if idx >= self.views.len() {
            return false;
        }
        if self.gens[idx] != handle.generation {
            return false;
        }

        self.views[idx] = None;
        self.gens[idx] = self.gens[idx].wrapping_add(1);
        self.free.push(idx);

        config.queue.write_buffer(
            &self.gens_buffer,
            (std::mem::size_of::<u32>() * idx) as u64,
            bytemuck::cast_slice(&[self.gens[idx]]),
        );
        self.update_bind_group(&config.device);
        true
    }
}
