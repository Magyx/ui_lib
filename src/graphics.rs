// TODO: should cache calls when no targets are attached
use std::{collections::HashMap, sync::Arc, time::Instant};

use crate::{
    consts::*,
    context::{Context, EventCtx, LayoutCtx, PaintCtx},
    event::{Event, ToEvent},
    model::*,
    primitive::{Primitive, Vertex},
    render::{
        pipeline::PipelineRegistry,
        renderer::Renderer,
        texture::{Atlas, TextureHandle},
    },
    widget::{Element, internal::PAINT_TOKEN},
};

#[derive(Default)]
struct TargetIdAlloc {
    next: u32,
}

impl TargetIdAlloc {
    fn alloc(&mut self) -> TargetId {
        let id = TargetId(self.next);
        self.next = self.next.checked_add(1).expect("TargetId overflow");
        id
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    window_size: [f32; 2], // pixels
    mouse_pos: [f32; 2],   // pixels
    mouse_buttons: u32,    // bit 0: left, bit 1: right (etc.)
    pub time: f32,         // seconds since start
    pub delta_time: f32,   // seconds since last frame
    pub frame: u32,        // frame counter
}

pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

pub struct Target<'a, M> {
    pub surface: wgpu::Surface<'a>,
    pub config: wgpu::SurfaceConfiguration,
    pub size: Size<u32>,
    pub scale: i32,
    pub globals: Globals,
    ctx: Context<M>,

    start_time: Instant,
    last_frame_time: Instant,
    root: Option<Element<M>>,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct TargetId(u32);

pub struct Engine<'a, M> {
    debug: bool,

    gpu: Arc<Gpu>,
    target_alloc: TargetIdAlloc,
    primary_target: Option<TargetId>,
    targets: HashMap<TargetId, Target<'a, M>>,
    pub(crate) push_constant_ranges: Vec<wgpu::PushConstantRange>,
    pipeline_registry: PipelineRegistry,
    renderer: Renderer,
}

impl<'a, M> Default for Engine<'a, M> {
    fn default() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: crate::consts::default_backends(),
            flags: crate::consts::default_instance_flags(),
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
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

        let gpu = Gpu {
            instance,
            adapter,
            device,
            queue,
        };

        let push_constant_ranges = vec![wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
            range: 0..std::mem::size_of::<Globals>() as u32,
        }];

        let renderer = Renderer::new(&gpu.device);
        let pipeline_registry = PipelineRegistry::new();

        let target_alloc = TargetIdAlloc::default();
        let targets = HashMap::with_capacity(1);

        Self {
            debug: false,

            gpu: Arc::new(gpu),
            target_alloc,
            primary_target: None,
            targets,
            push_constant_ranges,
            pipeline_registry,
            renderer,
        }
    }
}

