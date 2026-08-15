use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use super::*;

use crate::{
    engine::{Engine, TargetId},
    event::Event,
    render::pipeline::{Pipeline, PipelineRegistration},
    task::Task,
    widget::Element,
};

fn frame_interval_from_monitor(window: &Window) -> Duration {
    const NS_PER_S: u128 = 1_000_000_000;
    const M_PER: u128 = 1_000;
    const FALLBACK_NS_60HZ: u128 = NS_PER_S / 60;

    let ns = window
        .current_monitor()
        .and_then(|m| m.refresh_rate_millihertz())
        .map(|mhz| (NS_PER_S * M_PER) / (mhz as u128))
        .unwrap_or(FALLBACK_NS_60HZ);

    Duration::from_nanos(ns as u64)
}

pub struct WinitApp<'a, M, S, V, U>
where
    V: Fn(&TargetId, &S) -> Element + 'static,
    U: FnMut(
            TargetId,
            &mut Engine<'a>,
            &Event<M, WindowEvent>,
            &mut S,
            &ActiveEventLoop,
        ) -> Task<M>
        + 'static,
{
    window: Option<Arc<Window>>,
    target: Option<TargetId>,
    engine: Option<Engine<'a>>,
    extra_pipelines: Option<Vec<PipelineRegistration>>,
    state: S,
    view: V,
    update: U,
    window_attrs: WindowAttributes,
    next_frame: Instant,
    frame_interval: Duration,
    exit_on_close: bool,

    startup_error: Option<crate::Error>,

    _marker: std::marker::PhantomData<M>,
}

impl<'a, M, S, V, U> WinitApp<'a, M, S, V, U>
where
    M: 'static,
    V: Fn(&TargetId, &S) -> Element + 'static,
    U: FnMut(
            TargetId,
            &mut Engine<'a>,
            &Event<M, WindowEvent>,
            &mut S,
            &ActiveEventLoop,
        ) -> Task<M>
        + 'static,
{
    fn new(
        state: S,
        view: V,
        update: U,
        window_attrs: WindowAttributes,
        extra_pipelines: Option<Vec<PipelineRegistration>>,
        exit_on_close: bool,
    ) -> Self {
        Self {
            window: None,
            target: None,
            engine: None,
            extra_pipelines,
            state,
            view,
            update,
            window_attrs,
            next_frame: Instant::now(),
            frame_interval: Duration::from_millis(16),
            exit_on_close,

            startup_error: None,

            _marker: std::marker::PhantomData,
        }
    }

    /// See [`WinitAppBuilder::new`] for details.
    pub fn builder(state: S, view: V, update: U) -> WinitAppBuilder<M, S, V, U> {
        WinitAppBuilder::new(state, view, update)
    }
}

impl<'a, M, S, V, U> ApplicationHandler for WinitApp<'a, M, S, V, U>
where
    M: 'static,
    V: Fn(&TargetId, &S) -> Element + 'static,
    U: FnMut(
            TargetId,
            &mut Engine<'a>,
            &Event<M, WindowEvent>,
            &mut S,
            &ActiveEventLoop,
        ) -> Task<M>
        + 'static,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = match event_loop.create_window(self.window_attrs.clone()) {
                Ok(w) => Arc::new(w),
                Err(e) => {
                    self.startup_error = Some(e.into());
                    event_loop.exit();
                    return;
                }
            };
            let size = window.inner_size().into();
            let (target, mut engine) =
                Engine::new_for::<M, _>(window.clone(), size, window.scale_factor());
            if let Some(pipelines) = self.extra_pipelines.take() {
                for reg in pipelines {
                    engine.register(reg);
                }
            }

            self.frame_interval = frame_interval_from_monitor(&window);
            self.engine = Some(engine);
            self.target = Some(target);
            self.window = Some(window);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_frame {
            if let Some(w) = self.window.as_ref() {
                w.request_redraw();
            }
            self.next_frame = now + self.frame_interval;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: ::winit::window::WindowId,
        event: WindowEvent,
    ) {
        let update = &mut self.update;
        match event {
            WindowEvent::RedrawRequested => {
                let engine = self.engine.as_mut().unwrap();
                let should_redraw = engine.poll(
                    &self.target.unwrap(),
                    &mut |engine, event, state, loop_ctl| {
                        update(self.target.unwrap(), engine, event, state, loop_ctl)
                    },
                    &mut self.state,
                    event_loop,
                );
                _ = engine.render_if_needed(
                    &self.target.unwrap(),
                    should_redraw,
                    &self.view,
                    &mut self.state,
                );
                if should_redraw {
                    crate::profile::frame_mark();
                }
            }
            _ => {
                match event {
                    WindowEvent::ScaleFactorChanged { .. }
                    | WindowEvent::Moved(..)
                    | WindowEvent::Resized(..) => {
                        if let Some(window) = self.window.as_ref() {
                            self.frame_interval = frame_interval_from_monitor(window);
                        }
                    }
                    _ => (),
                }
                let engine = self.engine.as_mut().unwrap();
                engine.handle_platform_event(
                    &self.target.unwrap(),
                    &event,
                    &mut |engine, event, state, loop_ctl| {
                        update(self.target.unwrap(), engine, event, state, loop_ctl)
                    },
                    &mut self.state,
                    event_loop,
                );

                if self.exit_on_close && matches!(event, WindowEvent::CloseRequested) {
                    event_loop.exit();
                }
            }
        }
    }
}

