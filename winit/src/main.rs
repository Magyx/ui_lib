use std::sync::Arc;
use ui::{
    graphics::{Engine, TextureHandle},
    model::*,
    widget::{BorderStyle, ContentFit, Element, Image, Layout, Length, Rectangle, Row, TextBox},
};

use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

struct State {
    window_size: Size<u32>,
    bg_handle: Option<TextureHandle>,
}

struct App<'a> {
    window: Option<Arc<Window>>,
    engine: Option<Engine<'a>>,
    state: State,
}

fn build_ui(state: &State) -> Element {
    let bg: Element = state
        .bg_handle
        .map(|texture_handle| {
            Image {
                texture_handle,
                layout: Layout::default(),
                border: BorderStyle::default(),
                fit: ContentFit::Cover,
            }
            .into()
        })
        .unwrap_or(
            Rectangle {
                layout: Layout::default(),
                border: BorderStyle::default(),
                background_color: Color::from_rgb(123, 123, 123),
            }
            .into(),
        );

    let actions = Row {
        children: vec![],
        layout: Layout {
            size: Length::Fit,
            align: Vector2::from_scalar(0.5),
            ..Default::default()
        },
        spacing: 12,
        border: BorderStyle::default(),
        background_color: Color::from_rgba(50, 50, 50, 120),
    }
    .into();

    vec![bg, actions].into()
}

fn request_redraw(app: &mut App) {
    let window = app.window.as_ref().unwrap();
    window.request_redraw();
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(WindowAttributes::default().with_title("My Test GUI lib"))
                    .expect("Failed to create window"),
            );

            let mut engine = Engine::new(window.clone(), window.inner_size().into());

            let img = image::open("assets/background.jpg").unwrap();
            self.state.bg_handle = engine.load_texture(img).ok();

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
        let state = self.engine.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::F5),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                self.engine.as_mut().unwrap().reload_all();
                request_redraw(self);
            }
            WindowEvent::Resized(size) => {
                state.resize(size.into());
                self.state.window_size = size.into();

                request_redraw(self);
            }
            WindowEvent::RedrawRequested => {
                let engine = self.engine.as_mut().unwrap();
                _ = engine.view(|| build_ui(&self.state));

                _ = engine.render();
            }
            _ => (),
        }
    }
}

async fn run() -> Result<(), EventLoopError> {
    env_logger::init();
    let event_loop = EventLoop::new()?;

    let mut app = App {
        window: None,
        engine: None,
        state: State {
            bg_handle: None,
            window_size: Size::from_scalar(0),
        },
    };
    event_loop.run_app(&mut app)
}

fn main() {
    _ = pollster::block_on(run());
}
