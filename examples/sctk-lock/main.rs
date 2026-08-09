use ui::{
    event::{Event, KeyEvent, KeyState, LogicalKey},
    graphics::{Engine, TargetId},
    sctk::{LockOptions, OutputSet, SctkApp, SctkEvent, SctkLoop},
    task::Task,
};

#[path = "../common/mod.rs"]
mod common;
use common::{Message, State, view};

fn update<'a>(
    target: TargetId,
    engine: &mut Engine<'a>,
    event: &Event<Message, SctkEvent>,
    state: &mut State,
    loop_ctl: &SctkLoop,
) -> Task<Message> {
    match event {
        Event::Key(KeyEvent {
            state: KeyState::Pressed,
            logical_key: k,
            ..
        }) if k == &LogicalKey::Escape => {
            loop_ctl.exit();
            Task::None
        }
        _ => common::update(target, engine, event, state),
    }
}

fn main() -> ui::Result<()> {
    #[cfg(feature = "tracing")]
    {
        crate::common::trace::init();
        tracing::info!("Starting SCTK example");
    }

    let opts = LockOptions {
        output: Some(OutputSet::All),
        ..Default::default()
    };

    SctkApp::lock(State::default(), view, update, opts)
        .run()
}
