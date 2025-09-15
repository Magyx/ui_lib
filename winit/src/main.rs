use ui::{event::Event, graphics::Engine, widget::Element};
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowAttributes,
};

mod demos;

#[derive(Clone)]
enum View {
    Layout = 0,
    Interaction = 1,
}

impl View {
    const COUNT: u8 = 2;

    fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Layout,
            1 => Self::Interaction,
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
    event: &Event<Message, WindowEvent>,
    state: &mut State,
    event_loop: &ActiveEventLoop,
) -> bool {
    if let Event::Platform(window_event) = event {
        match window_event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state: key_state,
                        ..
                    },
                ..
            } => match (key, key_state) {
                (KeyCode::Escape, _) => {
                    event_loop.exit();
                }
                (KeyCode::KeyN, ElementState::Pressed) => {
                    state.view = state.view.clone().next();
                    return true;
                }
                _ => (),
            },
            _ => (),
        }
    } else if let Event::Message(msg) = event {
        match msg {
            Message::ButtonPressed => state.counter += 1,
        }
    }

    false
}

fn view(state: &State) -> Element<Message> {
    match state.view {
        View::Layout => demos::layout::view(state),
        View::Interaction => demos::interaction::view(state),
    }
}

fn main() {
    env_logger::init();
    let attrs = WindowAttributes::default().with_title("My Test GUI lib");
    _ = ui::winit::run_app::<Message, _, _, _>(
        State {
            view: View::Layout,
            counter: 0,
        },
        view,
        update,
        attrs,
    );
}
