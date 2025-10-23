use smol_str::ToSmolStr;
use ui::{
    event::{Event, KeyEvent, KeyState, LogicalKey},
    graphics::{Engine, TargetId},
    pipeline_factories,
    render::pipeline::Pipeline,
    sctk::{DefaultHandler, SctkEvent, SctkLoop, XdgOptions},
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
        }) if k == &LogicalKey::Escape || k == &LogicalKey::Character("q".to_smolstr()) => {
            loop_ctl.exit();
            false
        }
        _ => common::update(target, engine, event, state),
    }
}

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "env_logging")]
    {
        env_logger::init();
        log::info!("Starting SCTK example");
    }

    let opts = XdgOptions {
        title: "ui — XDG toplevel".into(),
        app_id: Some("ui-example".into()),
        ..Default::default()
    };
    ui::sctk::run_app_with::<Message, State, DefaultHandler, _, _, _>(
        State::default(),
        view,
        update,
        opts,
        pipeline_factories!["planet" => PlanetPipeline],
    )
}