fn run_app_core<'a, M, S, V, U>(
    state: S,
    view: V,
    update: U,
    window_attrs: WindowAttributes,
    extra_pipelines: Option<Vec<PipelineRegistration>>,
    exit_on_close: bool,
) -> crate::Result<()>
where
    M: 'static,
    V: Fn(&TargetId, &S) -> Element + 'static,
    U: FnMut(
            TargetId,
            &mut Engine<'a>,
            &Event<M, WindowEvent>,
            &mut S,
            &ActiveEventLoop,
        ) -> Task<M>
        + 'static,
{
    crate::profile::set_thread_name("ui-main");
    let event_loop = EventLoop::new()?;
    let mut app = WinitApp::<'a, M, S, V, U>::new(
        state,
        view,
        update,
        window_attrs,
        extra_pipelines,
        exit_on_close,
    );
    event_loop.run_app(&mut app)?;

    if let Some(err) = app.startup_error.take() {
        return Err(err);
    }
    Ok(())
}

#[allow(clippy::type_complexity)]
pub struct WinitAppBuilder<M, S, V, U> {
    state: S,
    view: V,
    update: U,
    window_attrs: WindowAttributes,
    extra_pipelines: Option<Vec<PipelineRegistration>>,
    exit_on_close: bool,
    _marker: std::marker::PhantomData<M>,
}
impl<'a, M, S, V, U> WinitAppBuilder<M, S, V, U>
where
    M: 'static,
    V: Fn(&TargetId, &S) -> Element + 'static,
    U: FnMut(
            TargetId,
            &mut Engine<'a>,
            &Event<M, WindowEvent>,
            &mut S,
            &ActiveEventLoop,
        ) -> Task<M>
        + 'static,
{
    /// Start building an application from its initial `state`, `view`, and
    /// `update` functions. The window defaults to [`WindowAttributes::default`]
    /// and no extra pipelines are registered.
    pub fn new(state: S, view: V, update: U) -> Self {
        Self {
            state,
            view,
            update,
            window_attrs: WindowAttributes::default(),
            extra_pipelines: None,
            exit_on_close: true,
            _marker: std::marker::PhantomData,
        }
    }

    /// Set the [`WindowAttributes`] used to create the window.
    pub fn window_attributes(mut self, window_attrs: WindowAttributes) -> Self {
        self.window_attrs = window_attrs;
        self
    }

    /// Control whether the event loop exits automatically when the window
    /// receives a close request ([`WindowEvent::CloseRequested`]).
    ///
    /// Defaults to `true`, so you don't need to wire up close handling in your
    /// `update` function. The close event is still delivered to `update` before
    /// the loop exits, so any cleanup there still runs. Pass `false` if you want
    /// to decide when to exit yourself (e.g. to prompt for unsaved changes).
    pub fn exit_on_close(mut self, exit_on_close: bool) -> Self {
        self.exit_on_close = exit_on_close;
        self
    }

    /// Register an extra render pipeline. Chainable:
    /// `.pipeline::<Planet>().pipeline::<Stars>()`.
    pub fn pipeline<P: Pipeline>(mut self) -> Self {
        self.extra_pipelines
            .get_or_insert_with(Vec::new)
            .push(PipelineRegistration::of::<P>());
        self
    }

    /// Register several at once. Registering the same pipeline type twice is
    /// harmless: the later build replaces the earlier one in the same slot.
    pub fn pipelines<I>(mut self, pipelines: I) -> Self
    where
        I: IntoIterator<Item = PipelineRegistration>,
    {
        self.extra_pipelines
            .get_or_insert_with(Vec::new)
            .extend(pipelines);
        self
    }

    /// Run the application. Blocks until the event loop exits.
    pub fn run(self) -> crate::Result<()> {
        run_app_core::<M, S, V, U>(
            self.state,
            self.view,
            self.update,
            self.window_attrs,
            self.extra_pipelines,
            self.exit_on_close,
        )
    }
}
