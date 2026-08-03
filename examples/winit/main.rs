use ui::{
    event::{Event, KeyEvent, KeyState, LogicalKey},
    graphics::{Engine, TargetId},
    pipeline_factories,
    task::Task,
    winit::WinitApp,
};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::WindowAttributes};

#[path = "../common/mod.rs"]
mod common;
use common::{Message, State, pipeline::PlanetPipeline, view};

fn update<'a>(
    target: TargetId,
    engine: &mut Engine<'a>,
    event: &Event<Message, WindowEvent>,
    state: &mut State,
    event_loop: &ActiveEventLoop,
) -> Task<Message> {
    match event {
        Event::Platform(WindowEvent::CloseRequested) => {
            event_loop.exit();
            Task::None
        }
        Event::Key(KeyEvent {
            state: KeyState::Pressed,
            logical_key: k,
            ..
        }) if k == &LogicalKey::Escape => {
            event_loop.exit();
            Task::None
        }
        _ => common::update(target, engine, event, state),
    }
}

fn main() -> ui::Result<()> {
    #[cfg(feature = "tracing")]
    {
        crate::common::trace::init();
        tracing::info!("Starting winit example");
    }
    let attrs = WindowAttributes::default().with_title("My Test GUI lib");

    WinitApp::builder(State::default(), view, update)
        .window_attributes(attrs)
        .pipelines(pipeline_factories!["planet" => PlanetPipeline])
        .run()
}
