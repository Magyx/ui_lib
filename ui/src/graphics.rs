use std::{num::NonZeroU32, path::PathBuf, sync::Arc};

use image::DynamicImage;
use wgpu::{SurfaceConfiguration, util::DeviceExt};

use crate::{
    context::Context,
    event::{Event, ToEvent},
    model::*,
    primitive::{Primitive, QUAD_INDICES, QUAD_VERTICES, Vertex},
    utils,
    widget::{Element, RenderOutput, Text, TextStyle, Widget},
};

const DEFAULT_MAX_TEXTURES: u32 = 128;
const DEFAULT_MAX_INSTANCES: u64 = 10_000;

fn cascade_widgets<'a, M>(
    widgets: &'a Box<dyn Widget<Message = M> + 'a>,
    window_size: &[f32],
    textures: &TextureArray,
    texts: &mut TextBundle,
    ctx: &mut Context<M>,
) -> Result<(Option<Vec<Primitive>>, Option<Vec<Text<'a>>>), &'static str> {
    let RenderOutput { primitives, texts } = widgets.as_primitive(
        Size {
            width: window_size[0] as i32,
            height: window_size[1] as i32,
        },
        textures,
        texts,
        ctx,
    )?;

    Ok((
        primitives
            .map(|p| p.iter().map(|p| p.primitive).collect())
            .filter(|p: &Vec<Primitive>| !p.is_empty()),
        texts.filter(|v| !v.is_empty()),
    ))
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    window_size: [f32; 2],
}

pub struct Engine<'a, M> {
    globals: Globals,
    config: Config<'a>,
    ctx: Context<M>,

    textures: TextureArray,
    primitive_bundle: PrimitiveBundle,
    text_bundle: TextBundle,
}

impl<'a, M: std::fmt::Debug + 'static> Engine<'a, M> {
    pub fn new<T>(target: Arc<T>, size: Size<u32>) -> Engine<'a, M>
    where
        T: wgpu::rwh::HasWindowHandle
            + wgpu::rwh::HasDisplayHandle
            + Sized
            + std::marker::Sync
            + std::marker::Send
            + 'a,
    {
        let ctx = Context::new();

        let config = Config::new(target, &size);

        let textures = TextureArray::new(&config);

        let primitive_bundle =
            PrimitiveBundle::primitive(&config, &textures.texture_array_layout, None);
        let mut text_bundle = TextBundle::new(&config.device, &config.queue, &config.config);
        text_bundle.resize(&config.queue, &size);

        Self {
            config,
            globals: Globals {
                window_size: [size.width as f32, size.height as f32],
            },
            ctx,

            textures,
            primitive_bundle,
            text_bundle,
        }
    }

    pub fn load_texture(&mut self, img: DynamicImage) -> Result<TextureHandle, String> {
        self.textures.load_texture(&self.config, img)
    }

    pub fn unload_texture(&mut self, handle: TextureHandle) {
        self.textures.unload_texture(handle);
    }

    pub fn reload_all(&mut self) {
        self.primitive_bundle.reload(
            &self.config.device,
            self.config.config.format,
            &self.textures.texture_array_layout,
        );
    }

    pub fn handle_event<S, P, E: ToEvent<M, E>>(
        &mut self,
        event: &E,
        build: impl FnOnce(&S) -> Element<M>,
        update: &mut impl FnMut(&mut Self, &Event<M, E>, &mut S, &P) -> Option<M>,
        state: &mut S,
        params: &P,
    ) {
        let event = event.to_event();
        let prev_mouse_down = self.ctx.mouse_down;

        match event {
            Event::Resized { size } => {
                if size.width > 0 && size.height > 0 {
                    self.config.config.width = size.width;
                    self.config.config.height = size.height;
                    self.globals.window_size = [size.width as f32, size.height as f32];
                    self.config
                        .surface
                        .configure(&self.config.device, &self.config.config);
                    self.text_bundle.resize(&self.config.queue, &size);
                }
            }
            Event::CursorMoved { position } => self.ctx.mouse_pos = position,
            Event::MouseInput { mouse_down } => {
                self.ctx.mouse_down = mouse_down;
                self.ctx.mouse_pressed = !prev_mouse_down && mouse_down;
                self.ctx.mouse_released = prev_mouse_down && !mouse_down;
            }
            _ => (),
        }

        _ = self.view(build, state);

        let mut should_redraw = matches!(event, Event::RedrawRequested);

        let messages = self.ctx.take();
        for msg in messages {
            let msg_event = Event::Message(msg);
            _ = update(self, &msg_event, state, params);
            should_redraw = true;
        }

        if should_redraw {
            _ = self.render();
        }
    }

    fn view<S>(
        &mut self,
        build: impl FnOnce(&S) -> Element<M>,
        state: &S,
    ) -> Result<(), &'static str> {
        let element = build(state);
        let (primitives, texts) = cascade_widgets(
            &element.widget,
            &self.globals.window_size,
            &self.textures,
            &mut self.text_bundle,
            &mut self.ctx,
        )?;

        if let Some(primitives) = primitives {
            self.primitive_bundle
                .update(&self.config.queue, &primitives);
        }
        if let Some(texts) = texts {
            self.text_bundle.update(&texts);

            self.text_bundle
                .prepare(&self.config.device, &self.config.queue, &texts);
        }
        Ok(())
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.config.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.config
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        let mut clear_color = Some(wgpu::Color::WHITE);

        self.primitive_bundle.render(
            &view,
            &mut encoder,
            &self.globals,
            &self.textures.bind_group(&self.config),
            &mut clear_color,
        );
        _ = self
            .text_bundle
            .render(&view, &mut encoder, &mut clear_color);

        self.config.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[derive(Debug)]
