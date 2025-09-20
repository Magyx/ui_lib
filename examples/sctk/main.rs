use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use ui::{
    event::Event,
    graphics::Engine,
    model::Size,
    pipeline_factories,
    render::pipeline::Pipeline,
    sctk::{DefaultHandler, LayerOptions, OutputSelector, SctkEvent, SctkLoop},
    widget::Element,
};

use crate::pipeline::PlanetPipeline;

mod demos;
mod pipeline;

#[derive(Clone)]
enum View {
    Layout = 0,
    Interaction = 1,
    Pipeline,
}

impl View {
    const COUNT: u8 = 3;

    fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Layout,
            1 => Self::Interaction,
            2 => Self::Pipeline,
            _ => unreachable!("value out of range"),
        }
    }

    fn next(self) -> Self {
        Self::from_u8((self as u8 + 1) % Self::COUNT)
    }
}

#[derive(Clone, Debug)]
enum Message {
    ButtonPressed,
}

struct State {
    counter: u32,
    view: View,
}

fn update<'a>(
    _engine: &mut Engine<'a, Message>,
    event: &Event<Message, SctkEvent>,
    state: &mut State,
    loop_ctl: &SctkLoop,
) -> bool {
    match event {
        Event::Platform(SctkEvent::Closed) => {
            loop_ctl.exit();
        }
        Event::KeyboardInput { char } if *char == b'q' => {
            loop_ctl.exit();
        }
        Event::KeyboardInput { char } if *char == b'n' => {
            state.view = state.view.clone().next();
            return true;
        }
        Event::Message(Message::ButtonPressed) => {
            state.counter += 1;
            return true;
        }
        _ => {}
    }

    false
}
fn view(state: &State) -> Element<Message> {
    match state.view {
        View::Layout => demos::layout::view(state),
        View::Interaction => demos::interaction::view(state),
        View::Pipeline => demos::pipeline::view(state),
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let opts = LayerOptions {
        layer: Layer::Top,
        size: Size::new(0, 100),
        anchors: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
        exclusive_zone: 100,
        keyboard_interactivity: KeyboardInteractivity::OnDemand,
        namespace: Some("ui-example"),
        output: Some(OutputSelector::Index(2)),
    };

    ui::sctk::run_app_with::<Message, State, DefaultHandler, _, _, _>(
        State {
            view: View::Layout,
            counter: 0,
        },
        view,
        update,
        opts,
        pipeline_factories!["planet" => PlanetPipeline],
    )
}
