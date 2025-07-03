use std::sync::Arc;
use ui::{
    button,
    event::Event,
    graphics::{Engine, TextureHandle},
    model::*,
    text_box,
    widget::{BorderStyle, ContentFit, Element, Layout, Length, TextStyle, image, rectangle, row},
};

use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes},
};

struct State {
    show_message: bool,
    bg_handle: Option<TextureHandle>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            show_message: false,
            bg_handle: None,
        }
    }
}

#[derive(Clone, Debug)]
enum Message {
    ToggleMessage,
    Reload,
    Exit,
}

struct App<'a> {
    window: Option<Arc<Window>>,
    engine: Option<Engine<'a, Message>>,
    state: State,
}

fn update<'a>(
    engine: &mut Engine<'a, Message>,
    event: &Event<Message, WindowEvent>,
    state: &mut State,
    event_loop: &ActiveEventLoop,
) -> Option<Message> {
    match event {
        Event::Message(m) => match m {
            Message::ToggleMessage => state.show_message = !state.show_message,
            Message::Reload => engine.reload_all(),
            Message::Exit => event_loop.exit(),
        },
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

fn view(state: &State) -> Element<Message> {
    let button_layout = Layout {
        size: Length::Fit,
        padding: Size::splat(8),
        ..Default::default()
    };
    let button_border = BorderStyle {
        radius: 12.0,
        ..Default::default()
    };
    vec![
        state
            .bg_handle
            .map(|texture_handle| image!(texture_handle).fit(ContentFit::Cover).into())
            .unwrap_or(rectangle!(Color::from_rgb(123, 123, 123)).into()),
        text_box!("Test")
            .text_style(TextStyle {
                font_size: 32.0,
                ..Default::default()
            })
            .layout(Layout {
                size: Length::Fit,
                ..Default::default()
            })
            .into(),
        row![
            button!("Toggle Message")
                .layout(button_layout)
                .border(button_border)
                .background_color(Color::from_rgb(211, 211, 211))
                .hover_color(Color::from_rgb(238, 238, 238))
                .pressed_color(Color::from_rgb(182, 182, 182))
                .on_click(Message::ToggleMessage),
            // button!("Reload")
            //     .layout(button_layout)
            //     .border(button_border)
            //     .background_color(Color::from_rgb(211, 211, 211))
            //     .hover_color(Color::from_rgb(238, 238, 238))
            //     .pressed_color(Color::from_rgb(182, 182, 182))
            //     .on_click(Message::Reload),
            // button!("Exit")
            //     .layout(button_layout)
            //     .border(button_border)
            //     .background_color(Color::from_rgb(211, 211, 211))
            //     .hover_color(Color::from_rgb(238, 238, 238))
            //     .pressed_color(Color::from_rgb(182, 182, 182))
            //     .on_click(Message::Exit),
        ]
        .layout(Layout {
            size: Length::Fit,
            align: Vector2::splat(0.5),
            padding: Size::splat(12),
            ..Default::default()
        })
        .border(BorderStyle {
            radius: 24.0,
            ..Default::default()
        })
        .spacing(12)
        .background_color(Color::from_rgba(50, 50, 50, 120))
        .into(),
    ]
    .into()
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
        state: State::default(),
    };
    event_loop.run_app(&mut app)
}

fn main() {
    _ = pollster::block_on(run());
}
