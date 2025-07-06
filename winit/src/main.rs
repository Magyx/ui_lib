use std::sync::Arc;
use ui::{
    event::Event,
    graphics::Engine,
    model::*,
    widget::{Column, Element, Length, Rectangle, Row, Widget},
};

use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

#[derive(Clone, Debug)]
enum Message {}

struct State {}

struct App<'a> {
    window: Option<Arc<Window>>,
    engine: Option<Engine<'a, Message>>,
    state: State,
}

fn update<'a>(
    _engine: &mut Engine<'a, Message>,
    event: &Event<Message, WindowEvent>,
    _state: &mut State,
    event_loop: &ActiveEventLoop,
) -> bool {
    if let Event::Platform(window_event) = event {
        match window_event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            }
            | WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::PinchGesture { .. } => todo!(),
            _ => (),
        }
    };

    false
}

fn view(_state: &State) -> Element<Message> {
    Column::new(vec![
        Row::new(vec![
            Rectangle::new(
                Size::new(Length::Fixed(200), Length::Fixed(100)),
                Color::RED,
            )
            .einto(),
            Rectangle::new(Size::new(Length::Grow, Length::Grow), Color::GREEN).einto(),
        ])
        .spacing(10)
        .color(Color::from_rgb(200, 10, 200))
        .padding(Vec4::splat(10))
        .size(Size::new(Length::Grow, Length::Grow))
        .einto(),
        Row::new(vec![
            Rectangle::new(
                Size::new(Length::Fixed(100), Length::Fixed(100)),
                Color::BLUE,
            )
            .einto(),
            Rectangle::new(
                Size::new(Length::Fixed(100), Length::Fixed(100)),
                Color::BLUE,
            )
            .einto(),
        ])
        .spacing(10)
        .color(Color::from_rgb(200, 10, 200))
        .padding(Vec4::splat(10))
        .size(Size::new(Length::Grow, Length::Grow))
        .einto(),
    ])
    .color(Color::from_rgb(100, 80, 100))
    .padding(Vec4::splat(20))
    .spacing(20)
    .size(Size::new(Length::Grow, Length::Grow))
    .einto()
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(WindowAttributes::default().with_title("My Test GUI lib"))
                    .expect("Failed to create window"),
            );

            let engine = Engine::new(window.clone(), window.inner_size().into());

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
        let engine = self.engine.as_mut().unwrap();

        engine.handle_event(&event, view, &mut update, &mut self.state, event_loop);
    }
}

async fn run() -> Result<(), EventLoopError> {
    env_logger::init();
    let event_loop = EventLoop::new()?;

    let mut app = App {
        window: None,
        engine: None,
        state: State {},
    };
    event_loop.run_app(&mut app)
}

fn main() {
    _ = pollster::block_on(run());
}
