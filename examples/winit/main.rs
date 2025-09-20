use ui::{event::Event, graphics::Engine, pipeline_factories, render::pipeline::Pipeline};
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowAttributes,
};

#[path = "../common/mod.rs"]
mod common;
use common::{Message, State, View, pipeline::PlanetPipeline, view};

fn update<'a>(
    _engine: &mut Engine<'a, Message>,
    event: &Event<Message, WindowEvent>,
    state: &mut State,
    event_loop: &ActiveEventLoop,
) -> bool {
    if let Event::Platform(window_event) = event {
        match window_event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state: key_state,
                        ..
                    },
                ..
            } => match (key, key_state) {
                (KeyCode::Escape, _) => {
                    event_loop.exit();
                }
                (KeyCode::KeyN, ElementState::Pressed) => {
                    state.view = state.view.clone().next();
                    return true;
                }
                _ => (),
            },
            _ => (),
        }
    } else if let Event::Message(msg) = event {
        match msg {
            Message::ButtonPressed => state.counter += 1,
        }
    }

    false
}

fn main() {
    env_logger::init();
    let attrs = WindowAttributes::default().with_title("My Test GUI lib");

    _ = ui::winit::run_app_with::<Message, _, _, _, _>(
        State {
            view: View::Layout,
            counter: 0,
        },
        view,
        update,
        attrs,
        pipeline_factories!["planet" => PlanetPipeline],
    );
}
