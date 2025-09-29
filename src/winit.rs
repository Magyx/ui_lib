use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

use crate::{
    Size,
    event::{Event, ToEvent},
    graphics::Engine,
    model::Position,
    render::PipelineFactoryFn,
    widget::Element,
};

impl<P> From<PhysicalSize<P>> for Size<P> {
    fn from(s: PhysicalSize<P>) -> Self {
        Size::new(s.width, s.height)
    }
}

impl<M> ToEvent<M, winit::event::WindowEvent> for winit::event::WindowEvent {
    fn to_event(&self) -> Event<M, Self> {
        match self {
            WindowEvent::RedrawRequested => Event::RedrawRequested,
            WindowEvent::Resized(size) => Event::Resized {
                size: (*size).into(),
            },
            WindowEvent::CursorMoved { position, .. } => Event::CursorMoved {
                position: Position::new(position.x as f32, position.y as f32),
            },
            WindowEvent::MouseInput { state, .. } => Event::MouseInput {
                mouse_down: state.is_pressed(),
            },
            _ => Event::Platform(self.clone()),
        }
    }
}

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
    M: 'static + std::fmt::Debug,
    V: Fn(&S) -> Element<M> + 'static,
    U: FnMut(&mut Engine<'a, M>, &Event<M, WindowEvent>, &mut S, &ActiveEventLoop) -> bool
        + 'static,
{
    window: Option<Arc<Window>>,
    engine: Option<Engine<'a, M>>,
    extra_pipelines: Option<HashMap<&'static str, PipelineFactoryFn>>,
    state: S,
    view: V,
    update: U,
    window_attrs: WindowAttributes,
    next_frame: Instant,
    frame_interval: Duration,
}

impl<'a, M, S, V, U> WinitApp<'a, M, S, V, U>
where
    M: 'static + std::fmt::Debug,
    V: Fn(&S) -> Element<M> + 'static,
    U: FnMut(&mut Engine<'a, M>, &Event<M, WindowEvent>, &mut S, &ActiveEventLoop) -> bool
        + 'static,
{
    pub fn new(
        state: S,
        view: V,
        update: U,
        window_attrs: WindowAttributes,
        extra_pipelines: Option<HashMap<&'static str, PipelineFactoryFn>>,
    ) -> Self {
        Self {
            window: None,
            engine: None,
            extra_pipelines,
            state,
            view,
            update,
            window_attrs,
            next_frame: Instant::now(),
            frame_interval: Duration::from_millis(16),
        }
    }
}

impl<'a, M, S, V, U> ApplicationHandler for WinitApp<'a, M, S, V, U>
where
    M: 'static + std::fmt::Debug,
    V: Fn(&S) -> Element<M> + 'static,
    U: FnMut(&mut Engine<'a, M>, &Event<M, WindowEvent>, &mut S, &ActiveEventLoop) -> bool
        + 'static,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(self.window_attrs.clone())
                    .expect("Failed to create window"),
            );
            let size = window.inner_size().into();
            let mut engine = Engine::new(window.clone(), size);
            if let Some(pipelines) = self.extra_pipelines.take() {
                for (key, factory) in pipelines {
                    engine.register_pipeline(
                        crate::render::pipeline::PipelineKey::Other(key),
                        factory,
                    );
                }
            }

            self.frame_interval = frame_interval_from_monitor(&window);
            self.engine = Some(engine);
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
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                let engine = self.engine.as_mut().unwrap();
                let should_redraw = engine.poll(&mut self.update, &mut self.state, event_loop);
                engine.render_if_needed(should_redraw, &self.view, &mut self.state);
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
                engine.handle_platform_event(&event, &mut self.update, &mut self.state, event_loop);
            }
        }
    }
}

fn run_app_core<'a, M, S, V, U>(
    state: S,
    view: V,
    update: U,
    window_attrs: WindowAttributes,
    extra_pipelines: Option<HashMap<&'static str, PipelineFactoryFn>>,
) -> Result<(), EventLoopError>
where
    M: 'static + std::fmt::Debug,
    V: Fn(&S) -> Element<M> + 'static,
    U: FnMut(&mut Engine<'a, M>, &Event<M, WindowEvent>, &mut S, &ActiveEventLoop) -> bool
        + 'static,
{
    let event_loop = EventLoop::new()?;
    let mut app =
        WinitApp::<'a, M, S, V, U>::new(state, view, update, window_attrs, extra_pipelines);
    event_loop.run_app(&mut app)
}

pub fn run_app<'a, M, S, V, U>(
    state: S,
    view: V,
    update: U,
    window_attrs: WindowAttributes,
) -> Result<(), EventLoopError>
where
    M: 'static + std::fmt::Debug,
    V: Fn(&S) -> Element<M> + 'static,
    U: FnMut(&mut Engine<'a, M>, &Event<M, WindowEvent>, &mut S, &ActiveEventLoop) -> bool
        + 'static,
{
    run_app_core(state, view, update, window_attrs, None)
}

pub fn run_app_with<'a, M, S, V, U, I>(
    state: S,
    view: V,
    update: U,
    window_attrs: WindowAttributes,
    extra_pipelines: I,
) -> Result<(), EventLoopError>
where
    M: 'static + std::fmt::Debug,
    V: Fn(&S) -> Element<M> + 'static,
    U: FnMut(&mut Engine<'a, M>, &Event<M, WindowEvent>, &mut S, &ActiveEventLoop) -> bool
        + 'static,
    I: IntoIterator<Item = (&'static str, PipelineFactoryFn)>,
{
    let extra_pipelines: HashMap<&'static str, PipelineFactoryFn> =
        extra_pipelines.into_iter().collect();
    run_app_core(state, view, update, window_attrs, Some(extra_pipelines))
}
