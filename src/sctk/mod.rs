use std::{
    any::Any,
    collections::HashMap,
    fmt::Debug,
    ptr::NonNull,
    sync::{Arc, Mutex, atomic::AtomicBool},
};

use crate::{
    event::{Event, KeyEvent, KeyLocation, KeyState, Modifiers, PhysicalKey, ToEvent},
    graphics::{Engine, TargetId},
    model::{Position, Size},
    render::PipelineFactoryFn,
    widget::Element,
};
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::client::{Connection, QueueHandle, globals::registry_queue_init},
    registry::RegistryState,
    seat::SeatState,
    session_lock::SessionLockState,
    shell::{wlr_layer::LayerShell, xdg::XdgShell},
};
use wayland_client::{Proxy, protocol::wl_surface::WlSurface};

pub use smithay_client_toolkit::shell::{
    wlr_layer::{Anchor, KeyboardInteractivity, Layer},
    xdg::window::WindowDecorations,
};

pub mod adapter;
mod erased;
pub mod handler;
mod helpers;
pub mod msg;
pub mod state;

// === Public API ================================================================================

#[derive(Clone, Debug)]
pub enum OutputSet {
    /// Use single-output selector
    One(OutputSelector),
    /// Mirror the surface to every compositor output
    All,
    /// Explicit list
    List(Vec<OutputSelector>),
}

#[derive(Clone, Debug)]
pub enum OutputSelector {
    /// First output in SCTK’s list (current behavior)
    First,
    /// Nth output (0-based)
    Index(usize),
    /// Choose the output whose info.name/model/make starts with this string
    NamePrefix(String),
    /// Prefer laptop panel-ish names (eDP, LVDS), fall back to First
    InternalPrefer,
    /// Pick the output with the highest reported scale factor
    HighestScale,
}

/// Options describing the layer-shell surface (instead of winit's WindowAttributes).
#[derive(Clone, Debug)]
pub struct LayerOptions {
    pub layer: Layer,
    pub size: Size<u32>,
    pub anchors: Anchor,
    /// Negative means "auto" (no reservation). Positive reserves screen space (e.g. status bar).
    pub exclusive_zone: i32,
    pub keyboard_interactivity: KeyboardInteractivity,
    /// Namespace, useful for compositor rules.
    pub namespace: Option<String>,
    pub output: Option<OutputSet>,
}

