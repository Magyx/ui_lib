use std::{any::Any, collections::HashMap, sync::Arc};

use crate::{
    engine::{Engine, TargetId},
    event::Event,
    model::Size,
    render::pipeline::{Pipeline, PipelineRegistration},
    task::Task,
    widget::Element,
};
use calloop::EventLoop;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::{
        calloop_wayland_source::WaylandSource,
        client::{Connection, QueueHandle, globals::registry_queue_init},
    },
    registry::RegistryState,
    seat::SeatState,
    session_lock::SessionLockState,
    shell::{wlr_layer::LayerShell, xdg::XdgShell},
};

use super::{
    LayerOptions, LockOptions, Options, SctkError, SctkEvent, SctkLoop, SurfaceId, XdgOptions,
    handler::{DefaultHandler, RunnerEvent, SctkHandler},
    raw,
    runner::{CalloopRunner, SctkMessageSink},
    state,
};

#[derive(Clone)]
enum OutputHotplugCfg {
    Layer(LayerOptions),
    Lock(LockOptions),
}

// TODO: collect error results for further diagnosis
fn run_app_core<M, S, V, U, H, F>(
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
    U: FnMut(TargetId, &mut Engine<'_>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> Task<M>
        + 'static,
    H: SctkHandler<M> + 'static,
    F: FnOnce(&mut Engine<'_>),
{
    // 1) Wayland connection + queue
    let conn = Connection::connect_to_env().map_err(SctkError::connect)?;
    let (globals, event_queue) = registry_queue_init(&conn).map_err(SctkError::registry_init)?;

    let qh: QueueHandle<state::SctkState> = event_queue.handle();

    // 2) Bind globals
    let registry = RegistryState::new(&globals);
    let compositor = CompositorState::bind(&globals, &qh).map_err(SctkError::bind_global)?;

    let outputs = OutputState::new(&globals, &qh);
    let seats = SeatState::new(&globals, &qh);
    let session_lock = SessionLockState::new(&globals, &qh);

    let (tx_sctk, rx_sctk) = calloop::channel::channel::<SctkEvent>();
    let (tx_runner, rx_runner) = calloop::channel::channel::<RunnerEvent>();
    let (tx_msg, rx_msg) = calloop::channel::channel::<Box<dyn Any>>();

    let (task_runner, rx_task) = CalloopRunner::new();
    let task_inbox = task_runner.inbox();

    let sctk_handler = super::erased::erase_with_runner::<H, M, _, _>(
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
            let layer_shell = LayerShell::bind(&globals, &qh).map_err(SctkError::bind_global)?;

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
            let xdg_shell = XdgShell::bind(&globals, &qh).map_err(SctkError::bind_global)?;

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
            return Err(SctkError::SurfaceSetup.into());
        };

        let mut engine = Engine::builder::<M>()
            .with_message_sink(Box::new(sink.clone()))
            .with_task_runner(Box::new(task_runner))
            .build()?;

        let rec = &st.surfaces[sid];
        let sf = rec.scale_factor.max(1) as f64;
        let phys = Size::new(
            rec.size.width * rec.scale_factor.max(1) as u32,
            rec.size.height * rec.scale_factor.max(1) as u32,
        );
        let target = Arc::new(raw::WaylandHandles::new(&conn, &rec.wl_surface));
        let tid = engine.attach_target(target, phys, sf);
        sid_to_tid.insert(*sid, tid);
        post_engine_init(&mut engine);

        for (&sid, rec) in st.surfaces.iter().skip(1) {
            let sf = rec.scale_factor.max(1) as f64;
            let phys = Size::new(
                rec.size.width * rec.scale_factor.max(1) as u32,
                rec.size.height * rec.scale_factor.max(1) as u32,
            );
            let target = Arc::new(raw::WaylandHandles::new(&conn, &rec.wl_surface));
            let tid = engine.attach_target(target, phys, sf);
            sid_to_tid.insert(sid, tid);
        }
        engine
    };

    let loop_ctl = SctkLoop::default();

    // 5) Main loop
    let mut event_loop: EventLoop<state::SctkState> =
        EventLoop::try_new().map_err(SctkError::event_loop)?;

    WaylandSource::new(conn.clone(), event_queue)
        .insert(event_loop.handle())
        .map_err(|e| SctkError::event_loop(e.error))?;

    event_loop
        .handle()
        .insert_source(rx_msg, move |event, _, _st| {
            if let calloop::channel::Event::Msg(msg) = event {
                sink.push(msg);
            }
        })
        .map_err(|e| SctkError::event_loop(e.error))?;

    event_loop
        .handle()
        .insert_source(rx_task, move |event, _, _st| {
            if let calloop::channel::Event::Msg(item) = event {
                task_inbox.borrow_mut().push_back(item);
            }
        })
        .map_err(|e| SctkError::event_loop(e.error))?;

    while !loop_ctl.should_exit() {
        event_loop
            .dispatch(None, &mut st)
            .map_err(SctkError::dispatch)?;

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
                        let target = Arc::new(raw::WaylandHandles::new(&conn, &rec.wl_surface));
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

pub struct SctkApp<M, S, V, U, H = DefaultHandler> {
    state: S,
    view: V,
    update: U,
    opts: Options,
    extra_pipelines: Vec<PipelineRegistration>,
    exit_on_close: bool,
    _marker: std::marker::PhantomData<(M, H)>,
}
impl<M, S, V, U> SctkApp<M, S, V, U, DefaultHandler> {
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
impl<M, S, V, U, H> SctkApp<M, S, V, U, H> {
    /// Use a custom [`SctkHandler`](handler::SctkHandler) type instead of the
    /// [`DefaultHandler`]. This only changes the handler *type*; there is no
    /// value to pass since handlers are zero-sized markers.
    pub fn handler<H2>(self) -> SctkApp<M, S, V, U, H2> {
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
    pub fn pipeline<P: Pipeline>(mut self) -> Self {
        self.extra_pipelines.push(PipelineRegistration::of::<P>());
        self
    }

    /// Register several at once. Registering the same pipeline type twice is
    /// harmless: the later build replaces the earlier one in the same slot.
    pub fn pipelines<I>(mut self, pipelines: I) -> Self
    where
        I: IntoIterator<Item = PipelineRegistration>,
    {
        self.extra_pipelines.extend(pipelines);
        self
    }

    /// Run the application. Blocks until the event loop exits.
    pub fn run(self) -> crate::Result<()>
    where
        M: 'static,
        H: SctkHandler<M> + 'static,
        V: Fn(&TargetId, &S) -> Element + 'static,
        U: FnMut(TargetId, &mut Engine<'_>, &Event<M, SctkEvent>, &mut S, &SctkLoop) -> Task<M>
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
                for reg in pipelines {
                    engine.register(reg);
                }
            },
        )
    }
}
