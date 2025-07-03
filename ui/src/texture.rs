use crate::consts::*;
use crate::graphics::Config;
use crate::model::Size;
use image::DynamicImage;
use std::num::NonZeroU32;

#[derive(Debug, Copy, Clone)]
pub struct TextureHandle(pub(crate) u32);

#[derive(Debug, Copy, Clone)]
pub(crate) struct TextureInfo {
    handle: TextureHandle,
    dims: Size<u32>,
}

pub struct TextureArray {
    pub(crate) texture_array_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    dummy_view: wgpu::TextureView,
    handles: Vec<Option<TextureInfo>>,
    texture_views: Vec<wgpu::TextureView>,
    texture_bind_group: wgpu::BindGroup,

    dirty: bool,
}

impl TextureArray {
    pub(crate) fn new(config: &Config) -> Self {
        let texture_array_layout =
            config
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Texture Array Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: NonZeroU32::new(DEFAULT_MAX_TEXTURES),
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let sampler = config.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToBorder,
            address_mode_v: wgpu::AddressMode::ClampToBorder,
            address_mode_w: wgpu::AddressMode::ClampToBorder,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let dummy_texture = config.device.create_texture(&wgpu::TextureDescriptor {
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
            label: Some("Dummy Texture"),
            view_formats: &[],
        });
        let dummy_view = dummy_texture.create_view(&Default::default());
        let texture_views = vec![dummy_view.clone(); DEFAULT_MAX_TEXTURES as usize];

        let view_refs: Vec<&wgpu::TextureView> = texture_views.iter().collect();
        let texture_bind_group = config.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Array Bind Group"),
            layout: &texture_array_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(&view_refs),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture_array_layout,
            sampler,
            dummy_view,
            handles: vec![None; DEFAULT_MAX_TEXTURES as usize],
            texture_views,
            texture_bind_group,

            dirty: false,
        }
    }

    pub(crate) fn bind_group(&mut self, config: &Config) -> &wgpu::BindGroup {
        if self.dirty {
            self.update_texture_bind_group(config);
            self.dirty = false;
        }
        &self.texture_bind_group
    }

    fn update_texture_bind_group(&mut self, config: &Config) {
        let view_refs: Vec<&wgpu::TextureView> = self.texture_views.iter().collect();

        self.texture_bind_group = config.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Array Bind Group"),
            layout: &self.texture_array_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(&view_refs),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
    }

    pub(crate) fn load_texture(
        &mut self,
        config: &Config,
        img: DynamicImage,
    ) -> Result<TextureHandle, String> {
        let img = img.to_rgba8();
        let (width, height) = img.dimensions();

        let index = match self.handles.iter().position(|v| v.is_none()) {
            Some(i) => i,
            None => return Err(String::from("No texture slots left")),
        };

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = config.device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: None,
            view_formats: &[],
        });
        config.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &img,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let handle = TextureHandle(index as u32);
        self.texture_views[index] = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.handles[index] = Some(TextureInfo {
            handle,
            dims: Size::new(width, height),
        });

        self.dirty = true;

        Ok(handle)
    }

    pub(crate) fn unload_texture(&mut self, handle: TextureHandle) {
        self.handles[handle.0 as usize] = None;
        self.texture_views[handle.0 as usize] = self.dummy_view.clone();
        self.dirty = true;
    }

    pub fn get_tex_info(
        &self,
        texture_handle: &TextureHandle,
    ) -> Result<(u32, Size<u32>), &'static str> {
        let index = self
            .handles
            .iter()
            .position(|h| h.is_some_and(|x| x.handle.0 == texture_handle.0))
            .map(|idx| idx as u32)
            .ok_or("Texture handle not found")?;
        let size = self.handles[index as usize]
            .map(|i| i.dims)
            .ok_or("Texture handle not found")?;
        Ok((index, size))
    }
}
