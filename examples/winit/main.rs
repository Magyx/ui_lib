use ui::prelude::*;

#[path = "../common/mod.rs"]
mod common;
use common::{Message, State, view};

fn update<'a>(
    target: TargetId,
    engine: &mut Engine<'a>,
    event: &Event<Message, WindowEvent>,
    state: &mut State,
    event_loop: &ActiveEventLoop,
) -> Task<Message> {
    match event {
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
        .run()
}
