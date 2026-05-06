// TODO: should cache calls when no targets are attached
use std::{collections::HashMap, sync::Arc, time::Instant};

use crate::{
    consts::*,
    context::{Context, EventCtx, LayoutCtx, PaintCtx, PrepareCtx},
    event::{Event, KeyState, ToEvent},
    layout::{self, LayoutEngine},
    model::*,
    primitive::{Instance, Primitive, Vertex},
    render::{
        pipeline::PipelineRegistry,
        renderer::Renderer,
        texture::{Atlas, TextureHandle},
    },
    widget::Element,
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
    pub window_size: [f32; 2], // pixels
    pub mouse_pos: [f32; 2],   // pixels
    pub mouse_buttons: u32,    // bit 0: left, bit 1: right (etc.)
    pub time: f32,             // seconds since start
    pub delta_time: f32,       // seconds since last frame
    pub frame: u32,            // frame counter
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
    instance_buf: Vec<Instance>,
    primitive_buf: Vec<Primitive>,
    layout_engine: LayoutEngine,

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
        #[cfg(feature = "tracing")]
        device.on_uncaptured_error(Box::new(|err| {
            tracing::warn!("wgpu uncaptured error: {err}");
        }));

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
            layout_engine: LayoutEngine::new(),
            instance_buf: Vec::new(),
            primitive_buf: Vec::new(),

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

        // TODO: should add configurability
        let surface_caps = surface.get_capabilities(&self.gpu.adapter);
        let format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);

        let present_mode = if surface_caps
            .present_modes
            .contains(&wgpu::PresentMode::AutoVsync)
        {
            wgpu::PresentMode::AutoVsync
        } else {
            surface_caps
                .present_modes
                .first()
                .copied()
                .unwrap_or(wgpu::PresentMode::Fifo)
        };

        let alpha_mode = if surface_caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
        {
            wgpu::CompositeAlphaMode::PreMultiplied
        } else if surface_caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::Inherit)
        {
            wgpu::CompositeAlphaMode::Inherit
        } else {
            surface_caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::PreMultiplied)
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode,
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
        self.layout_engine.toggle_debug();
    }

    pub fn globals(&self, tid: &TargetId) -> Option<&Globals> {
        self.targets.get(tid).map(|t| &t.globals)
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
    pub fn update_texture_rgba8(&mut self, handle: TextureHandle, pixels: &[u8]) -> bool {
        self.renderer
            .textures
            .update_rgba8(&self.gpu, handle, pixels)
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
        crate::scope!("Engine::poll");

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

        crate::plot!("ui.dt_ms", (target.globals.delta_time as f64) * 1000.0);

        let mut require_redraw = false;

        if let Some(root) = target.root.as_mut() {
            let mut event_cx = EventCtx {
                text: &mut self.renderer.text,
                event: None,
                globals: &target.globals,
                ui: &mut target.ctx,
                layout: &self.layout_engine,
                current_node: 0usize,
            };
            let mut cursor = 0usize;
            layout::handle_tree(root.as_mut(), &mut event_cx, &mut cursor);
            target.ctx.mouse_buttons_pressed = 0;
            target.ctx.mouse_buttons_released = 0;
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
        if !need {
            return;
        }

        // TODO: need to invalidate and cleanup target.view_state
        let Some(target) = self.targets.get_mut(tid) else {
            return; // TODO: maybe return a result instead
        };

        crate::scope!("Engine::render");

        target.root = Some(view(tid, state));
        let root = target.root.as_mut().unwrap();

        let max = Size::new(
            target.globals.window_size[0] as i32,
            target.globals.window_size[1] as i32,
        )
        .max(Size::new(1, 1));

        let root_id = {
            crate::scope!("layout");
            let mut layout_ctx = LayoutCtx {
                globals: &target.globals,
                ui: &mut target.ctx,
                text: &mut self.renderer.text,
            };
            layout::run_layout(
                &mut self.layout_engine,
                &mut layout_ctx,
                root.as_mut(),
                max.width,
                max.height,
            )
        };

        {
            crate::scope!("prepare");
            let mut prepare_ctx = PrepareCtx {
                globals: &target.globals,
                text: &mut self.renderer.text,
                gpu: &self.gpu.clone(),
                texture: &mut self.renderer.textures,
                view_state: &mut target.ctx.view_state,
                layout: &self.layout_engine,
                current_node: root_id,
            };
            let mut cursor = root_id;
            layout::prepare_tree(root.as_mut(), &mut prepare_ctx, &mut cursor);
        }

        {
            crate::scope!("paint");
            let mut paint_ctx = PaintCtx {
                globals: &target.globals,
                text: &self.renderer.text,
                view_state: &mut target.ctx.view_state,
                layout: &self.layout_engine,
                current_node: root_id,
            };

            let mut cursor = root_id;
            let screen_clip = Some([
                0,
                0,
                target.globals.window_size[0] as i32,
                target.globals.window_size[1] as i32,
            ]);
            self.instance_buf.clear();
            layout::paint_tree(
                root.as_mut(),
                &mut paint_ctx,
                &self.layout_engine,
                &mut cursor,
                &mut self.instance_buf,
                screen_clip,
            );

            self.primitive_buf.clear();
            self.primitive_buf
                .extend(self.instance_buf.iter().map(|i| i.primitive));
        }

        crate::plot!("ui.instances", self.instance_buf.len() as f64);
        crate::plot!("ui.nodes", self.layout_engine.node_count as f64);

        target.globals.frame = target.globals.frame.wrapping_add(1);

        let _ = self.renderer.render(
            &self.gpu,
            target,
            &mut self.pipeline_registry,
            &target.globals,
            &self.instance_buf,
            &self.primitive_buf,
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
        target.ctx.mouse_buttons_pressed = 0;
        target.ctx.mouse_buttons_released = 0;

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
            Event::MouseInput { button, state } => {
                let bit = 1u32 << button.bit();
                match state {
                    KeyState::Pressed => {
                        target.ctx.mouse_buttons_down |= bit;
                        target.ctx.mouse_buttons_pressed |= bit;
                        target.globals.mouse_buttons |= bit;
                    }
                    KeyState::Released => {
                        target.ctx.mouse_buttons_down &= !bit;
                        target.ctx.mouse_buttons_released |= bit;
                        target.globals.mouse_buttons &= !bit;
                    }
                }
            }
            _ => (),
        }

        if let Some(root) = target.root.as_mut() {
            use crate::event::UiEventRef as Ui;
            let ev_view = match &event {
                Event::RedrawRequested => Some(Ui::RedrawRequested),
                Event::Resized { size } => Some(Ui::Resized { size: *size }),
                Event::CursorMoved { position } => Some(Ui::CursorMoved {
                    position: *position,
                }),
                Event::MouseInput { button, state } => Some(Ui::MouseButton {
                    button: *button,
                    state: *state,
                }),
                Event::MouseWheel(d) => Some(Ui::MouseWheel(*d)),
                Event::Key(k) => Some(Ui::Key(k)),
                Event::Text(t) => Some(Ui::Text(t)),
                Event::ModifiersChanged(m) => Some(Ui::ModifiersChanged(m)),
                _ => None,
            };

            if ev_view.is_some() {
                let mut ctx = EventCtx {
                    text: &mut self.renderer.text,
                    globals: &target.globals,
                    ui: &mut target.ctx,
                    event: ev_view,
                    layout: &self.layout_engine,
                    current_node: 0usize,
                };
                let mut cursor = 0usize;
                layout::handle_tree(root.as_mut(), &mut ctx, &mut cursor);
            }
        }

        let had_target = self.targets.contains_key(target_id);
        if had_target
            && update(self, &event, state, params)
            && let Some(target) = self.targets.get_mut(target_id)
        {
            target.ctx.request_redraw();
        }
    }
}
