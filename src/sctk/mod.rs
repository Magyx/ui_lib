use std::{
    any::Any,
    cell::Cell,
    fmt::Debug,
    ptr::NonNull,
    sync::{Arc, Mutex, mpsc},
};

use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::client::{Connection, QueueHandle, globals::registry_queue_init},
    registry::RegistryState,
    seat::SeatState,
    shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell},
};
use wayland_client::{Proxy, protocol::wl_surface::WlSurface};

use crate::{
    event::{Event, ToEvent},
    graphics::Engine,
    model::{Position, Size},
    render::PipelineFactoryFn,
    widget::Element,
};

mod adapter;
mod erased;
mod handler;
mod helpers;
mod msg;
mod state;

// === Public API ================================================================================

#[derive(Clone, Debug)]
pub enum OutputSelector<'a> {
    /// First output in SCTK’s list (current behavior)
    First,
    /// Nth output (0-based)
    Index(usize),
    /// Choose the output whose info.name/model/make starts with this string
    NamePrefix(&'a str),
    /// Prefer laptop panel-ish names (eDP, LVDS), fall back to First
    InternalPrefer,
    /// Pick the output with the highest reported scale factor
    HighestScale,
}

/// Options describing the layer-shell surface (instead of winit's WindowAttributes).
#[derive(Clone, Debug)]
pub struct LayerOptions<'a> {
    pub layer: Layer,
    pub size: Size<u32>,
    pub anchors: Anchor,
    /// Negative means "auto" (no reservation). Positive reserves screen space (e.g. status bar).
    pub exclusive_zone: i32,
    pub keyboard_interactivity: KeyboardInteractivity,
    /// Namespace, useful for compositor rules.
    pub namespace: Option<&'a str>,
    pub output: Option<OutputSelector<'a>>,
}

impl<'a> Default for LayerOptions<'a> {
    fn default() -> Self {
        Self {
            layer: Layer::Top,
            size: Size::new(640, 360),
            anchors: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
            exclusive_zone: -1,
            keyboard_interactivity: KeyboardInteractivity::None,
            namespace: Some("ui"),
            output: None,
        }
    }
}

/// Loop control, analogous to winit's `ActiveEventLoop` (shared borrow in `update`).
pub struct SctkLoop {
    exit: Cell<bool>,
}

impl SctkLoop {
    fn new() -> Self {
        Self {
            exit: Cell::new(false),
        }
    }
    pub fn exit(&self) {
        self.exit.set(true);
    }
    pub fn should_exit(&self) -> bool {
        self.exit.get()
    }
}

/// Platform event type for the SCTK backend.
#[derive(Debug)]
pub enum SctkEvent {
    Redraw,
    Resized { size: Size<u32> },
    PointerMoved { pos: Position<f32> },
    PointerDown,
    PointerUp,
    Keyboard { ch: u8 },
    Closed,
    Message(Arc<Mutex<Option<Box<dyn Any + Send>>>>),
}

impl SctkEvent {
    pub fn message<M: Send + 'static>(m: M) -> Self {
        SctkEvent::Message(Arc::new(Mutex::new(Some(Box::new(m)))))
    }
}

