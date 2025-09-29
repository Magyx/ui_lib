use std::{sync::Arc, time::Instant};

use crate::{
    consts::*,
    context::{Context, EventCtx, FitCtx, GrowCtx, PaintCtx, PlaceCtx},
    event::{Event, ToEvent},
    model::*,
    primitive::{Primitive, Vertex},
    render::{
        pipeline::PipelineRegistry,
        renderer::Renderer,
        texture::{Atlas, TextureHandle},
    },
    widget::Element,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    window_size: [f32; 2], // pixels
    pub time: f32,         // seconds since start
    pub delta_time: f32,   // seconds since last frame
    mouse_pos: [f32; 2],   // pixels
    mouse_buttons: u32,    // bit 0: left, bit 1: right (etc.)
    pub frame: u32,        // frame counter
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

        let renderer = Renderer::new(&config);

        let mut pipeline_registry = PipelineRegistry::new();
        pipeline_registry.register_default_pipelines(
            &config,
            &[Vertex::desc(), Primitive::desc()],
            renderer.textures.layout(),
            &push_constant_ranges,
        );

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
        self.pipeline_registry.reload(
            &self.config,
            &[Vertex::desc(), Primitive::desc()],
            self.renderer.textures.layout(),
            &self.push_constant_ranges,
        );
    }

    pub fn register_pipeline(
        &mut self,
        key: crate::render::pipeline::PipelineKey,
        pipeline_factory: crate::render::PipelineFactoryFn,
    ) {
        let pipeline = pipeline_factory(
            &self.config,
            &[Vertex::desc(), Primitive::desc()],
            self.renderer.textures.layout(),
            &self.push_constant_ranges,
        );
        self.pipeline_registry.register_pipeline(key, pipeline);
    }

    pub fn load_texture_rgba8(&mut self, width: u32, height: u32, pixels: &[u8]) -> TextureHandle {
        self.renderer
            .textures
            .load_rgba8(&self.config, width, height, pixels)
    }

    pub fn unload_texture(&mut self, handle: TextureHandle) -> bool {
        self.renderer.textures.unload(&self.config, handle)
    }

    pub fn create_atlas(&mut self, width: u32, height: u32) -> Atlas {
        self.renderer
            .textures
            .create_atlas(&self.config, width, height)
    }

    pub fn load_texture_into_atlas(
        &mut self,
        atlas: &mut Atlas,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Option<TextureHandle> {
        self.renderer
            .textures
            .load_into_atlas(&self.config, atlas, width, height, pixels)
    }

    pub fn destroy_atlas(&mut self, atlas: &mut Atlas) {
        self.renderer.textures.destroy_atlas(&self.config, atlas)
    }

    pub fn poll<S, P, E: ToEvent<M, E> + std::fmt::Debug>(
        &mut self,
        update: &mut impl FnMut(&mut Self, &Event<M, E>, &mut S, &P) -> bool,
        state: &mut S,
        params: &P,
    ) -> bool {
        let now = std::time::Instant::now();
        let total = now.duration_since(self.start_time);
        let dt = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;
        self.globals.time = total.as_secs_f32();
        self.globals.delta_time = dt.as_secs_f32();

        let mut require_redraw = false;

        if let Some(root) = self.root.as_mut() {
            let mut event_cx = EventCtx {
                globals: &self.globals,
                ui: &mut self.ctx,
            };
            root.handle(&mut event_cx);
        } else {
            require_redraw = true;
        }

        require_redraw |= self.ctx.take_redraw();

        for message in self.ctx.take() {
            require_redraw |= update(self, &Event::Message(message), state, params);
        }

        require_redraw |= update(self, &Event::RedrawRequested, state, params);

        require_redraw
    }

    pub fn render_if_needed<S>(
        &mut self,
        need: bool,
        view: &impl Fn(&S) -> Element<M>,
        state: &mut S,
    ) {
        if !need {
            return;
        }

        // TODO: this should eventually be removed, as it is not accurate way to have id's
        // maybe move to a depth based id system where id is passed from context instead of
        // generated in each widget
        crate::context::reset_ids_for_frame();

        self.root = Some(view(state));
        let root = self.root.as_mut().expect("root built");

        let max = Size::new(
            self.globals.window_size[0] as i32,
            self.globals.window_size[1] as i32,
        )
        .max(Size::new(1, 1));

        {
            let mut fit_cx = FitCtx {
                globals: &self.globals,
                ui: &mut self.ctx,
            };
            let _ = root.fit_size(&mut fit_cx);
        }
        {
            let mut grow_cx = GrowCtx {
                globals: &self.globals,
                ui: &mut self.ctx,
            };
            root.grow_size(&mut grow_cx, max);
        }
        {
            let mut place_cx = PlaceCtx {
                globals: &self.globals,
                ui: &mut self.ctx,
            };
            root.place(&mut place_cx, Position::splat(0));
        }

        let mut event_cx = EventCtx {
            globals: &self.globals,
            ui: &mut self.ctx,
        };
        root.handle(&mut event_cx);

        self.ctx.take_redraw();

        let mut instances = Vec::new();
        {
            let mut paint_cx = PaintCtx {
                globals: &self.globals,
            };
            root.draw(&mut paint_cx, &mut instances);
        }

        self.globals.frame = self.globals.frame.wrapping_add(1);

        let _ = self.renderer.render(
            &self.config,
            &self.pipeline_registry,
            &self.globals,
            &instances,
        );
    }

    pub fn handle_platform_event<S, P, E: ToEvent<M, E> + std::fmt::Debug>(
        &mut self,
        event: &E,
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
                self.ctx.request_redraw();
            }
            Event::CursorMoved { position } => {
                self.ctx.mouse_pos = position;
                self.globals.mouse_pos = [position.x, position.y];
            }
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

        if update(self, &event, state, params) {
            self.ctx.request_redraw();
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
            flags: crate::consts::default_instance_flags(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(target.clone())
            .expect("wgpu: failed to create surface (window/display handle mismatch?)");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("wgpu: no suitable adapter found for the current surface");

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
        .expect("wgpu: failed to request logical device/queue (feature set unsupported?)");

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
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
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
