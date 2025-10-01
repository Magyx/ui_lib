use smol_str::ToSmolStr;
use ui::{
    event::{Event, KeyEvent, KeyState, LogicalKey},
    graphics::Engine,
    pipeline_factories,
    render::pipeline::Pipeline,
};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::WindowAttributes};

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
        Event::Platform(WindowEvent::CloseRequested) => {
            event_loop.exit();
            false
        }
        Event::Key(KeyEvent {
            state: KeyState::Pressed,
            logical_key: k,
            ..
        }) if k == &LogicalKey::Escape || k == &LogicalKey::Character("q".to_smolstr()) => {
            event_loop.exit();
            false
        }
        _ => common::update(engine, event, state),
    }
}

fn main() {
    #[cfg(feature = "env_logging")]
    {
        env_logger::init();
        log::info!("Starting winit example");
    }
    let attrs = WindowAttributes::default().with_title("My Test GUI lib");

    _ = ui::winit::run_app_with::<Message, _, _, _, _>(
        State::default(),
        view,
        update,
        attrs,
        pipeline_factories!["planet" => PlanetPipeline],
    );
}
