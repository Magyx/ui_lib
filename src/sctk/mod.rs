use std::{
    any::Any,
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fmt::Debug,
    ptr::NonNull,
    rc::Rc,
    sync::{Arc, atomic::AtomicBool},
};

use crate::{
    context::MessageSink,
    event::{
        Event, KeyEvent, KeyLocation, KeyState, Modifiers, MouseButton, PhysicalKey, ScrollDelta,
        ScrollUnits, ToEvent,
    },
    graphics::{Engine, TargetId},
    model::{Position, Size},
    render::PipelineFactoryFn,
    task::{BoxWork, Payload, Task, TaskId, TaskRunner},
    widget::Element,
};
use calloop::EventLoop;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::{
        calloop_wayland_source::WaylandSource,
        client::{
            Connection, Proxy, QueueHandle, globals::registry_queue_init,
            protocol::wl_surface::WlSurface,
        },
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

// Public API

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
    ScaleChanged {
        surface: SurfaceId,
        factor: i32,
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
}

impl SctkEvent {
    pub fn surface_id(&self) -> Option<SurfaceId> {
        match self {
            SctkEvent::Resized { surface, .. }
            | SctkEvent::ScaleChanged { surface, .. }
            | SctkEvent::PointerMoved { surface, .. }
            | SctkEvent::PointerButton { surface, .. }
            | SctkEvent::PointerAxis { surface, .. }
            | SctkEvent::Key { surface, .. }
            | SctkEvent::Modifiers(surface, ..) => Some(*surface),
            _ => None,
        }
    }
}

impl<M> ToEvent<M, SctkEvent> for SctkEvent {
    fn to_event(&self) -> Event<M, SctkEvent> {
        match self {
            SctkEvent::Redraw => Event::RedrawRequested,
            SctkEvent::Resized { size, .. } => Event::Resized { size: *size },
            SctkEvent::ScaleChanged { factor, .. } => Event::ScaleFactorChanged {
                factor: *factor as f64,
            },
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

#[derive(Clone, Default)]
pub struct SctkMessageSink(Rc<RefCell<Vec<Box<dyn Any>>>>);

impl MessageSink for SctkMessageSink {
    fn emit(&mut self, msg: Box<dyn Any>) {
        self.0.borrow_mut().push(msg);
    }
    fn drain(&mut self) -> Vec<Box<dyn Any>> {
        std::mem::take(&mut *self.0.borrow_mut())
    }
}

pub struct CalloopRunner {
    tx: calloop::channel::Sender<(TargetId, TaskId, Payload)>,
    inbox: Rc<RefCell<VecDeque<(TargetId, TaskId, Payload)>>>,
}
impl CalloopRunner {
    pub fn new() -> (Self, calloop::channel::Channel<(TargetId, TaskId, Payload)>) {
        let (tx, rx) = calloop::channel::channel();
        let inbox = Rc::new(RefCell::new(VecDeque::new()));
        (Self { tx, inbox }, rx)
    }

    pub fn inbox(&self) -> Rc<RefCell<VecDeque<(TargetId, TaskId, Payload)>>> {
        self.inbox.clone()
    }
}
impl TaskRunner for CalloopRunner {
    fn spawn(&self, target: TargetId, id: TaskId, run: BoxWork) {
        let tx = self.tx.clone();
        std::thread::Builder::new()
            .name("ui-task".into())
            .spawn(move || {
                let payload = pollster::block_on(run);
                // Sending wakes the loop; an error means the loop/receiver is
                // gone (shutting down), so dropping the payload is correct.
                let _ = tx.send((target, id, payload));
            })
            .expect("spawn ui-task thread");
    }

    fn drain(&self, out: &mut Vec<(TargetId, TaskId, Payload)>) {
        out.extend(self.inbox.borrow_mut().drain(..));
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
    exit_on_close: bool,
    post_engine_init: F,
) -> crate::Result<()>
where
    M: 'static,
    V: Fn(&TargetId, &S) -> Element + 'static,
    U: FnMut(TargetId, &mut Engine<'a>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> Task<M>
        + 'static,
    H: handler::SctkHandler<M> + 'static,
    F: FnOnce(&mut Engine<'a>),
{
    // 1) Wayland connection + queue
    let conn = Connection::connect_to_env().map_err(crate::error::SctkError::connect)?;
    let (globals, event_queue) =
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
    let (tx_msg, rx_msg) = calloop::channel::channel::<Box<dyn Any>>();

    let (task_runner, rx_task) = CalloopRunner::new();
    let task_inbox = task_runner.inbox();

    let sctk_handler = erased::erase_with_runner::<H, M, _, _>(
        move |m| {
            let _ = tx_msg.send(Box::new(m));
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
    let mut sink = SctkMessageSink::default();

    let mut sid_to_tid = HashMap::new();
    let mut engine = {
        let Some(sid) = st.surfaces.keys().next() else {
            return Err(crate::error::SctkError::SurfaceSetup.into());
        };

        let mut engine = Engine::<'a>::builder::<M>()
            .with_message_sink(Box::new(sink.clone()))
            .with_task_runner(Box::new(task_runner))
            .build()?;

        let rec = &st.surfaces[sid];
        let sf = rec.scale_factor.max(1) as f64;
        let phys = Size::new(
            rec.size.width * rec.scale_factor.max(1) as u32,
            rec.size.height * rec.scale_factor.max(1) as u32,
        );
        let target = Arc::new(RawWaylandHandles::new(&conn, &rec.wl_surface));
        let tid = engine.attach_target(target, phys, sf);
        sid_to_tid.insert(*sid, tid);
        post_engine_init(&mut engine);

        for (&sid, rec) in st.surfaces.iter().skip(1) {
            let sf = rec.scale_factor.max(1) as f64;
            let phys = Size::new(
                rec.size.width * rec.scale_factor.max(1) as u32,
                rec.size.height * rec.scale_factor.max(1) as u32,
            );
            let target = Arc::new(RawWaylandHandles::new(&conn, &rec.wl_surface));
            let tid = engine.attach_target(target, phys, sf);
            sid_to_tid.insert(sid, tid);
        }
        engine
    };

    let loop_ctl = SctkLoop::default();

    // 5) Main loop
    let mut event_loop: EventLoop<state::SctkState> =
        EventLoop::try_new().map_err(crate::error::SctkError::event_loop)?;

    WaylandSource::new(conn.clone(), event_queue)
        .insert(event_loop.handle())
        .map_err(|e| crate::error::SctkError::event_loop(e.error))?;

    event_loop
        .handle()
        .insert_source(rx_msg, move |event, _, _st| {
            if let calloop::channel::Event::Msg(msg) = event {
                sink.emit(msg);
            }
        })
        .map_err(|e| crate::error::SctkError::event_loop(e.error))?;

    event_loop
        .handle()
        .insert_source(rx_task, move |event, _, _st| {
            if let calloop::channel::Event::Msg(item) = event {
                task_inbox.borrow_mut().push_back(item);
            }
        })
        .map_err(|e| crate::error::SctkError::event_loop(e.error))?;

    while !loop_ctl.should_exit() {
        event_loop
            .dispatch(None, &mut st)
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
                        let rec = &st.surfaces[&sid];
                        let sf = rec.scale_factor.max(1) as f64;
                        let phys = Size::new(
                            size.width * rec.scale_factor.max(1) as u32,
                            size.height * rec.scale_factor.max(1) as u32,
                        );
                        let target = Arc::new(RawWaylandHandles::new(&conn, &rec.wl_surface));
                        let tid = engine.attach_target(target, phys, sf);
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

            // TODO: SctkEvent::Closed should carry the sid
            if exit_on_close && matches!(ev, SctkEvent::Closed) {
                loop_ctl.exit();
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
            if let Err(e) = engine.render_if_needed(&tid, need, &view, &mut state) {
                #[cfg(feature = "tracing")]
                tracing::error!("error dialog render failed: {e:?}");
            }
            any_rendered |= need;
        }

        if any_rendered {
            crate::profile::frame_mark();
        }
    }

    st.unlock_session();
    let _ = conn.flush();

    Ok(())
}

#[allow(clippy::type_complexity)]
pub struct SctkApp<'a, M, S, V, U, H = DefaultHandler> {
    state: S,
    view: V,
    update: U,
    opts: Options,
    extra_pipelines: Vec<(&'static str, PipelineFactoryFn)>,
    exit_on_close: bool,
    _marker: std::marker::PhantomData<(fn() -> M, fn() -> H, &'a ())>,
}

impl<'a, M, S, V, U> SctkApp<'a, M, S, V, U, DefaultHandler> {
    /// Build a `wlr-layer-shell` surface application (bars, overlays, wallpapers).
    pub fn layer(state: S, view: V, update: U, opts: LayerOptions) -> Self {
        Self::with_options(state, view, update, Options::Layer(opts))
    }

    /// Build an XDG toplevel (regular window) application.
    pub fn window(state: S, view: V, update: U, opts: XdgOptions) -> Self {
        Self::with_options(state, view, update, Options::Xdg(opts))
    }

    /// Build a `session-lock` surface application (lock screens).
    pub fn lock(state: S, view: V, update: U, opts: LockOptions) -> Self {
        Self::with_options(state, view, update, Options::Lock(opts))
    }

    fn with_options(state: S, view: V, update: U, opts: Options) -> Self {
        Self {
            state,
            view,
            update,
            opts,
            extra_pipelines: Vec::new(),
            exit_on_close: true,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'a, M, S, V, U, H> SctkApp<'a, M, S, V, U, H> {
    /// Use a custom [`SctkHandler`](handler::SctkHandler) type instead of the
    /// [`DefaultHandler`]. This only changes the handler *type*; there is no
    /// value to pass since handlers are zero-sized markers.
    pub fn handler<H2>(self) -> SctkApp<'a, M, S, V, U, H2> {
        SctkApp {
            state: self.state,
            view: self.view,
            update: self.update,
            opts: self.opts,
            extra_pipelines: self.extra_pipelines,
            exit_on_close: self.exit_on_close,
            _marker: std::marker::PhantomData,
        }
    }

    /// Control whether the event loop exits automatically when the surface
    /// receives a close request ([`SctkEvent::Closed`]).
    ///
    /// Defaults to `true`, so you don't need to wire up close handling in your
    /// `update` function. The close event is still delivered to `update` before
    /// the loop exits, so any cleanup there still runs. Pass `false` if you want
    /// to decide when to exit yourself.
    pub fn exit_on_close(mut self, exit_on_close: bool) -> Self {
        self.exit_on_close = exit_on_close;
        self
    }

    /// Register a single extra render pipeline factory under `name`.
    pub fn pipeline(mut self, name: &'static str, factory: PipelineFactoryFn) -> Self {
        self.extra_pipelines.push((name, factory));
        self
    }

    /// Register extra render pipeline factories, e.g. from the
    /// [`pipeline_factories!`](crate::pipeline_factories) macro. Can be called
    /// more than once to accumulate factories.
    pub fn pipelines<I>(mut self, pipelines: I) -> Self
    where
        I: IntoIterator<Item = (&'static str, PipelineFactoryFn)>,
    {
        self.extra_pipelines.extend(pipelines);
        self
    }

    /// Run the application. Blocks until the event loop exits.
    pub fn run(self) -> crate::Result<()>
    where
        M: 'static,
        H: handler::SctkHandler<M> + 'static,
        V: Fn(&TargetId, &S) -> Element + 'static,
        U: FnMut(TargetId, &mut Engine<'a>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> Task<M>
            + 'static,
    {
        let pipelines = self.extra_pipelines;
        run_app_core::<M, S, V, U, H, _>(
            self.state,
            self.view,
            self.update,
            self.opts,
            self.exit_on_close,
            move |engine| {
                for (key, factory) in pipelines {
                    engine.register_pipeline(
                        crate::render::pipeline::PipelineKey::Other(key),
                        factory,
                    );
                }
            },
        )
    }
}