pub struct Config<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl<'a> Config<'a> {
    fn new<T>(target: Arc<T>, size: &Size<u32>) -> Config<'a>
    where
        T: wgpu::rwh::HasWindowHandle
            + wgpu::rwh::HasDisplayHandle
            + Sized
            + std::marker::Sync
            + std::marker::Send
            + 'a,
    {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::DEBUG,
            ..Default::default()
        });

        let surface = instance.create_surface(target.clone()).unwrap();

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let is_metal = adapter.get_info().backend == wgpu::Backend::Metal;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::PUSH_CONSTANTS
                | wgpu::Features::TEXTURE_BINDING_ARRAY
                | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
                | wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER
                | if !is_metal {
                    wgpu::Features::PARTIALLY_BOUND_BINDING_ARRAY
                } else {
                    wgpu::Features::empty()
                },
            required_limits: wgpu::Limits {
                max_push_constant_size: 128,
                max_binding_array_elements_per_shader_stage: DEFAULT_MAX_TEXTURES,
                ..Default::default()
            },
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        }))
        .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TextureHandle(pub(crate) u32);

#[derive(Debug, Copy, Clone)]
pub struct TextureInfo {
    handle: TextureHandle,
    dims: Size<u32>,
}

pub struct TextureArray {
    texture_array_layout: wgpu::BindGroupLayout,
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

    fn bind_group(&mut self, config: &Config) -> &wgpu::BindGroup {
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

    fn load_texture(
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

    fn unload_texture(&mut self, handle: TextureHandle) {
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

pub struct PrimitiveBundle {
    shader_path: PathBuf,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: Option<wgpu::Buffer>,
    instance_buffer: wgpu::Buffer,

    num_indices: u32,
    num_instances: u32,
}

impl PrimitiveBundle {
    pub fn primitive(
        config: &Config,
        texture_array_layout: &wgpu::BindGroupLayout,
        max_instances: Option<u64>,
    ) -> PrimitiveBundle {
        Self::new(
            "Primitive",
            std::path::Path::new("ui/src/shaders/primitive_shader.wgsl"),
            QUAD_VERTICES,
            QUAD_INDICES,
            max_instances.unwrap_or(DEFAULT_MAX_INSTANCES),
            config,
            texture_array_layout,
        )
    }

    pub fn new(
        name: &str,
        shader_path: &std::path::Path,
        vertices: &[Vertex],
        indices: &[u16],
        max_instances: u64,
        config: &Config,
        texture_array_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader_module = utils::wgsl::load_wgsl(&config.device, shader_path, name);

        let render_pipeline_layout =
            config
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Primitive Render Pipeline Layout"),
                    bind_group_layouts: &[texture_array_layout],
                    push_constant_ranges: &[wgpu::PushConstantRange {
                        stages: wgpu::ShaderStages::VERTEX,
                        range: 0..std::mem::size_of::<Globals>() as u32,
                    }],
                });
        let render_pipeline =
            config
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Primitive Render Pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader_module,
                        entry_point: Some("vs_main"),
                        buffers: &[Vertex::desc(), Primitive::desc()],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader_module,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: config.config.format,
                            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                    cache: None,
                });

        let vertex_buffer = config
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Primitive Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = if indices.is_empty() {
            None
        } else {
            Some(
                config
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Primitive Index Buffer"),
                        contents: bytemuck::cast_slice(indices),
                        usage: wgpu::BufferUsages::INDEX,
                    }),
            )
        };

        let instance_buffer = config.device.create_buffer(&wgpu::wgt::BufferDescriptor {
            label: Some("Primitive Instance Buffer"),
            size: std::mem::size_of::<Primitive>() as u64 * max_instances,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let num_indices = if !indices.is_empty() {
            indices.len()
        } else {
            vertices.len()
        } as u32;

        Self {
            shader_path: shader_path.to_path_buf(),
            render_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,

            num_indices,
            num_instances: 0,
        }
    }

    pub fn reload(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        texture_array_layout: &wgpu::BindGroupLayout,
    ) {
        let shader_module = utils::wgsl::load_wgsl(device, &self.shader_path, "Primitive");
        self.render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Primitive Render Pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Primitive Layout"),
                    bind_group_layouts: &[texture_array_layout],
                    push_constant_ranges: &[wgpu::PushConstantRange {
                        stages: wgpu::ShaderStages::VERTEX,
                        range: 0..std::mem::size_of::<Globals>() as u32,
                    }],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc(), Primitive::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
    }

    fn update(&mut self, queue: &wgpu::Queue, instances: &[Primitive]) {
        self.num_instances = instances.len() as u32;
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
    }

    fn render(
        &self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        globals: &Globals,
        texture_bind_group: &wgpu::BindGroup,
        clear_color: &mut Option<wgpu::Color>,
    ) {
        if self.num_instances <= 0 {
            return;
        }

        let load = if let Some(clear_color) = clear_color.take() {
            wgpu::LoadOp::Clear(clear_color)
        } else {
            wgpu::LoadOp::Load
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Primitive Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, bytemuck::bytes_of(globals));

        if let Some(index_buffer) = self.index_buffer.as_ref() {
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..self.num_instances);
        } else {
            render_pass.draw(0..self.num_indices, 0..self.num_instances);
        }
    }
}

