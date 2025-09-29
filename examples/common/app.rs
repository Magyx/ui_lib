use ui::widget::Element;

use super::demos;

#[derive(Clone)]
pub enum View {
    Layout = 0,
    Interaction = 1,
    Pipeline = 2,
    Texture = 3,
}

impl View {
    const COUNT: u8 = 4;

    fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Layout,
            1 => Self::Interaction,
            2 => Self::Pipeline,
            3 => Self::Texture,
            _ => unreachable!("value out of range"),
        }
    }

    pub fn next(self) -> Self {
        Self::from_u8((self as u8 + 1) % Self::COUNT)
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    ButtonPressed,
}

pub struct State {
    pub counter: u32,
    pub view: View,

    pub background: Option<ui::render::texture::TextureHandle>,
    pub icon_atlas: Option<ui::render::texture::Atlas>,
    pub icons: Vec<ui::render::texture::TextureHandle>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            counter: 0,
            view: View::Pipeline,
            background: None,
            icon_atlas: None,
            icons: Vec::new(),
        }
    }
}

pub mod update {
    use ui::graphics::Engine;

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
        engine: &mut Engine<'a, super::Message>,
        state: &mut super::State,
    ) -> bool {
        state.view = state.view.clone().next();
        if let super::View::Texture = state.view {
            ensure_background_loaded(engine, state);
            ensure_icons_loaded(engine, state);
        }

        true
    }

    pub fn increment_counter(state: &mut super::State) -> bool {
        state.counter += 1;
        true
    }
}

pub fn view(state: &State) -> Element<Message> {
    match state.view {
        View::Layout => demos::layout::view(state),
        View::Interaction => demos::interaction::view(state),
        View::Pipeline => demos::pipeline::view(state),
        View::Texture => demos::texture::view(state),
    }
}
