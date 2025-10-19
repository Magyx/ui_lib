use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use smol_str::ToSmolStr;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key as WKey, KeyLocation as WLoc, PhysicalKey as WPhys},
    window::{Window, WindowAttributes},
};

use crate::{
    Size,
    event::{
        Event, KeyEvent, KeyLocation, KeyState, LogicalKey, Modifiers, PhysicalKey, TextInput,
        ToEvent,
    },
    graphics::{Engine, TargetId},
    model::Position,
    render::PipelineFactoryFn,
    widget::Element,
};

impl<P> From<PhysicalSize<P>> for Size<P> {
    fn from(s: PhysicalSize<P>) -> Self {
        Size::new(s.width, s.height)
    }
}

fn map_winit_logical(k: &WKey) -> LogicalKey {
    use winit::keyboard::NamedKey;
    match k {
        WKey::Character(s) => LogicalKey::Character(s.to_smolstr()),
        WKey::Named(n) => match n {
            NamedKey::Enter => LogicalKey::Enter,
            NamedKey::Escape => LogicalKey::Escape,
            NamedKey::Backspace => LogicalKey::Backspace,
            NamedKey::Tab => LogicalKey::Tab,
            NamedKey::Space => LogicalKey::Space,
            NamedKey::ArrowLeft => LogicalKey::ArrowLeft,
            NamedKey::ArrowRight => LogicalKey::ArrowRight,
            NamedKey::ArrowUp => LogicalKey::ArrowUp,
            NamedKey::ArrowDown => LogicalKey::ArrowDown,
            NamedKey::Home => LogicalKey::Home,
            NamedKey::End => LogicalKey::End,
            NamedKey::PageUp => LogicalKey::PageUp,
            NamedKey::PageDown => LogicalKey::PageDown,
            NamedKey::Insert => LogicalKey::Insert,
            NamedKey::Delete => LogicalKey::Delete,
            NamedKey::F1 => LogicalKey::F(1),
            NamedKey::F2 => LogicalKey::F(2),
            NamedKey::F3 => LogicalKey::F(3),
            NamedKey::F4 => LogicalKey::F(4),
            NamedKey::F5 => LogicalKey::F(5),
            NamedKey::F6 => LogicalKey::F(6),
            NamedKey::F7 => LogicalKey::F(7),
            NamedKey::F8 => LogicalKey::F(8),
            NamedKey::F9 => LogicalKey::F(9),
            NamedKey::F10 => LogicalKey::F(10),
            NamedKey::F11 => LogicalKey::F(11),
            NamedKey::F12 => LogicalKey::F(12),
            NamedKey::F13 => LogicalKey::F(13),
            NamedKey::F14 => LogicalKey::F(14),
            NamedKey::F15 => LogicalKey::F(15),
            NamedKey::F16 => LogicalKey::F(16),
            NamedKey::F17 => LogicalKey::F(17),
            NamedKey::F18 => LogicalKey::F(18),
            NamedKey::F19 => LogicalKey::F(19),
            NamedKey::F20 => LogicalKey::F(20),
            NamedKey::F21 => LogicalKey::F(21),
            NamedKey::F22 => LogicalKey::F(22),
            NamedKey::F23 => LogicalKey::F(23),
            NamedKey::F24 => LogicalKey::F(24),
            _ => LogicalKey::Unknown,
        },
        _ => LogicalKey::Unknown,
    }
}

fn map_winit_physical(p: &WPhys) -> PhysicalKey {
    match p {
        WPhys::Code(code) => PhysicalKey::Code(*code as u32),
        WPhys::Unidentified(_) => PhysicalKey::Unidentified,
    }
}

fn map_winit_location(l: WLoc) -> KeyLocation {
    match l {
        WLoc::Standard => KeyLocation::Standard,
        WLoc::Left => KeyLocation::Left,
        WLoc::Right => KeyLocation::Right,
        WLoc::Numpad => KeyLocation::Numpad,
    }
}

