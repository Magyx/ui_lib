use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use ui::{
    event::Event,
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
        Event::Platform(SctkEvent::Closed) | Event::KeyboardInput { char: b'q' } => {
            loop_ctl.exit();
        }
        Event::KeyboardInput { char: b'n' } => {
            state.view = state.view.clone().next();
            if let common::View::Texture = state.view
                && state.background.is_none()
            {
                if let Ok(reader) = image::ImageReader::open("assets/background.jpg")
                    && let Ok(img) = reader.decode()
                {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    println!("Loaded image with dimensions: {}x{}", w, h);

                    let handle = engine.load_texture_rgba8(w, h, rgba.as_raw());

                    state.background = Some(handle);
                } else {
                    eprintln!("Couldn't load image!");
                }
            }
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
        State::default(),
        view,
        update,
        opts,
        pipeline_factories!["planet" => PlanetPipeline],
    )
}
