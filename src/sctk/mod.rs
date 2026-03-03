use std::{
    any::Any,
    collections::HashMap,
    fmt::Debug,
    ptr::NonNull,
    sync::{Arc, Mutex, atomic::AtomicBool},
};

use crate::{
    event::{
        Event, KeyEvent, KeyLocation, KeyState, Modifiers, MouseButton, PhysicalKey, ScrollDelta,
        ScrollUnits, ToEvent,
    },
    graphics::{Engine, TargetId},
    model::{Position, Size},
    render::PipelineFactoryFn,
    widget::Element,
};
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::client::{
        Connection, Proxy, QueueHandle, globals::registry_queue_init,
        protocol::wl_surface::WlSurface,
    },
    registry::RegistryState,
    seat::SeatState,
    session_lock::SessionLockState,
    shell::{WaylandSurface, wlr_layer::LayerShell, xdg::XdgShell},
};

pub use smithay_client_toolkit::shell::{
    wlr_layer::{Anchor, KeyboardInteractivity, Layer},
    xdg::window::WindowDecorations,
};

pub mod erased;
pub mod handler;
mod helpers;
pub mod state;

// === Public API ================================================================================

#[derive(Clone, Debug)]
pub enum OutputSet {
    /// Use single-output selector
    One(OutputSelector),
    /// Use the last active output.
    Active,
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
pub struct LockOptions {
    pub size: Size<u32>,
    pub output: Option<OutputSet>,
}

impl Default for LockOptions {
    fn default() -> Self {
        Self {
            size: Size::new(640, 360),
            output: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Options {
    Layer(LayerOptions),
    Xdg(XdgOptions),
    Lock(LockOptions),
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
    PointerButton {
        surface: SurfaceId,
        button: u32, // linux input BTN_* code
        pressed: bool,
    },
    PointerAxis {
        surface: SurfaceId,
        h: f64,
        v: f64,
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
            | SctkEvent::PointerButton { surface, .. }
            | SctkEvent::PointerAxis { surface, .. }
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
            SctkEvent::PointerButton {
                button, pressed, ..
            } => {
                // Map common BTN_* codes; unknown -> Other(code)
                let mb = match *button {
                    272 => MouseButton::Left,    // BTN_LEFT
                    273 => MouseButton::Right,   // BTN_RIGHT
                    274 => MouseButton::Middle,  // BTN_MIDDLE
                    275 => MouseButton::Back,    // BTN_SIDE
                    276 => MouseButton::Forward, // BTN_EXTRA
                    n => MouseButton::Other(n as u16),
                };
                let ks = if *pressed {
                    KeyState::Pressed
                } else {
                    KeyState::Released
                };
                Event::MouseInput {
                    button: mb,
                    state: ks,
                }
            }
            SctkEvent::PointerAxis { h, v, .. } => Event::MouseWheel(ScrollDelta {
                dx: *h as f32,
                dy: *v as f32,
                units: ScrollUnits::Pixels,
            }),

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

pub struct DefaultHandler;

impl<M> handler::SctkHandler<M> for DefaultHandler {}

enum RunnerEvent {
    SurfaceDestroyed(u32),
    OutputCreated,
    LockFinished,
}

struct RunnerHandler;

impl handler::SctkHandler<RunnerEvent> for RunnerHandler {
    fn new_output(
        _conn: &Connection,
        _qh: &QueueHandle<self::state::SctkState>,
        _output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) -> handler::Emit<RunnerEvent> {
        handler::Emit::One(RunnerEvent::OutputCreated)
    }
    fn update_output(
        _conn: &Connection,
        _qh: &QueueHandle<self::state::SctkState>,
        _output: smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
    ) -> handler::Emit<RunnerEvent> {
        handler::Emit::One(RunnerEvent::OutputCreated)
    }
    fn closed(
        _conn: &Connection,
        _qh: &QueueHandle<self::state::SctkState>,
        layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
    ) -> handler::Emit<RunnerEvent> {
        handler::Emit::One(RunnerEvent::SurfaceDestroyed(
            layer.wl_surface().id().protocol_id(),
        ))
    }

    fn request_close(
        _conn: &Connection,
        _qh: &QueueHandle<self::state::SctkState>,
        window: &smithay_client_toolkit::shell::xdg::window::Window,
    ) -> handler::Emit<RunnerEvent> {
        handler::Emit::One(RunnerEvent::SurfaceDestroyed(
            window.wl_surface().id().protocol_id(),
        ))
    }

    fn finished(
        _conn: &Connection,
        _qh: &QueueHandle<self::state::SctkState>,
        _session_lock: smithay_client_toolkit::session_lock::SessionLock,
    ) -> handler::Emit<RunnerEvent> {
        handler::Emit::One(RunnerEvent::LockFinished)
    }
}

#[derive(Clone)]
enum OutputHotplugCfg {
    Layer(LayerOptions),
    Lock(LockOptions),
}

// TODO: collect error results for further diagnosis
fn run_app_core<'a, M, S, V, U, H, F>(
    mut state: S,
    view: V,
    mut update: U,
    opts: Options,
    post_engine_init: F,
) -> crate::Result<()>
where
    M: 'static + std::fmt::Debug + Clone + Send,
    V: Fn(&TargetId, &S) -> Element<M> + 'static,
    U: FnMut(TargetId, &mut Engine<'a, M>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> bool
        + 'static,
    H: handler::SctkHandler<M> + 'static,
    F: FnOnce(&mut Engine<'a, M>),
{
    // 1) Wayland connection + queue
    let conn = Connection::connect_to_env().map_err(crate::error::SctkError::connect)?;
    let (globals, mut event_queue) =
        registry_queue_init(&conn).map_err(crate::error::SctkError::registry_init)?;

    let qh: QueueHandle<state::SctkState> = event_queue.handle();

    // 2) Bind globals
    let registry = RegistryState::new(&globals);
    let compositor =
        CompositorState::bind(&globals, &qh).map_err(crate::error::SctkError::bind_global)?;

    let outputs = OutputState::new(&globals, &qh);
    let seats = SeatState::new(&globals, &qh);
    let session_lock = SessionLockState::new(&globals, &qh);

    let (tx_sctk, rx_sctk) = calloop::channel::channel::<SctkEvent>();
    let (tx_runner, rx_runner) = calloop::channel::channel::<RunnerEvent>();
    let sctk_handler = erased::erase_with_runner::<H, M, _, _>(
        {
            let t = tx_sctk.clone();
            move |m| {
                let _ = t.send(SctkEvent::message(m));
            }
        },
        move |re| {
            let _ = tx_runner.send(re);
        },
    );

    // 3) Concrete SCTK state
    let hotplug_cfg = match opts.clone() {
        Options::Layer(o) => Some(OutputHotplugCfg::Layer(o)),
        Options::Lock(o) => Some(OutputHotplugCfg::Lock(o)),
        _ => None,
    };

    let mut st = match opts {
        Options::Layer(layer_options) => {
            let layer_shell =
                LayerShell::bind(&globals, &qh).map_err(crate::error::SctkError::bind_global)?;

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
                tx_sctk,
            )?
        }
        Options::Xdg(xdg_options) => {
            let xdg_shell =
                XdgShell::bind(&globals, &qh).map_err(crate::error::SctkError::bind_global)?;

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
                tx_sctk,
            )?
        }
        Options::Lock(lock_options) => {
            let mut st = state::SctkState::new(
                compositor,
                None,
                None,
                outputs,
                seats,
                registry,
                session_lock,
                sctk_handler,
                tx_sctk,
            );

            st.lock_session(&qh, lock_options)?;
            st
        }
    };

    // 4) Create engine and attach surfaces
    let mut sid_to_tid = HashMap::new();
    let mut engine = {
        let Some(sid) = st.surfaces.keys().next() else {
            return Err(crate::error::SctkError::SurfaceSetup.into());
        };
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
    while !loop_ctl.should_exit() {
        event_queue
            .blocking_dispatch(&mut st)
            .map_err(crate::error::SctkError::dispatch)?;

        let mut any_rendered = false;

        while let Ok(re) = rx_runner.try_recv() {
            match re {
                RunnerEvent::SurfaceDestroyed(sid) => {
                    let sid = SurfaceId(sid);

                    st.remove_surface_by_surface_id(sid);
                    if let Some(tid) = sid_to_tid.remove(&sid) {
                        engine.detach_target(&tid);
                    }

                    if sid_to_tid.is_empty() {
                        loop_ctl.exit();
                    }
                }
                RunnerEvent::OutputCreated => {
                    let Some(cfg) = &hotplug_cfg else { continue };

                    let new_surfaces = match cfg {
                        OutputHotplugCfg::Layer(layer_opts) => {
                            st.ensure_layer_surfaces(&qh, layer_opts)
                        }
                        OutputHotplugCfg::Lock(lock_opts) => {
                            st.ensure_lock_surfaces(&qh, lock_opts).unwrap_or_default()
                        }
                    };

                    for (sid, size) in new_surfaces {
                        let target =
                            Arc::new(RawWaylandHandles::new(&conn, &st.surfaces[&sid].wl_surface));
                        let tid = engine.attach_target(target, size);
                        sid_to_tid.insert(sid, tid);
                    }
                }
                RunnerEvent::LockFinished => {
                    loop_ctl.exit();
                }
            }
        }

        while let Ok(ev) = rx_sctk.try_recv() {
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

        for (sid, &tid) in sid_to_tid.iter() {
            if !st.surfaces.contains_key(sid) {
                continue;
            }

            let need = st.surfaces.get(sid).map(|s| s.configured).unwrap_or(false)
                && engine.poll(
                    &tid,
                    &mut |eng, e, s, ctl| update(tid, eng, e, s, ctl),
                    &mut state,
                    &loop_ctl,
                );
            engine.render_if_needed(&tid, need, &view, &mut state);
            any_rendered |= need;
        }

        if any_rendered {
            crate::profile::frame_mark();
        }
    }

    st.unlock_session();
    conn.flush().map_err(crate::error::SctkError::flush)?;

    event_queue
        .roundtrip(&mut st)
        .map_err(crate::error::SctkError::roundtrip)?;

    Ok(())
}

pub fn run_layer<'a, M, S, H, V, U>(
    state: S,
    view: V,
    update: U,
    opts: LayerOptions,
) -> crate::Result<()>
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
) -> crate::Result<()>
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
) -> crate::Result<()>
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
) -> crate::Result<()>
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

pub fn run_lock<'a, M, S, H, V, U>(
    state: S,
    view: V,
    update: U,
    opts: LockOptions,
) -> crate::Result<()>
where
    M: 'static + std::fmt::Debug + Clone + Send,
    H: handler::SctkHandler<M> + 'static,
    V: Fn(&TargetId, &S) -> Element<M> + 'static,
    U: FnMut(TargetId, &mut Engine<'a, M>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> bool
        + 'static,
{
    run_app_core::<M, S, V, U, H, _>(state, view, update, Options::Lock(opts), |_| {})
}

pub fn run_lock_with<'a, M, S, H, V, U, I>(
    state: S,
    view: V,
    update: U,
    opts: LockOptions,
    extra_pipelines: I,
) -> crate::Result<()>
where
    M: 'static + std::fmt::Debug + Clone + Send,
    H: handler::SctkHandler<M> + 'static,
    V: Fn(&TargetId, &S) -> Element<M> + 'static,
    U: FnMut(TargetId, &mut Engine<'a, M>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> bool
        + 'static,
    I: IntoIterator<Item = (&'static str, PipelineFactoryFn)>,
{
    let pipelines: Vec<(&'static str, PipelineFactoryFn)> = extra_pipelines.into_iter().collect();

    run_app_core::<M, S, V, U, H, _>(state, view, update, Options::Lock(opts), move |engine| {
        for (key, factory) in pipelines.iter().copied() {
            engine.register_pipeline(crate::render::pipeline::PipelineKey::Other(key), factory);
        }
    })
}
