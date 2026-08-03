use ui::{
    event::{Event, KeyEvent, KeyState, LogicalKey},
    graphics::{Engine, TargetId},
    pipeline_factories,
    sctk::{SctkApp, SctkEvent, SctkLoop, XdgOptions},
    task::Task,
};

#[path = "../common/mod.rs"]
mod common;
use common::{Message, State, pipeline::PlanetPipeline, view};

fn update<'a>(
    target: TargetId,
    engine: &mut Engine<'a>,
    event: &Event<Message, SctkEvent>,
    state: &mut State,
    loop_ctl: &SctkLoop,
) -> Task<Message> {
    match event {
        Event::Platform(SctkEvent::Closed) => {
            loop_ctl.exit();
            Task::None
        }
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

    let opts = XdgOptions {
        title: "ui — XDG toplevel".into(),
        app_id: Some("ui-example".into()),
        ..Default::default()
    };

    SctkApp::window(State::default(), view, update, opts)
        .pipelines(pipeline_factories!["planet" => PlanetPipeline])
        .run()
}
