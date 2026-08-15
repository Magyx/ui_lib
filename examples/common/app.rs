use std::{collections::HashMap, path::PathBuf};

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

#[derive(Clone)]
pub struct DecodedPng {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl std::fmt::Debug for DecodedPng {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodedPng")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes", &self.rgba.len())
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct DecodedIcons {
    pub atlas_phys: u32,
    pub pngs: Vec<DecodedPng>,
    pub svg_paths: Vec<PathBuf>,
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
    BackgroundLoaded(ui::render::texture::TextureHandle),
    IconsDecoded(DecodedIcons),
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
    pub bg_loading: bool,
    pub icon_atlas: Option<ui::render::texture::Atlas>,
    pub icons: Vec<ui::render::texture::TextureHandle>,
    pub icons_loading: bool,
    pub svg_icons: Vec<PathBuf>,
}

mod update {
    use ui::{
        prelude::*,
        render::AllocatorKind,
        task::{RawImage, Task},
    };

    use crate::common::Message;

    fn decode_icons(scale: f32) -> super::DecodedIcons {
        const MAX_DEMO_ICONS: usize = 16;

        let icon_phys = (48.0 * scale).round() as u32;
        let atlas_phys = (512.0 * scale).round() as u32;
        let mut pngs = Vec::new();
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
                    let (width, height) = rgba.dimensions();
                    pngs.push(super::DecodedPng {
                        width,
                        height,
                        rgba: rgba.into_raw(),
                    });
                    if pngs.len() >= MAX_DEMO_ICONS {
                        break;
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
                svg_paths.push(path);
                if svg_paths.len() >= MAX_DEMO_ICONS {
                    break;
                }
            }
        }

        super::DecodedIcons {
            atlas_phys,
            pngs,
            svg_paths,
        }
    }

    fn load_icons_task(scale: f32) -> Task<Message> {
        Task::blocking(move || decode_icons(scale), Message::IconsDecoded)
    }

    pub fn apply_icons<'a>(
        engine: &mut Engine<'a>,
        state: &mut super::State,
        decoded: &super::DecodedIcons,
    ) -> Task<Message> {
        let mut atlas =
            engine.create_atlas(decoded.atlas_phys, decoded.atlas_phys, AllocatorKind::Shelf);
        let mut handles = Vec::new();

        for png in &decoded.pngs {
            if let Some(handle) =
                engine.load_texture_into_atlas(&mut atlas, png.width, png.height, &png.rgba)
            {
                handles.push(handle);
            } else {
                #[cfg(feature = "tracing")]
                tracing::warn!("Atlas is full, cannot add icon");
            }
        }

        state.icon_atlas = Some(atlas);
        state.icons = handles;
        state.svg_icons = decoded.svg_paths.clone();
        state.icons_loading = false;
        Task::Redraw
    }

    fn load_background_task() -> Task<Message> {
        Task::load_image(
            || {
                let decoded = image::ImageReader::open("assets/background.jpg")
                    .ok()
                    .and_then(|reader| reader.decode().ok());

                match decoded {
                    Some(img) => {
                        let rgba = img.to_rgba8();
                        let (width, height) = rgba.dimensions();
                        #[cfg(feature = "tracing")]
                        tracing::info!("Loaded background with dimensions: {}x{}", width, height);
                        RawImage {
                            width,
                            height,
                            rgba: rgba.into_raw(),
                        }
                    }
                    None => {
                        #[cfg(feature = "tracing")]
                        tracing::warn!("Couldn't load background image!");
                        // 1x1 transparent fallback so the upload still succeeds.
                        RawImage {
                            width: 1,
                            height: 1,
                            rgba: vec![0, 0, 0, 0],
                        }
                    }
                }
            },
            Message::BackgroundLoaded,
        )
    }

    pub fn cycle_view<'a>(
        tid: TargetId,
        engine: &mut Engine<'a>,
        state: &mut super::State,
        dir: bool,
    ) -> Task<Message> {
        let view = {
            let Some(target) = state.per_target.get_mut(&tid) else {
                return Task::None;
            };
            target.view = if dir {
                target.view.next()
            } else {
                target.view.prev()
            };
            target.view
        };

        if let super::View::Texture = view {
            let scale = engine.globals(&tid).map(|g| g.scale).unwrap_or(1.0);

            let mut tasks = vec![Task::Redraw];
            if state.background.is_none() && !state.bg_loading {
                state.bg_loading = true;
                tasks.push(load_background_task());
            }
            if state.icon_atlas.is_none() && !state.icons_loading {
                state.icons_loading = true;
                tasks.push(load_icons_task(scale));
            }
            return Task::batch(tasks);
        }

        Task::Redraw
    }

    pub fn increment_counter(target: &mut super::Target) -> Task<Message> {
        target.counter += 1;
        Task::Redraw
    }

    pub fn toggle_debug<'a>(engine: &mut Engine<'a>) -> Task<Message> {
        engine.toggle_debug();
        Task::Redraw
    }

    pub fn set_slider(target: &mut super::Target, v: f32) -> Task<Message> {
        target.slider = v;
        Task::Redraw
    }
    pub fn submit_name(target: &mut super::Target, s: String) -> Task<Message> {
        target.name = s;
        Task::Redraw
    }
    pub fn submit_text_area(target: &mut super::Target, s: String) -> Task<Message> {
        target.text_area_content = s;
        Task::Redraw
    }
}

pub fn update<'a, E: ui::event::ToEvent<Message, E>>(
    tid: TargetId,
    engine: &mut Engine<'a>,
    event: &crate::Event<Message, E>,
    state: &mut State,
) -> Task<Message> {
    let target = state.per_target.entry(tid).or_default();
    match event {
        crate::Event::RedrawRequested => {
            let dt = engine.globals(&tid).unwrap().delta_time;
            target.fps[target.fps_idx] = 1.0 / dt;
            target.fps_idx = (target.fps_idx + 1) % 5;
            Task::None
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
                _ => Task::None,
            },
            _ => Task::None,
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
            Task::Redraw
        }
        crate::Event::Message(Message::ThemeSetLight) => {
            state.theme = Theme::light();
            engine.set_theme(state.theme);
            Task::Redraw
        }
        crate::Event::Message(Message::ThemeCornerRadius(r)) => {
            state.theme.corner_radius = *r;
            engine.set_theme(state.theme);
            Task::Redraw
        }
        crate::Event::Message(Message::ThemeBorderWidth(w)) => {
            state.theme.border_width = *w as i32;
            engine.set_theme(state.theme);
            Task::Redraw
        }
        crate::Event::Message(Message::BackgroundLoaded(handle)) => {
            state.background = Some(*handle);
            state.bg_loading = false;
            Task::Redraw
        }
        crate::Event::Message(Message::IconsDecoded(decoded)) => {
            update::apply_icons(engine, state, decoded)
        }
        _ => Task::None,
    }
}

pub fn view(tid: &TargetId, state: &State) -> Element {
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