impl<'a, M: std::fmt::Debug + 'static> Engine<'a, M> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_for<T>(target: Arc<T>, size: Size<u32>) -> (TargetId, Self)
    where
        T: wgpu::rwh::HasWindowHandle
            + wgpu::rwh::HasDisplayHandle
            + Sized
            + std::marker::Sync
            + std::marker::Send
            + 'a,
    {
        let mut engine = Self::new();

        let target = engine.create_target(target, size);

        (target, engine)
    }

    fn create_target<T>(&mut self, target: Arc<T>, size: Size<u32>) -> TargetId
    where
        T: wgpu::rwh::HasWindowHandle
            + wgpu::rwh::HasDisplayHandle
            + Sized
            + std::marker::Sync
            + std::marker::Send
            + 'a,
    {
        let size = size.max(Size::new(1, 1));

        let surface = self
            .gpu
            .instance
            .create_surface(target.clone())
            .expect("wgpu: failed to create surface (window/display handle mismatch?)");

        let surface_caps = surface.get_capabilities(&self.gpu.adapter);
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

        surface.configure(&self.gpu.device, &config);

        let now = Instant::now();
        let target = Target {
            surface,
            config,
            size,
            scale: 1,
            globals: Globals {
                window_size: [size.width as f32, size.height as f32],
                time: 0.0,
                delta_time: 0.0,
                mouse_pos: [0.0, 0.0],
                mouse_buttons: 0,
                frame: 0,
            },
            ctx: Context::new(),

            start_time: now,
            last_frame_time: now,

            root: None,
        };

        if !self.pipeline_registry.has_default_pipelines() {
            self.pipeline_registry.register_default_pipelines(
                &self.gpu,
                &target.config.format,
                &[Vertex::desc(), Primitive::desc()],
                self.renderer.textures.layout(),
                &self.push_constant_ranges,
            );
        }

        let tid = self.target_alloc.alloc();
        self.targets.insert(tid, target);

        if self.primary_target.is_none() {
            self.primary_target = Some(tid);
        }

        tid
    }

    #[inline]
    fn primary_target_id(&self) -> Option<TargetId> {
        self.primary_target
    }

    #[inline]
    fn primary_target(&self) -> Option<&Target<'a, M>> {
        self.primary_target_id()
            .and_then(|id| self.targets.get(&id))
    }

    pub fn reload_all(&mut self) {
        let fmt = if let Some(t) = self.primary_target() {
            t.config.format
        } else {
            return;
        };

        self.pipeline_registry.reload(
            &self.gpu,
            &fmt,
            &[Vertex::desc(), Primitive::desc()],
            self.renderer.textures.layout(),
            &self.push_constant_ranges,
        );
    }

    pub fn toggle_debug(&mut self) {
        self.debug = !self.debug;
    }

    pub fn globals(&self, tid: TargetId) -> Option<&Globals> {
        self.targets.get(&tid).map(|t| &t.globals)
    }

    pub fn attach_target<T>(&mut self, target: Arc<T>, size: Size<u32>) -> TargetId
    where
        T: wgpu::rwh::HasWindowHandle
            + wgpu::rwh::HasDisplayHandle
            + Sized
            + std::marker::Sync
            + std::marker::Send
            + 'a,
    {
        self.create_target(target, size)
    }

    pub fn detach_target(&mut self, tid: &TargetId) {
        if self.targets.remove(tid).is_some() && self.primary_target == Some(*tid) {
            if self.primary_target == Some(*tid) && !self.targets.is_empty() {
                self.primary_target = self.targets.keys().next().copied();
            } else {
                _ = self.primary_target.take();
            }
        }
    }

    pub fn register_pipeline(
        &mut self,
        key: crate::render::pipeline::PipelineKey,
        pipeline_factory: crate::render::PipelineFactoryFn,
    ) {
        let fmt = if let Some(t) = self.primary_target() {
            t.config.format
        } else {
            return; // TODO: we should definitely return a result here
        };

        let pipeline = pipeline_factory(
            &self.gpu,
            &fmt,
            &[Vertex::desc(), Primitive::desc()],
            self.renderer.textures.layout(),
            &self.push_constant_ranges,
        );
        self.pipeline_registry.register_pipeline(key, pipeline);
    }

    pub fn load_texture_rgba8(&mut self, width: u32, height: u32, pixels: &[u8]) -> TextureHandle {
        self.renderer
            .textures
            .load_rgba8(&self.gpu, width, height, pixels)
    }

    pub fn unload_texture(&mut self, handle: TextureHandle) -> bool {
        self.renderer.textures.unload(&self.gpu, handle)
    }

    pub fn create_atlas(&mut self, width: u32, height: u32) -> Atlas {
        self.renderer
            .textures
            .create_atlas(&self.gpu, width, height)
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
            .load_into_atlas(&self.gpu, atlas, width, height, pixels)
    }

    pub fn destroy_atlas(&mut self, atlas: &mut Atlas) {
        self.renderer.textures.destroy_atlas(&self.gpu, atlas)
    }

    pub fn poll<S, P, E: ToEvent<M, E> + std::fmt::Debug>(
        &mut self,
        tid: &TargetId,
        update: &mut impl FnMut(&mut Self, &Event<M, E>, &mut S, &P) -> bool,
        state: &mut S,
        params: &P,
    ) -> bool {
        let target = if let Some(t) = self.targets.get_mut(tid) {
            t
        } else {
            return false;
        };

        let now = std::time::Instant::now();
        let total = now.duration_since(target.start_time);
        let dt = now.duration_since(target.last_frame_time);
        target.last_frame_time = now;
        target.globals.time = total.as_secs_f32();
        target.globals.delta_time = dt.as_secs_f32();

        let mut require_redraw = false;

        if let Some(root) = target.root.as_mut() {
            let mut event_cx = EventCtx {
                globals: &target.globals,
                ui: &mut target.ctx,
            };
            root.handle(&mut event_cx);
        } else {
            require_redraw = true;
        }

        require_redraw |= target.ctx.take_redraw();

        for message in target.ctx.take() {
            require_redraw |= update(self, &Event::Message(message), state, params);
        }

        require_redraw |= update(self, &Event::RedrawRequested, state, params);

        require_redraw
    }

    pub fn render_if_needed<S>(
        &mut self,
        tid: &TargetId,
        need: bool,
        view: &impl Fn(&TargetId, &S) -> Element<M>,
        state: &mut S,
    ) {
        let target = if let Some(t) = self.targets.get_mut(tid) {
            t
        } else {
            return; // TODO: maybe return a result instead
        };

        if !need {
            return;
        }

        // TODO: this should eventually be removed, as it is not accurate way to have id's
        // maybe move to a depth based id system where id is passed from context instead of
        // generated in each widget
        crate::context::reset_ids_for_frame();

        target.root = Some(view(tid, state));
        let root = target.root.as_mut().expect("root built");

        let max = Size::new(
            target.globals.window_size[0] as i32,
            target.globals.window_size[1] as i32,
        )
        .max(Size::new(1, 1));

        {
            let mut layout_ctx = LayoutCtx {
                globals: &target.globals,
                ui: &mut target.ctx,
                text: &mut self.renderer.text,
            };
            _ = root.fit_width(&mut layout_ctx);
            root.grow_width(&mut layout_ctx, max.width);

            _ = root.fit_height(&mut layout_ctx);
            root.grow_height(&mut layout_ctx, max.height);

            root.place(&mut layout_ctx, Position::splat(0));
        }

        let mut event_ctx = EventCtx {
            globals: &target.globals,
            ui: &mut target.ctx,
        };

        // TODO: split handle into prepare and other steps so we don't need to force a take_redraw
        root.handle(&mut event_ctx);
        target.ctx.take_redraw();

        let mut instances = Vec::new();
        {
            let mut paint_ctx = PaintCtx {
                globals: &target.globals,
                text: &mut self.renderer.text,
                gpu: &self.gpu.clone(),
                texture: &mut self.renderer.textures,
            };
            root.__paint(&mut paint_ctx, &mut instances, &PAINT_TOKEN, self.debug);
        }

        target.globals.frame = target.globals.frame.wrapping_add(1);

        let _ = self.renderer.render(
            &self.gpu,
            target,
            &self.pipeline_registry,
            &target.globals,
            &instances,
        );
    }

    pub fn handle_platform_event<S, P, E: ToEvent<M, E> + std::fmt::Debug>(
        &mut self,
        target_id: &TargetId,
        event: &E,
        update: &mut impl FnMut(&mut Self, &Event<M, E>, &mut S, &P) -> bool,
        state: &mut S,
        params: &P,
    ) {
        let target = match self.targets.get_mut(target_id) {
            Some(t) => t,
            None => {
                return; // TODO: maybe return a result instead
            }
        };

        let event = event.to_event();
        let prev_mouse_down = target.ctx.mouse_down;

        match event {
            Event::Resized { size } => {
                if size.width > 0 && size.height > 0 {
                    target.config.width = size.width;
                    target.config.height = size.height;
                    target.globals.window_size = [size.width as f32, size.height as f32];
                    target.surface.configure(&self.gpu.device, &target.config);
                }
                target.ctx.request_redraw();
            }
            Event::CursorMoved { position } => {
                target.ctx.mouse_pos = position;
                target.globals.mouse_pos = [position.x, position.y];
            }
            Event::MouseInput { mouse_down } => {
                target.ctx.mouse_down = mouse_down;
                target.ctx.mouse_pressed = !prev_mouse_down && mouse_down;
                target.ctx.mouse_released = prev_mouse_down && !mouse_down;

                if mouse_down {
                    target.globals.mouse_buttons |= 1;
                } else {
                    target.globals.mouse_buttons &= !1;
                }
            }
            _ => (),
        }

        if update(self, &event, state, params)
            && let Some(target) = self.targets.get_mut(target_id)
        {
            target.ctx.request_redraw();
        }
    }
}
