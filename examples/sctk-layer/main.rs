use ui::prelude::*;

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

    let opts = LayerOptions {
        layer: Layer::Background,
        size: Size::new(0, 0),
        anchors: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        exclusive_zone: -1,
        keyboard_interactivity: KeyboardInteractivity::OnDemand,
        namespace: Some("ui-example".to_string()),
        output: Some(ui::sctk::OutputSet::All),
    };

    SctkApp::layer(State::default(), view, update, opts).run()
}
