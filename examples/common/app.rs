use ui::widget::Element;

use super::demos;

#[derive(Clone)]
pub enum View {
    Layout = 0,
    Interaction = 1,
    Pipeline = 2,
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
}

impl Default for State {
    fn default() -> Self {
        Self {
            counter: 0,
            view: View::Layout,
        }
    }
}

pub fn view(state: &State) -> Element<Message> {
    match state.view {
        View::Layout => demos::layout::view(state),
        View::Interaction => demos::interaction::view(state),
        View::Pipeline => demos::pipeline::view(state),
    }
}