impl<M: 'static + Send> ToEvent<M, SctkEvent> for SctkEvent {
    fn to_event(&self) -> Event<M, SctkEvent> {
        match self {
            SctkEvent::Redraw => Event::RedrawRequested,
            SctkEvent::Resized { size } => Event::Resized { size: *size },
            SctkEvent::PointerMoved { pos } => Event::CursorMoved { position: *pos },
            SctkEvent::PointerDown => Event::MouseInput { mouse_down: true },
            SctkEvent::PointerUp => Event::MouseInput { mouse_down: false },
            SctkEvent::Keyboard { ch } => Event::KeyboardInput { char: *ch },
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

pub struct DefaultHandler;

impl<M> handler::SctkHandler<M> for DefaultHandler {}

#[derive(Clone)]
struct UnsafeWaylandHandles {
    display: NonNull<std::ffi::c_void>,
    surface: NonNull<std::ffi::c_void>,
}

impl UnsafeWaylandHandles {
    fn new(conn: &Connection, wl_surface: &WlSurface) -> Self {
        let display = NonNull::new(conn.display().id().as_ptr().cast()).expect("null wl_display");
        let surface = NonNull::new(wl_surface.id().as_ptr().cast()).expect("null wl_surface");
        Self { display, surface }
    }
}

unsafe impl Send for UnsafeWaylandHandles {}
unsafe impl Sync for UnsafeWaylandHandles {}

impl wgpu::rwh::HasWindowHandle for UnsafeWaylandHandles {
    fn window_handle(&self) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError> {
        let wl = wgpu::rwh::WaylandWindowHandle::new(self.surface);
        Ok(unsafe { wgpu::rwh::WindowHandle::borrow_raw(wgpu::rwh::RawWindowHandle::from(wl)) })
    }
}
impl wgpu::rwh::HasDisplayHandle for UnsafeWaylandHandles {
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
        let wl = wgpu::rwh::WaylandDisplayHandle::new(self.display);
        Ok(unsafe { wgpu::rwh::DisplayHandle::borrow_raw(wgpu::rwh::RawDisplayHandle::from(wl)) })
    }
}

fn run_app_core<'a, M, S, V, U, H, F>(
    mut state: S,
    view: V,
    mut update: U,
    opts: LayerOptions<'_>,
    post_engine_init: F,
) -> anyhow::Result<()>
where
    M: 'static + std::fmt::Debug + Send,
    V: Fn(&S) -> Element<M> + 'static,
    U: FnMut(&mut Engine<'a, M>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> bool + 'static,
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
    let layer_shell = LayerShell::bind(&globals, &qh)?;

    let (tx, rx) = mpsc::channel();
    let sctk_handler = adapter::erase::<H, M, _>(move |m| {
        let _ = tx.send(m);
    });

    // 3) Concrete SCTK state
    let mut st = state::SctkState::new(
        &globals,
        &qh,
        opts,
        compositor,
        layer_shell,
        outputs,
        seats,
        registry,
        sctk_handler,
    )?;

    // 4) Create engine
    let window_target = Arc::new(UnsafeWaylandHandles::new(&conn, &st.wl_surface));
    let mut engine = {
        let size = st.size;
        let mut engine = Engine::new(window_target, size);
        post_engine_init(&mut engine);
        engine
    };

    let loop_ctl = SctkLoop::new();

    // 5) Main loop
    while !loop_ctl.should_exit() && !st.closed {
        event_queue.blocking_dispatch(&mut st)?;

        for ev in st.take_events() {
            engine.handle_platform_event(&ev, &mut update, &mut state, &loop_ctl);
        }

        while let Ok(m) = rx.try_recv() {
            engine.handle_platform_event(
                &SctkEvent::message(m),
                &mut update,
                &mut state,
                &loop_ctl,
            );
        }

        if st.needs_redraw {
            // only happens once, on configure
            st.needs_redraw = false;
            engine.render_if_needed(true, &view, &mut state);
        } else {
            let require_redraw = engine.poll(&view, &mut update, &mut state, &loop_ctl);
            engine.render_if_needed(require_redraw, &view, &mut state);
        }
    }

    Ok(())
}

pub fn run_app<'a, M, S, H, V, U>(
    state: S,
    view: V,
    update: U,
    opts: LayerOptions<'_>,
) -> anyhow::Result<()>
where
    M: 'static + std::fmt::Debug + Send,
    H: handler::SctkHandler<M> + 'static,
    V: Fn(&S) -> Element<M> + 'static,
    U: FnMut(&mut Engine<'a, M>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> bool + 'static,
{
    run_app_core::<M, S, V, U, H, _>(state, view, update, opts, |_| {})
}

pub fn run_app_with<'a, M, S, H, V, U, I>(
    state: S,
    view: V,
    update: U,
    opts: LayerOptions<'_>,
    extra_pipelines: I,
) -> anyhow::Result<()>
where
    M: 'static + std::fmt::Debug + Send,
    H: handler::SctkHandler<M> + 'static,
    V: Fn(&S) -> Element<M> + 'static,
    U: FnMut(&mut Engine<'a, M>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> bool + 'static,
    I: IntoIterator<Item = (&'static str, PipelineFactoryFn)>,
{
    let pipelines: Vec<(&'static str, PipelineFactoryFn)> = extra_pipelines.into_iter().collect();

    run_app_core::<M, S, V, U, H, _>(state, view, update, opts, move |engine| {
        for (key, factory) in pipelines {
            engine.register_pipeline(crate::render::pipeline::PipelineKey::Other(key), factory);
        }
    })
}
