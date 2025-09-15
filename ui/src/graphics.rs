use std::sync::Arc;

use crate::{
    consts::*,
    context::Context,
    event::{Event, ToEvent},
    model::*,
    primitive::PrimitiveBundle,
    widget::Element,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    window_size: [f32; 2],
}

pub struct Engine<'a, M> {
    globals: Globals,
    config: Config<'a>,
    ctx: Context<M>,

    primitive_bundle: PrimitiveBundle,

    root: Option<Element<M>>,
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
        let config = Config::new(target, &size);
        let ctx = Context::new();

        let primitive_bundle = PrimitiveBundle::primitive(&config, None);

        Self {
            globals: Globals {
                window_size: [size.width as f32, size.height as f32],
            },
            config,
            ctx,

            primitive_bundle,

            root: None,
        }
    }

    pub fn reload_all(&mut self) {
        self.primitive_bundle
            .reload(&self.config.device, self.config.config.format);
    }

    pub fn handle_event<S, P, E: ToEvent<M, E> + std::fmt::Debug>(
        &mut self,
        event: &E,
        view: impl Fn(&S) -> Element<M>,
        update: &mut impl FnMut(&mut Self, &Event<M, E>, &mut S, &P) -> bool,
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

        let mut require_redraw = matches!(event, Event::Resized { .. } | Event::RedrawRequested);

        let max = Size::new(
            self.globals.window_size[0] as i32,
            self.globals.window_size[1] as i32,
        );

        if self.root.is_some() {
            let root = self.root.as_mut().unwrap();
            if require_redraw {
                _ = root.fit_size();
                root.grow_size(max);
                root.place(Position::splat(0));
            }
            root.handle(&mut self.ctx);

            require_redraw |= self.ctx.take_redraw();

            for message in self.ctx.take() {
                require_redraw |= update(self, &Event::Message(message), state, params);
            }
        }

        require_redraw |= update(self, &event, state, params);

        if require_redraw {
            crate::context::reset_ids_for_frame();
            self.root = Some(view(state));

            let root = self.root.as_mut().unwrap();
            _ = root.fit_size();
            root.grow_size(max);
            root.place(Position::splat(0));
            root.handle(&mut self.ctx);

            let mut prims = Vec::new();
            root.draw(&mut prims);
            self.primitive_bundle.update(&self.config.queue, &prims);

            let _ = self.render();
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = match self.config.surface.get_current_texture() {
            Ok(o) => o,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.config
                    .surface
                    .configure(&self.config.device, &self.config.config);
                self.config.surface.get_current_texture()?
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(e) => return Err(e),
        };

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

        self.primitive_bundle
            .render(&view, &mut encoder, &self.globals, &mut clear_color);

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
            backends: crate::consts::default_backends(),
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
