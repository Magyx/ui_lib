use std::sync::Arc;

use image::DynamicImage;
use wgpu::SurfaceConfiguration;

use crate::{
    consts::*,
    context::Context,
    event::{Event, ToEvent},
    model::*,
    primitive::{Primitive, PrimitiveBundle},
    text::TextBundle,
    texture::{TextureArray, TextureHandle},
    widget::{Element, RenderOutput, Text, TextStyle, Widget},
};

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
    pub(crate) config: Config<'a>,
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
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) config: wgpu::SurfaceConfiguration,
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
