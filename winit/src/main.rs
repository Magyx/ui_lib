use std::sync::Arc;
use ui::{event::Event, graphics::Engine, model::*, widget::Element};

use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes},
};

#[derive(Clone, Debug)]
enum Message {}

struct App<'a> {
    window: Option<Arc<Window>>,
    engine: Option<Engine<'a, Message>>,
}

fn update<'a>(
    _engine: &mut Engine<'a, Message>,
    event: &Event<Message, WindowEvent>,
    _state: &mut (),
    event_loop: &ActiveEventLoop,
) -> Option<Message> {
    match event {
        Event::Platform(window_event) => match window_event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => (),
        },
        _ => (),
    };
    None
}

fn view(state: &()) -> Element<Message> {
    Element::new()
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

        engine.handle_event(&event, view, &mut update, &mut (), event_loop);
    }
}

async fn run() -> Result<(), EventLoopError> {
    env_logger::init();
    let event_loop = EventLoop::new()?;

    let mut app = App {
        window: None,
        engine: None,
    };
    event_loop.run_app(&mut app)
}

fn main() {
    _ = pollster::block_on(run());
}
