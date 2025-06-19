use std::sync::Arc;
use ui::{
    graphics::Engine,
    model::*,
    widget::{Element, TextBox},
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
    fps: u32,
}

struct App<'a> {
    window: Option<Arc<Window>>,
    engine: Option<Engine<'a>>,
    state: State,
}

fn view(engine: &Engine, state: &State) -> Element {
    // let bg: Option<Element> = if let Some(h) = state.bg_handle {
    //     let (uv0, uv1) = engine.uv(h);
    //
    //     Some(
    //         Image {
    //             uv_min: uv0,
    //             uv_max: uv1,
    //             position: Position::from_scalar(0),
    //             size: Length::Fill,
    //             margin: Size::from_scalar(0),
    //             padding: Size::from_scalar(0),
    //             border_color: Color::BLACK,
    //             border_radius: 0.0,
    //             border_width: 0,
    //         }
    //         .into(),
    //     )
    // } else {
    //     None
    // };
    //
    let fps_box: Element = TextBox {
        content: format!("FPS: {}", state.fps),
        text_style: Style {
            font: ui::Family::Fantasy,
            font_size: 26.0,
            color: Color::BLACK,
            weight: ui::Weight::BOLD,
            ..Default::default()
        },
        position: Position::from_scalar(0),
        size: Length::Fill,
        margin: Size::from_scalar(12),
        padding: Size::from_scalar(32),
        border_radius: 80.0,
        border_color: Color::BLACK,
        border_width: 20,
        background_color: Color::from_rgb(250, 0, 50),
    }
    .into();

    let mut children = Vec::new();
    // if let Some(bg) = bg {
    //     children.push(bg);
    // }
    children.push(fps_box);

    children.into()
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

            // let test = engine.load_image("assets/background.jpg");
            // println!("{:?}", test);
            // self.state.bg_handle = test.ok();

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

                request_redraw(self);
            }
            WindowEvent::RedrawRequested => {
                let engine = self.engine.as_mut().unwrap();
                engine.view(|engine| view(engine, &self.state));

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
        state: State { fps: 144 },
    };
    event_loop.run_app(&mut app)
}

fn main() {
    _ = pollster::block_on(run());
}
