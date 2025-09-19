use std::{sync::Arc, time::Instant};

use crate::{
    consts::*,
    context::Context,
    event::{Event, ToEvent},
    model::*,
    render::{pipeline::PipelineRegistry, renderer::Renderer},
    widget::Element,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    window_size: [f32; 2], // pixels
    time: f32,             // seconds since start
    delta_time: f32,       // seconds since last frame
    mouse_pos: [f32; 2],   // pixels
    mouse_buttons: u32,    // bit 0: left, bit 1: right (etc.)
    frame: u32,            // frame counter
}

pub struct Engine<'a, M> {
    globals: Globals,
    pub(crate) config: Config<'a>,
    ctx: Context<M>,

    start_time: Instant,
    last_frame_time: Instant,

    pub(crate) push_constant_ranges: Vec<wgpu::PushConstantRange>,
    pipeline_registry: PipelineRegistry,
    renderer: Renderer,

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
        let size = size.max(Size::new(1, 1));
        let config = Config::new(target, &size);
        let ctx = Context::new();

        let push_constant_ranges = vec![wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
            range: 0..std::mem::size_of::<Globals>() as u32,
        }];

        let mut pipeline_registry = PipelineRegistry::new();
        pipeline_registry.register_default_pipelines(&config, &push_constant_ranges);

        let renderer = Renderer::new(&config);

        let now = Instant::now();

        Self {
            globals: Globals {
                window_size: [size.width as f32, size.height as f32],
                time: 0.0,
                delta_time: 0.0,
                mouse_pos: [0.0, 0.0],
                mouse_buttons: 0,
                frame: 0,
            },
            config,
            ctx,

            start_time: now,
            last_frame_time: now,

            push_constant_ranges,
            pipeline_registry,
            renderer,

            root: None,
        }
    }

    pub fn reload_all(&mut self) {
        self.pipeline_registry
            .reload(&self.config, &self.push_constant_ranges);
    }

    pub fn register_pipeline(
        &mut self,
        key: crate::render::pipeline::PipelineKey,
        pipeline: Box<dyn crate::render::pipeline::Pipeline>,
    ) {
        self.pipeline_registry.register_pipeline(key, pipeline);
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

                if mouse_down {
                    self.globals.mouse_buttons |= 1;
                } else {
                    self.globals.mouse_buttons &= !1;
                }
            }
            _ => (),
        }

        let mut require_redraw = matches!(event, Event::Resized { .. } | Event::RedrawRequested);

        let max = Size::new(
            self.globals.window_size[0] as i32,
            self.globals.window_size[1] as i32,
        )
        .max(Size::new(1, 1));

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

            let mut instances = Vec::new();
            root.draw(&mut instances);

            let now = Instant::now();
            let total = now.duration_since(self.start_time);
            let dt = now.duration_since(self.last_frame_time);
            self.last_frame_time = now;

            self.globals.time = total.as_secs_f32();
            self.globals.delta_time = dt.as_secs_f32();
            self.globals.frame = self.globals.frame.wrapping_add(1);

            _ = self.renderer.render(
                &self.config,
                &self.pipeline_registry,
                &self.globals,
                &instances,
            );
        }
    }
}

#[derive(Debug)]
pub struct Config<'a> {
    pub(crate) surface: wgpu::Surface<'a>,
    pub(crate) queue: wgpu::Queue,
    pub device: wgpu::Device,
    pub config: wgpu::SurfaceConfiguration,
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
            flags: wgpu::InstanceFlags::DEBUG, // TODO: make this configurable
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
            queue,
            device,
            config,
        }
    }
}
