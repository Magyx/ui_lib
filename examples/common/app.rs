use std::collections::{HashMap, VecDeque};

use ui::{
    event::{KeyEvent, KeyState, LogicalKey},
    graphics::{Engine, TargetId},
    widget::{Container, Element, Widget},
};

use super::demos;

#[derive(Clone)]
pub enum View {
    Layout = 0,
    Interaction = 1,
    Pipeline = 2,
    Texture = 3,
    Text = 4,
}

impl View {
    const COUNT: u8 = 5;

    fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Layout,
            1 => Self::Interaction,
            2 => Self::Pipeline,
            3 => Self::Texture,
            4 => Self::Text,
            _ => unreachable!("value out of range"),
        }
    }

    fn to_str(&self) -> &str {
        match self {
            View::Layout => "Layout",
            View::Interaction => "Interaction",
            View::Pipeline => "Pipeline",
            View::Texture => "Texture",
            View::Text => "Text",
        }
    }

    pub fn next(self) -> Self {
        Self::from_u8((self as u8 + 1) % Self::COUNT)
    }

    fn prev(self) -> View {
        Self::from_u8((self as u8 + Self::COUNT - 1) % Self::COUNT)
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    ButtonPressed,
}

pub struct Target {
    pub counter: u32,
    pub view: View,
    pub fps: VecDeque<f32>,
}

impl Default for Target {
    fn default() -> Self {
        Self {
            counter: 0,
            view: View::Layout,
            fps: VecDeque::with_capacity(5),
        }
    }
}

#[derive(Default)]
pub struct State {
    pub per_target: HashMap<TargetId, Target>,

    pub background: Option<ui::render::texture::TextureHandle>,
    pub icon_atlas: Option<ui::render::texture::Atlas>,
    pub icons: Vec<ui::render::texture::TextureHandle>,
}

mod update {
    use ui::graphics::{Engine, TargetId};

    pub fn ensure_icons_loaded<'a>(
        engine: &mut Engine<'a, super::Message>,
        state: &mut super::State,
    ) {
        if state.icon_atlas.is_some() {
            return;
        }

        let mut atlas = engine.create_atlas(1024, 1024);
        let mut handles = Vec::new();

        if let Ok(entries) = std::fs::read_dir("assets/open-iconic/png/") {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.ends_with("-8x.png"))
                    .unwrap_or(false)
                {
                    continue;
                }

                if let Ok(reader) = image::ImageReader::open(&path)
                    && let Ok(img) = reader.decode()
                {
                    let img = img.resize_exact(48, 48, image::imageops::FilterType::Triangle);
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    #[cfg(feature = "env_logging")]
                    log::info!(
                        "Loaded icon '{}' with dimensions: {}x{}",
                        path.display(),
                        w,
                        h
                    );

                    if let Some(handle) = engine.load_texture_into_atlas(&mut atlas, w, h, &rgba) {
                        handles.push(handle);
                    } else {
                        #[cfg(feature = "env_logging")]
                        log::warn!("Atlas is full, cannot add icon '{}'", path.display());
                    }
                } else {
                    #[cfg(feature = "env_logging")]
                    log::warn!("Couldn't load icon '{}'", path.display());
                }
            }
        }

        state.icon_atlas = Some(atlas);
        state.icons = handles;
    }

    fn ensure_background_loaded<'a>(
        engine: &mut Engine<'a, super::Message>,
        state: &mut super::State,
    ) {
        if state.background.is_some() {
            return;
        }
        if let Ok(reader) = image::ImageReader::open("assets/background.jpg")
            && let Ok(img) = reader.decode()
        {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();

            #[cfg(feature = "env_logging")]
            log::info!("Loaded image with dimensions: {}x{}", w, h);

            let handle = engine.load_texture_rgba8(w, h, rgba.as_raw());

            state.background = Some(handle);
        } else {
            #[cfg(feature = "env_logging")]
            log::warn!("Couldn't load image!");
        }
    }

    pub fn cycle_view<'a>(
        tid: TargetId,
        engine: &mut Engine<'a, super::Message>,
        state: &mut super::State,
        dir: bool,
    ) -> bool {
        let target = match state.per_target.get_mut(&tid) {
            Some(t) => t,
            None => return false,
        };
        if dir {
            target.view = target.view.clone().next();
        } else {
            target.view = target.view.clone().prev();
        }
        if let super::View::Texture = target.view {
            ensure_background_loaded(engine, state);
            ensure_icons_loaded(engine, state);
        }

        true
    }

    pub fn increment_counter(target: &mut super::Target) -> bool {
        target.counter += 1;
        true
    }

    pub fn toggle_debug<'a>(engine: &mut Engine<'a, super::Message>) -> bool {
        engine.toggle_debug();
        true
    }
}

pub fn update<'a, E: ui::event::ToEvent<Message, E>>(
    tid: TargetId,
    engine: &mut Engine<'a, Message>,
    event: &crate::Event<Message, E>,
    state: &mut State,
) -> bool {
    let target = state.per_target.entry(tid).or_default();
    match event {
        crate::Event::RedrawRequested => {
            if target.fps.len() == 5 {
                target.fps.pop_front();
            }
            target
                .fps
                .push_back(1.0 / engine.globals(tid).unwrap().delta_time);
            false
        }
        crate::Event::Key(KeyEvent {
            state: KeyState::Pressed,
            logical_key: k,
            ..
        }) => match k {
            LogicalKey::F(12) => update::toggle_debug(engine),
            LogicalKey::Character(s) => match s.as_str() {
                "n" => update::cycle_view(tid, engine, state, true),
                "p" => update::cycle_view(tid, engine, state, false),
                _ => false,
            },
            _ => false,
        },
        crate::Event::Message(Message::ButtonPressed) => update::increment_counter(target),
        _ => false,
    }
}

pub fn view(tid: &TargetId, state: &State) -> Element<Message> {
    let target = match state.per_target.get(tid) {
        Some(t) => t,
        None => return Container::new(vec![]).einto(),
    };
    match target.view {
        View::Layout => demos::layout::view(state),
        View::Interaction => demos::interaction::view(tid, state),
        View::Pipeline => demos::pipeline::view(tid, state),
        View::Texture => demos::texture::view(state),
        View::Text => demos::text::view(state),
    }
}
