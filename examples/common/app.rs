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
}

impl Default for State {
    fn default() -> Self {
        Self {
            counter: 0,
            view: View::Pipeline,
            background: None,
        }
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
