use ui::{
    event::{Event, KeyEvent, KeyState, LogicalKey},
    graphics::{Engine, TargetId},
    pipeline_factories,
    render::pipeline::Pipeline,
    sctk::{DefaultHandler, LockOptions, OutputSet, SctkEvent, SctkLoop},
};

#[path = "../common/mod.rs"]
mod common;
use common::{Message, State, pipeline::PlanetPipeline, view};

fn update<'a>(
    target: TargetId,
    engine: &mut Engine<'a, Message>,
    event: &Event<Message, SctkEvent>,
    state: &mut State,
    loop_ctl: &SctkLoop,
) -> bool {
    match event {
        Event::Platform(SctkEvent::Closed) => {
            loop_ctl.exit();
            false
        }
        Event::Key(KeyEvent {
            state: KeyState::Pressed,
            logical_key: k,
            ..
        }) if k == &LogicalKey::Escape => {
            loop_ctl.exit();
            false
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
    ui::sctk::run_lock_with::<Message, State, DefaultHandler, _, _, _>(
        State::default(),
        view,
        update,
        opts,
        pipeline_factories!["planet" => PlanetPipeline],
    )
}
