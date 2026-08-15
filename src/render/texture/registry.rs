use super::{Atlas, TextureHandle};
use crate::{
    defaults::DEFAULT_MAX_TEXTURES,
    graphics::Gpu,
    model::Size,
    render::{
        AllocatorKind,
        pack::{pack_slot_gen, pack_unorm2x16, unpack_slot_gen},
    },
};

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

#[derive(Clone)]
struct TexSlot {
    tex: wgpu::Texture,
    view: wgpu::TextureView,
}

pub struct TextureRegistry {
    layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,

    views: Vec<Option<TexSlot>>,
    gens: Vec<u32>,
    gens_buffer: wgpu::Buffer,

    free: Vec<usize>,
    placeholder_view: wgpu::TextureView,
}

impl TextureRegistry {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("UI Texture Sampler"),
            ..Default::default()
        });

        let placeholder = device.create_texture(&wgpu::TextureDescriptor {
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

        let gens_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI Texture Generations Buffer"),
            size: (std::mem::size_of::<u32>() * n) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut reg = Self {
            layout,
            bind_group: dummy_bind_group(device),
            sampler,

            views,
            gens,
            gens_buffer,
            free: (0..n).rev().collect(),
            placeholder_view,
        };
        reg.update_bind_group(device);
        reg
    }

    // FIX: this recreates the ENTIRE texture-array bind
    // group over every slot, and it is called from load_rgba8 / load_into_atlas
    // / unload on every single upload. Now that the task runtime can drain many
    // image finishers in one `poll`, that is N full rebuilds per frame. Replace
    // the eager calls with a `dirty: bool` flag set here, and rebuild exactly
    // once per frame — after the finisher-drain loop in `Engine::poll`, or at
    // the top of `render_if_needed`. Turns O(uploads * slots) into O(slots).
    fn update_bind_group(&mut self, device: &wgpu::Device) {
        let mut slice: Vec<&wgpu::TextureView> = Vec::with_capacity(self.views.len());
        for v in &self.views {
            slice.push(
                v.as_ref()
                    .map(|s| &s.view)
                    .unwrap_or(&self.placeholder_view),
            );
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
        gpu: &Gpu,
        width: u32,
        height: u32,
        pixels_rgba8: &[u8],
    ) -> TextureHandle {
        let idx = self
            .free
            .pop()
            .expect("Texture slots exhausted; bump DEFAULT_MAX_TEXTURES");

        let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
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
        gpu.queue.write_texture(
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
        let view = tex.create_view(&Default::default());

        self.views[idx] = Some(TexSlot { tex, view });

        gpu.queue.write_buffer(
            &self.gens_buffer,
            (std::mem::size_of::<u32>() * idx) as u64,
            bytemuck::cast_slice(&[self.gens[idx]]),
        );
        self.update_bind_group(&gpu.device);

        TextureHandle {
            slot_gen: pack_slot_gen(idx, self.gens[idx]),
            scale_packed: pack_unorm2x16([1.0, 1.0]),
            offset_packed: pack_unorm2x16([0.0, 0.0]),
            size_px: Size::new(width, height),
        }
    }

    pub fn update_rgba8(&mut self, gpu: &Gpu, handle: TextureHandle, pixels_rgba8: &[u8]) -> bool {
        let (idx, generation) = unpack_slot_gen(handle.slot_gen);
        if idx >= self.views.len() {
            return false;
        }
        if self.gens[idx] != generation {
            return false;
        }
        let Some(slot) = &self.views[idx] else {
            return false;
        };

        let w = handle.size_px.width;
        let h = handle.size_px.height;

        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &slot.tex,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            pixels_rgba8,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        true
    }

    pub fn unload(&mut self, gpu: &Gpu, handle: TextureHandle) -> bool {
        let (idx, generation) = unpack_slot_gen(handle.slot_gen);
        if idx >= self.views.len() {
            return false;
        }
        if self.gens[idx] != generation {
            return false;
        }

        self.views[idx] = None;
        self.gens[idx] = self.gens[idx].wrapping_add(1);
        self.free.push(idx);

        gpu.queue.write_buffer(
            &self.gens_buffer,
            (std::mem::size_of::<u32>() * idx) as u64,
            bytemuck::cast_slice(&[self.gens[idx]]),
        );
        self.update_bind_group(&gpu.device);
        true
    }

    pub fn create_atlas(
        &mut self,
        gpu: &Gpu,
        width: u32,
        height: u32,
        kind: AllocatorKind,
    ) -> Atlas {
        let idx = self
            .free
            .pop()
            .expect("Texture slots exhausted; bump DEFAULT_MAX_TEXTURES");
        let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("UI Atlas"),
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
        let view = tex.create_view(&Default::default());
        self.views[idx] = Some(TexSlot { tex, view });

        gpu.queue.write_buffer(
            &self.gens_buffer,
            (std::mem::size_of::<u32>() * idx) as u64,
            bytemuck::cast_slice(&[self.gens[idx]]),
        );
        self.update_bind_group(&gpu.device);

        Atlas::new(idx, self.gens[idx], Size::new(width, height), kind)
    }

    pub fn load_into_atlas(
        &mut self,
        gpu: &Gpu,
        atlas: &mut Atlas,
        w: u32,
        h: u32,
        pixels_rgba8: &[u8],
    ) -> Option<TextureHandle> {
        let rect = atlas.alloc(w, h)?;
        let slot = self.views[atlas.slot_index]
            .as_ref()
            .expect("atlas slot missing");

        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &slot.tex,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: rect.x,
                    y: rect.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            pixels_rgba8,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        let scale = [
            w as f32 / atlas.size_px.width as f32,
            h as f32 / atlas.size_px.height as f32,
        ];
        let offs = [
            rect.x as f32 / atlas.size_px.width as f32,
            rect.y as f32 / atlas.size_px.height as f32,
        ];

        Some(TextureHandle {
            slot_gen: pack_slot_gen(atlas.slot_index, atlas.generation),
            scale_packed: pack_unorm2x16(scale),
            offset_packed: pack_unorm2x16(offs),
            size_px: Size::new(w, h),
        })
    }

    pub fn destroy_atlas(&mut self, gpu: &Gpu, atlas: &mut Atlas) {
        let idx = atlas.slot_index;

        self.gens[idx] = self.gens[idx].wrapping_add(1);
        gpu.queue.write_buffer(
            &self.gens_buffer,
            (std::mem::size_of::<u32>() * idx) as u64,
            bytemuck::cast_slice(&[self.gens[idx]]),
        );

        self.views[idx] = None;
        self.update_bind_group(&gpu.device);
        self.free.push(idx);

        atlas.size_px = Size::new(0, 0);
        atlas.allocator.reset();
        atlas.generation = self.gens[idx];
    }
}
