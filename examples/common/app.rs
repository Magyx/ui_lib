use std::{collections::HashMap, path::PathBuf};

use ui::event::{KeyEvent, KeyState, LogicalKey};
use ui::prelude::*;

use super::demos;

#[derive(Clone, Copy)]
pub enum View {
    Layout = 0,
    Interaction = 1,
    Pipeline = 2,
    Texture = 3,
    Text = 4,
    Scrollable = 5,
    ThemeEditor = 6,
}

#[allow(dead_code)]
impl View {
    const COUNT: u8 = 7;

    fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Layout,
            1 => Self::Interaction,
            2 => Self::Pipeline,
            3 => Self::Texture,
            4 => Self::Text,
            5 => Self::Scrollable,
            6 => Self::ThemeEditor,
            _ => unreachable!("value out of range"),
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
    SliderChanged(f32),
    NameChanged(String),
    TextAreaContentChanged(String),
    ThemeSetDark,
    ThemeSetLight,
    ThemeCornerRadius(f32),
    ThemeBorderWidth(f32),
}

#[derive(Clone)]
pub struct Target {
    pub counter: u32,
    pub view: View,
    pub fps: [f32; 5],
    pub fps_idx: usize,

    pub slider: f32,
    pub name: String,
    pub text_area_content: String,
}

impl Default for Target {
    fn default() -> Self {
        Self {
            counter: 0,
            view: View::Layout,
            fps: [0.0; 5],
            fps_idx: Default::default(),

            slider: 50.0,
            name: String::new(),
            text_area_content: String::new(),
        }
    }
}

#[derive(Default)]
pub struct State {
    pub per_target: HashMap<TargetId, Target>,

    pub theme: Theme,
    pub background: Option<ui::render::texture::TextureHandle>,
    pub icon_atlas: Option<ui::render::texture::Atlas>,
    pub icons: Vec<ui::render::texture::TextureHandle>,
    pub svg_icons: Vec<PathBuf>,
}

mod update {
    use ui::{
        graphics::{Engine, TargetId},
        render::AllocatorKind,
    };

    pub fn ensure_icons_loaded<'a>(
        engine: &mut Engine<'a, super::Message>,
        state: &mut super::State,
        scale: f32,
    ) {
        if state.icon_atlas.is_some() {
            return;
        }

        const MAX_DEMO_ICONS: usize = 16;

        let icon_phys = (48.0 * scale).round() as u32;
        let atlas_phys = (512.0 * scale).round() as u32;
        let mut atlas = engine.create_atlas(atlas_phys, atlas_phys, AllocatorKind::Shelf);
        let mut handles = Vec::new();
        let mut svg_paths = Vec::with_capacity(MAX_DEMO_ICONS);

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
                    let img = img.resize_exact(
                        icon_phys,
                        icon_phys,
                        image::imageops::FilterType::Triangle,
                    );
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    #[cfg(feature = "tracing")]
                    tracing::info!(
                        "Loaded png icon '{}' with dimensions: {}x{}",
                        path.display(),
                        w,
                        h
                    );

                    if let Some(handle) = engine.load_texture_into_atlas(&mut atlas, w, h, &rgba) {
                        handles.push(handle);
                        if handles.len() >= MAX_DEMO_ICONS {
                            break;
                        }
                    } else {
                        #[cfg(feature = "tracing")]
                        tracing::warn!("Atlas is full, cannot add icon '{}'", path.display());
                    }
                } else {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("Couldn't load icon '{}'", path.display());
                }
            }
        }

        if let Ok(entries) = std::fs::read_dir("assets/open-iconic/svg/") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("svg") {
                    continue;
                }
                #[cfg(feature = "tracing")]
                tracing::info!("Loaded svg icon '{}'", path.display(),);
                svg_paths.push(path);
                if svg_paths.len() >= MAX_DEMO_ICONS {
                    break;
                }
            }
        }

        state.icon_atlas = Some(atlas);
        state.icons = handles;
        state.svg_icons = svg_paths;
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

            #[cfg(feature = "tracing")]
            tracing::info!("Loaded image with dimensions: {}x{}", w, h);

            let handle = engine.load_texture_rgba8(w, h, rgba.as_raw());

            state.background = Some(handle);
        } else {
            #[cfg(feature = "tracing")]
            tracing::warn!("Couldn't load image!");
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
            target.view = target.view.next();
        } else {
            target.view = target.view.prev();
        }

        if let super::View::Texture = target.view {
            let scale = engine.globals(&tid).map(|g| g.scale).unwrap_or(1.0);
            ensure_background_loaded(engine, state);
            ensure_icons_loaded(engine, state, scale);
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

    pub fn set_slider(target: &mut super::Target, v: f32) -> bool {
        target.slider = v;
        true
    }
    pub fn submit_name(target: &mut super::Target, s: String) -> bool {
        target.name = s;
        true
    }
    pub fn submit_text_area(target: &mut super::Target, s: String) -> bool {
        target.text_area_content = s;
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
            let dt = engine.globals(&tid).unwrap().delta_time;
            target.fps[target.fps_idx] = 1.0 / dt;
            target.fps_idx = (target.fps_idx + 1) % 5;
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
        crate::Event::Message(Message::SliderChanged(v)) => update::set_slider(target, *v),
        crate::Event::Message(Message::NameChanged(s)) => update::submit_name(target, s.clone()),
        crate::Event::Message(Message::TextAreaContentChanged(s)) => {
            update::submit_text_area(target, s.clone())
        }
        crate::Event::Message(Message::ThemeSetDark) => {
            state.theme = Theme::dark();
            engine.set_theme(state.theme);
            true
        }
        crate::Event::Message(Message::ThemeSetLight) => {
            state.theme = Theme::light();
            engine.set_theme(state.theme);
            true
        }
        crate::Event::Message(Message::ThemeCornerRadius(r)) => {
            state.theme.corner_radius = *r;
            engine.set_theme(state.theme);
            true
        }
        crate::Event::Message(Message::ThemeBorderWidth(w)) => {
            state.theme.border_width = *w as i32;
            engine.set_theme(state.theme);
            true
        }
        _ => false,
    }
}

pub fn view(tid: &TargetId, state: &State) -> Element<Message> {
    let target = match state.per_target.get(tid) {
        Some(t) => t,
        None => return Rectangle::placeholder().into(),
    };
    match target.view {
        View::Layout => demos::layout::view(state),
        View::Interaction => demos::interaction::view(tid, state),
        View::Pipeline => demos::pipeline::view(tid, state),
        View::Texture => demos::texture::view(state),
        View::Text => demos::text::view(state),
        View::Scrollable => demos::scrollable::view(tid, state),
        View::ThemeEditor => demos::theme_editor::view(state),
    }
}
