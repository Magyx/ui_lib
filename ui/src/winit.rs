use std::{collections::HashMap, sync::Arc};

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes},
};

use crate::{
    Size,
    event::{Event, ToEvent},
    graphics::{Config, Engine},
    model::Position,
    widget::Element,
};

impl<P> From<PhysicalSize<P>> for Size<P> {
    fn from(s: PhysicalSize<P>) -> Self {
        Size::new(s.width, s.height)
    }
}

pub type PipelineFactoryFn =
    fn(&Config, &[wgpu::PushConstantRange]) -> Box<dyn crate::render::pipeline::Pipeline>;

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
                for (key, pipeline_factory) in pipelines {
                    let pipeline = pipeline_factory(&engine.config, &engine.push_constant_ranges);
                    engine.register_pipeline(
                        crate::render::pipeline::PipelineKey::Other(key),
                        pipeline,
                    );
                }
            }
            self.engine = Some(engine);
            self.window = Some(window);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let engine = self.engine.as_mut().expect("engine not initialized");
        engine.handle_event(
            &event,
            &self.view,
            &mut self.update,
            &mut self.state,
            event_loop,
        );
    }
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
    let event_loop = EventLoop::new()?;
    let mut app = WinitApp::<'a, M, S, V, U>::new(state, view, update, window_attrs, None);
    event_loop.run_app(&mut app)
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
    let event_loop = EventLoop::new()?;
    let mut app =
        WinitApp::<'a, M, S, V, U>::new(state, view, update, window_attrs, Some(extra_pipelines));
    event_loop.run_app(&mut app)
}