impl Default for LayerOptions {
    fn default() -> Self {
        Self {
            layer: Layer::Top,
            size: Size::new(640, 360),
            anchors: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
            exclusive_zone: -1,
            keyboard_interactivity: KeyboardInteractivity::None,
            namespace: Some("ui".to_string()),
            output: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct XdgOptions {
    pub size: Size<u32>,
    pub title: String,
    pub app_id: Option<String>,
    pub decorations: WindowDecorations,
    pub output: Option<OutputSelector>,
}

impl Default for XdgOptions {
    fn default() -> Self {
        Self {
            size: Size::new(640, 360),
            title: "my_app".to_string(),
            app_id: Some("ui".to_string()),
            decorations: WindowDecorations::RequestClient,
            output: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Options {
    Layer(LayerOptions),
    Xdg(XdgOptions),
}

/// Platform event type for the SCTK backend.
#[derive(Debug, Clone)]
pub enum SctkEvent {
    Redraw,
    Resized {
        surface: SurfaceId,
        size: Size<u32>,
    },
    PointerMoved {
        surface: SurfaceId,
        pos: Position<f32>,
    },
    PointerDown {
        surface: SurfaceId,
    },
    PointerUp {
        surface: SurfaceId,
    },

    Key {
        surface: SurfaceId,
        raw_code: u32,
        keysym: smithay_client_toolkit::seat::keyboard::Keysym,
        utf8: Option<String>,
        pressed: bool,
        repeat: bool,
    },

    Modifiers(SurfaceId, smithay_client_toolkit::seat::keyboard::Modifiers),
    Closed,
    Message(Arc<Mutex<Option<Box<dyn Any + Send>>>>),
}

impl SctkEvent {
    pub fn message<M: Send + 'static>(m: M) -> Self {
        SctkEvent::Message(Arc::new(Mutex::new(Some(Box::new(m)))))
    }

    pub fn surface_id(&self) -> Option<SurfaceId> {
        match self {
            SctkEvent::Resized { surface, .. }
            | SctkEvent::PointerMoved { surface, .. }
            | SctkEvent::PointerDown { surface }
            | SctkEvent::PointerUp { surface }
            | SctkEvent::Key { surface, .. }
            | SctkEvent::Modifiers(surface, ..) => Some(*surface),
            _ => None,
        }
    }
}

impl<M: 'static + Send> ToEvent<M, SctkEvent> for SctkEvent {
    fn to_event(&self) -> Event<M, SctkEvent> {
        match self {
            SctkEvent::Redraw => Event::RedrawRequested,
            SctkEvent::Resized { size, .. } => Event::Resized { size: *size },
            SctkEvent::PointerMoved { pos, .. } => Event::CursorMoved { position: *pos },
            SctkEvent::PointerDown { .. } => Event::MouseInput { mouse_down: true },
            SctkEvent::PointerUp { .. } => Event::MouseInput { mouse_down: false },

            SctkEvent::Key {
                raw_code,
                keysym,
                utf8,
                pressed,
                repeat,
                ..
            } => {
                let state = if *pressed {
                    KeyState::Pressed
                } else {
                    KeyState::Released
                };
                let logical_key = helpers::map_keysym_to_logical(*keysym, utf8.as_deref());
                let physical_key = PhysicalKey::Code(*raw_code);

                Event::Key(KeyEvent {
                    state,
                    repeat: *repeat,
                    logical_key,
                    physical_key,
                    location: KeyLocation::Standard,
                    modifiers: Modifiers::default(),
                })
            }

            SctkEvent::Modifiers(_, m) => Event::ModifiersChanged(Modifiers {
                shift: m.shift,
                control: m.ctrl,
                alt: m.alt,
                super_: m.logo,
                caps_lock: Some(m.caps_lock),
                num_lock: Some(m.num_lock),
            }),

            SctkEvent::Closed => Event::Platform(SctkEvent::Closed),

            SctkEvent::Message(slot) => {
                if let Some(m) = slot.lock().unwrap().take() {
                    if let Ok(m) = m.downcast::<M>() {
                        Event::Message(*m)
                    } else {
                        Event::Platform(SctkEvent::Message(slot.clone()))
                    }
                } else {
                    Event::Platform(SctkEvent::Message(slot.clone()))
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SurfaceId(u32);

#[derive(Default)]
pub struct SctkLoop {
    exit: AtomicBool,
}

impl SctkLoop {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn exit(&self) {
        self.exit.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn should_exit(&self) -> bool {
        self.exit.load(std::sync::atomic::Ordering::Relaxed)
    }
}

pub struct DefaultHandler;

impl<M> handler::SctkHandler<M> for DefaultHandler {}

#[derive(Clone, Debug)]
pub struct RawWaylandHandles {
    display: NonNull<std::ffi::c_void>,
    surface: NonNull<std::ffi::c_void>,
}

impl RawWaylandHandles {
    pub fn new(conn: &Connection, wl_surface: &WlSurface) -> Self {
        let display = NonNull::new(conn.display().id().as_ptr().cast()).expect("null wl_display");
        let surface = NonNull::new(wl_surface.id().as_ptr().cast()).expect("null wl_surface");
        Self { display, surface }
    }
}

unsafe impl Send for RawWaylandHandles {}
unsafe impl Sync for RawWaylandHandles {}

impl wgpu::rwh::HasWindowHandle for RawWaylandHandles {
    fn window_handle(&self) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError> {
        let wl = wgpu::rwh::WaylandWindowHandle::new(self.surface);
        Ok(unsafe { wgpu::rwh::WindowHandle::borrow_raw(wgpu::rwh::RawWindowHandle::from(wl)) })
    }
}
impl wgpu::rwh::HasDisplayHandle for RawWaylandHandles {
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
        let wl = wgpu::rwh::WaylandDisplayHandle::new(self.display);
        Ok(unsafe { wgpu::rwh::DisplayHandle::borrow_raw(wgpu::rwh::RawDisplayHandle::from(wl)) })
    }
}

fn run_app_core<'a, M, S, V, U, H, F>(
    mut state: S,
    view: V,
    mut update: U,
    opts: Options,
    post_engine_init: F,
) -> anyhow::Result<()>
where
    M: 'static + std::fmt::Debug + Clone + Send,
    V: Fn(&TargetId, &S) -> Element<M> + 'static,
    U: FnMut(TargetId, &mut Engine<'a, M>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> bool
        + 'static,
    H: handler::SctkHandler<M> + 'static,
    F: FnOnce(&mut Engine<'a, M>),
{
    // 1) Wayland connection + queue
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh: QueueHandle<state::SctkState> = event_queue.handle();

    // 2) Bind globals
    let registry = RegistryState::new(&globals);
    let compositor = CompositorState::bind(&globals, &qh)?;
    let outputs = OutputState::new(&globals, &qh);
    let seats = SeatState::new(&globals, &qh);
    let session_lock = SessionLockState::new(&globals, &qh);

    let (tx, rx) = calloop::channel::channel();
    let handler_tx = tx.clone();
    let sctk_handler = adapter::erase::<H, M, _>(move |m| {
        let _ = handler_tx.send(SctkEvent::message(m));
    });

    // 3) Concrete SCTK state
    let mut st = match opts {
        Options::Layer(layer_options) => {
            let layer_shell = LayerShell::bind(&globals, &qh)?;
            state::SctkState::new_for_layer(
                &qh,
                layer_options,
                compositor,
                layer_shell,
                outputs,
                seats,
                registry,
                session_lock,
                sctk_handler,
                tx,
            )?
        }
        Options::Xdg(xdg_options) => {
            let xdg_shell = XdgShell::bind(&globals, &qh)?;
            state::SctkState::new_for_window(
                &qh,
                xdg_options,
                compositor,
                xdg_shell,
                outputs,
                seats,
                registry,
                session_lock,
                sctk_handler,
                tx,
            )?
        }
    };

    // 4) Create engine and attach surfaces
    let mut sid_to_tid = HashMap::new();
    let mut engine = {
        let sid = st
            .surfaces
            .keys()
            .next()
            .expect("At least one surface required");
        let target = Arc::new(RawWaylandHandles::new(&conn, &st.surfaces[sid].wl_surface));
        let (tid, mut engine) = Engine::new_for(target, st.surfaces[sid].size);
        post_engine_init(&mut engine);
        sid_to_tid.insert(*sid, tid);

        for (&sid, rec) in st.surfaces.iter().skip(1) {
            let target = Arc::new(RawWaylandHandles::new(&conn, &rec.wl_surface));
            let tid = engine.attach_target(target, rec.size);
            sid_to_tid.insert(sid, tid);
        }
        engine
    };

    let loop_ctl = SctkLoop::default();

    // 5) Main loop
    while !loop_ctl.should_exit() && !st.closed {
        event_queue.blocking_dispatch(&mut st)?;

        while let Ok(ev) = rx.try_recv() {
            match ev.surface_id() {
                Some(sid) => {
                    if let Some(tid) = sid_to_tid.get(&sid).copied() {
                        engine.handle_platform_event(
                            &tid,
                            &ev,
                            &mut |eng, e, s, ctl| update(tid, eng, e, s, ctl),
                            &mut state,
                            &loop_ctl,
                        );
                    }
                }
                None => {
                    for &tid in sid_to_tid.values() {
                        engine.handle_platform_event(
                            &tid,
                            &ev,
                            &mut |engine, event, state, loop_ctl| {
                                update(tid, engine, event, state, loop_ctl)
                            },
                            &mut state,
                            &loop_ctl,
                        );
                    }
                }
            }
        }

        for (_, &tid) in sid_to_tid.iter() {
            let need = if st.needs_redraw {
                true
            } else {
                engine.poll(
                    &tid,
                    &mut |eng, e, s, ctl| update(tid, eng, e, s, ctl),
                    &mut state,
                    &loop_ctl,
                )
            };
            engine.render_if_needed(&tid, need, &view, &mut state);
        }
        st.needs_redraw = false;
    }

    Ok(())
}

pub fn run_layer<'a, M, S, H, V, U>(
    state: S,
    view: V,
    update: U,
    opts: LayerOptions,
) -> anyhow::Result<()>
where
    M: 'static + std::fmt::Debug + Clone + Send,
    H: handler::SctkHandler<M> + 'static,
    V: Fn(&TargetId, &S) -> Element<M> + 'static,
    U: FnMut(TargetId, &mut Engine<'a, M>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> bool
        + 'static,
{
    run_app_core::<M, S, V, U, H, _>(state, view, update, Options::Layer(opts), |_| {})
}

pub fn run_layer_with<'a, M, S, H, V, U, I>(
    state: S,
    view: V,
    update: U,
    opts: LayerOptions,
    extra_pipelines: I,
) -> anyhow::Result<()>
where
    M: 'static + std::fmt::Debug + Clone + Send,
    H: handler::SctkHandler<M> + 'static,
    V: Fn(&TargetId, &S) -> Element<M> + 'static,
    U: FnMut(TargetId, &mut Engine<'a, M>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> bool
        + 'static,
    I: IntoIterator<Item = (&'static str, PipelineFactoryFn)>,
{
    let pipelines: Vec<(&'static str, PipelineFactoryFn)> = extra_pipelines.into_iter().collect();

    run_app_core::<M, S, V, U, H, _>(state, view, update, Options::Layer(opts), move |engine| {
        for (key, factory) in pipelines {
            engine.register_pipeline(crate::render::pipeline::PipelineKey::Other(key), factory);
        }
    })
}

pub fn run_app<'a, M, S, H, V, U>(
    state: S,
    view: V,
    update: U,
    opts: XdgOptions,
) -> anyhow::Result<()>
where
    M: 'static + std::fmt::Debug + Clone + Send,
    H: handler::SctkHandler<M> + 'static,
    V: Fn(&TargetId, &S) -> Element<M> + 'static,
    U: FnMut(TargetId, &mut Engine<'a, M>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> bool
        + 'static,
{
    run_app_core::<M, S, V, U, H, _>(state, view, update, Options::Xdg(opts), |_| {})
}

pub fn run_app_with<'a, M, S, H, V, U, I>(
    state: S,
    view: V,
    update: U,
    opts: XdgOptions,
    extra_pipelines: I,
) -> anyhow::Result<()>
where
    M: 'static + std::fmt::Debug + Clone + Send,
    H: handler::SctkHandler<M> + 'static,
    V: Fn(&TargetId, &S) -> Element<M> + 'static,
    U: FnMut(TargetId, &mut Engine<'a, M>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> bool
        + 'static,
    I: IntoIterator<Item = (&'static str, PipelineFactoryFn)>,
{
    let pipelines: Vec<(&'static str, PipelineFactoryFn)> = extra_pipelines.into_iter().collect();

    run_app_core::<M, S, V, U, H, _>(state, view, update, Options::Xdg(opts), move |engine| {
        for (key, factory) in pipelines.iter().copied() {
            engine.register_pipeline(crate::render::pipeline::PipelineKey::Other(key), factory);
        }
    })
}
