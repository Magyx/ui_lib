use ui::{event::Event, graphics::Engine, pipeline_factories, render::pipeline::Pipeline};
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowAttributes,
};

#[path = "../common/mod.rs"]
mod common;
use common::{Message, State, pipeline::PlanetPipeline, view};

fn update<'a>(
    engine: &mut Engine<'a, Message>,
    event: &Event<Message, WindowEvent>,
    state: &mut State,
    event_loop: &ActiveEventLoop,
) -> bool {
    match event {
        Event::Platform(
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            },
        ) => event_loop.exit(),
        Event::Platform(WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::KeyN),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        }) => {
            state.view = state.view.clone().next();
            if let common::View::Texture = state.view
                && state.background.is_none()
            {
                if let Ok(reader) = image::ImageReader::open("assets/background.jpg")
                    && let Ok(img) = reader.decode()
                {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();

                    println!("Loaded image with dimensions: {}x{}", w, h);

                    let handle = engine.load_texture_rgba8(w, h, rgba.as_raw());

                    state.background = Some(handle);
                } else {
                    eprintln!("Couldn't load image!");
                }
            }
            return true;
        }
        Event::Message(msg) => match msg {
            Message::ButtonPressed => {
                state.counter += 1;
                return true;
            }
        },
        _ => (),
    }

    false
}

fn main() {
    env_logger::init();
    let attrs = WindowAttributes::default().with_title("My Test GUI lib");

    _ = ui::winit::run_app_with::<Message, _, _, _, _>(
        State::default(),
        view,
        update,
        attrs,
        pipeline_factories!["planet" => PlanetPipeline],
    );
}
