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

use crate::common::update;

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
        ) => {
            event_loop.exit();
            false
        }
        Event::Platform(WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::KeyN),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        }) => update::cycle_view(engine, state),
        Event::Message(Message::ButtonPressed) => update::increment_counter(state),
        _ => false,
    }
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