impl<M> ToEvent<M, winit::event::WindowEvent> for winit::event::WindowEvent {
    fn to_event(&self) -> Event<M, Self> {
        use winit::event::{ElementState, WindowEvent as WE};

        match self {
            WE::RedrawRequested => Event::RedrawRequested,
            WE::Resized(size) => Event::Resized {
                size: (*size).into(),
            },
            WE::CursorMoved { position, .. } => Event::CursorMoved {
                position: Position::new(position.x as f32, position.y as f32),
            },
            WE::MouseInput { state, .. } => Event::MouseInput {
                mouse_down: state.is_pressed(),
            },
            WE::KeyboardInput { event, .. } => {
                let state = match event.state {
                    ElementState::Pressed => KeyState::Pressed,
                    ElementState::Released => KeyState::Released,
                };

                let logical_key = map_winit_logical(&event.logical_key);
                let physical_key = map_winit_physical(&event.physical_key);
                let location = map_winit_location(event.location);

                Event::Key(KeyEvent {
                    state,
                    repeat: event.repeat,
                    logical_key,
                    physical_key,
                    location,
                    modifiers: Modifiers::default(),
                })
            }
            WE::Ime(winit::event::Ime::Commit(s)) => Event::Text(TextInput { text: s.clone() }),
            WE::ModifiersChanged(m) => Event::ModifiersChanged(Modifiers {
                shift: m.state().shift_key(),
                control: m.state().control_key(),
                alt: m.state().alt_key(),
                super_: m.state().super_key(),
                caps_lock: None,
                num_lock: None,
            }),
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
    V: Fn(&TargetId, &S) -> Element<M> + 'static,
    U: FnMut(
            TargetId,
            &mut Engine<'a, M>,
            &Event<M, WindowEvent>,
            &mut S,
            &ActiveEventLoop,
        ) -> bool
        + 'static,
{
    window: Option<Arc<Window>>,
    target: Option<TargetId>,
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
    V: Fn(&TargetId, &S) -> Element<M> + 'static,
    U: FnMut(
            TargetId,
            &mut Engine<'a, M>,
            &Event<M, WindowEvent>,
            &mut S,
            &ActiveEventLoop,
        ) -> bool
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
            target: None,
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
    V: Fn(&TargetId, &S) -> Element<M> + 'static,
    U: FnMut(
            TargetId,
            &mut Engine<'a, M>,
            &Event<M, WindowEvent>,
            &mut S,
            &ActiveEventLoop,
        ) -> bool
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
            let (target, mut engine) = Engine::new_for(window.clone(), size);
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
        _window_id: winit::window::WindowId,
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
                engine.render_if_needed(
                    &self.target.unwrap(),
                    should_redraw,
                    &self.view,
                    &mut self.state,
                );
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
    V: Fn(&TargetId, &S) -> Element<M> + 'static,
    U: FnMut(
            TargetId,
            &mut Engine<'a, M>,
            &Event<M, WindowEvent>,
            &mut S,
            &ActiveEventLoop,
        ) -> bool
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
    V: Fn(&TargetId, &S) -> Element<M> + 'static,
    U: FnMut(
            TargetId,
            &mut Engine<'a, M>,
            &Event<M, WindowEvent>,
            &mut S,
            &ActiveEventLoop,
        ) -> bool
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
    V: Fn(&TargetId, &S) -> Element<M> + 'static,
    U: FnMut(
            TargetId,
            &mut Engine<'a, M>,
            &Event<M, WindowEvent>,
            &mut S,
            &ActiveEventLoop,
        ) -> bool
        + 'static,
    I: IntoIterator<Item = (&'static str, PipelineFactoryFn)>,
{
    let extra_pipelines: HashMap<&'static str, PipelineFactoryFn> =
        extra_pipelines.into_iter().collect();
    run_app_core(state, view, update, window_attrs, Some(extra_pipelines))
}
