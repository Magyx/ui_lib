use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use smol_str::ToSmolStr;
use ui::{
    event::{Event, KeyEvent, KeyState, LogicalKey},
    graphics::Engine,
    model::Size,
    pipeline_factories,
    render::pipeline::Pipeline,
    sctk::{DefaultHandler, LayerOptions, OutputSelector, SctkEvent, SctkLoop},
};

#[path = "../common/mod.rs"]
mod common;
use common::{Message, State, pipeline::PlanetPipeline, view};

fn update<'a>(
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
        _ => common::update(engine, event, state),
    }
}

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "env_logging")]
    {
        env_logger::init();
        log::info!("Starting SCTK example");
    }

    let opts = LayerOptions {
        layer: Layer::Background,
        size: Size::new(0, 0),
        anchors: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        exclusive_zone: -1,
        keyboard_interactivity: KeyboardInteractivity::OnDemand,
        namespace: Some("ui-example"),
        output: Some(OutputSelector::HighestScale),
    };

    ui::sctk::run_app_with::<Message, State, DefaultHandler, _, _, _>(
        State::default(),
        view,
        update,
        opts,
        pipeline_factories!["planet" => PlanetPipeline],
    )
}