pub struct TextBundle {
    font_system: glyphon::FontSystem,
    swash_cache: glyphon::SwashCache,
    atlas: glyphon::TextAtlas,
    viewport: glyphon::Viewport,
    text_renderer: glyphon::TextRenderer,

    buffers: Vec<glyphon::Buffer>,
    is_ready: bool,
}

impl<'a> TextBundle {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, config: &SurfaceConfiguration) -> Self {
        let mut font_system = glyphon::FontSystem::new();
        font_system.db_mut().load_system_fonts();

        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(&device);
        let mut atlas = glyphon::TextAtlas::new(&device, &queue, &cache, config.format);
        let viewport = glyphon::Viewport::new(device, &cache);
        let text_renderer = glyphon::TextRenderer::new(
            &mut atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );

        Self {
            font_system,
            swash_cache,
            atlas,
            viewport,
            text_renderer,
            buffers: Vec::new(),
            is_ready: true,
        }
    }

    fn update(&mut self, texts: &[Text]) {
        self.buffers.clear();

        for t in texts {
            let metrics = glyphon::Metrics::new(t.style.font_size, t.style.font_size * 1.4);
            let mut buf = glyphon::Buffer::new(&mut self.font_system, metrics);
            buf.set_size(
                &mut self.font_system,
                Some(t.size.width as f32),
                Some(t.size.height as f32),
            );

            let mut attrs = glyphon::Attrs::new().family(glyphon::Family::SansSerif);
            attrs = attrs.weight(t.style.weight);
            if t.style.italic {
                attrs.style(glyphon::Style::Italic);
            }
            buf.set_text(
                &mut self.font_system,
                &t.content,
                &glyphon::Attrs::new(),
                glyphon::Shaping::Advanced,
            );

            self.buffers.push(buf);
        }

        self.is_ready = false;
    }

    fn resize(&mut self, queue: &wgpu::Queue, size: &Size<u32>) {
        self.viewport.update(
            queue,
            glyphon::Resolution {
                width: size.width,
                height: size.height,
            },
        );
        self.is_ready = false;
    }

    fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, texts: &[Text]) {
        if self.is_ready {
            return;
        }

        let text_areas: Vec<_> = texts
            .iter()
            .zip(&self.buffers)
            .map(|(text, buffer)| crate::utils::glyphon::to_text(text, buffer))
            .collect();

        self.text_renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .expect("glyphon prepare failed");

        self.is_ready = true;
    }

    fn render(
        &self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        clear_color: &mut Option<wgpu::Color>,
    ) -> Result<(), glyphon::RenderError> {
        if self.buffers.is_empty() {
            return Ok(());
        }

        let load = if let Some(clear_color) = clear_color.take() {
            wgpu::LoadOp::Clear(clear_color)
        } else {
            wgpu::LoadOp::Load
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Text Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        self.text_renderer
            .render(&self.atlas, &self.viewport, &mut render_pass)
    }

    pub(crate) fn get_min_size(&mut self, style: &TextStyle, content: &str) -> Size<f32> {
        let metrics = glyphon::Metrics::new(style.font_size, style.font_size * 1.4);
        let mut buf = glyphon::Buffer::new(&mut self.font_system, metrics);

        let mut attrs = glyphon::Attrs::new().family(glyphon::Family::SansSerif);
        attrs = attrs.weight(style.weight);
        if style.italic {
            attrs = attrs.style(glyphon::Style::Italic);
        }
        buf.set_text(
            &mut self.font_system,
            &content,
            &attrs,
            glyphon::Shaping::Advanced,
        );

        let width = buf.layout_runs().map(|run| run.line_w).fold(0.0, f32::max);
        let height = buf.metrics().line_height;

        Size { width, height }
    }
}
