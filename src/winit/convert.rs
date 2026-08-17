use smol_str::ToSmolStr;
use winit::{
    dpi::PhysicalSize,
    event::{MouseButton as WMouseButton, MouseScrollDelta},
    keyboard::{Key as WKey, KeyLocation as WLoc, PhysicalKey as WPhys},
};

use crate::{
    event::{
        Event, KeyEvent, KeyLocation, KeyState, LogicalKey, Modifiers, MouseButton, PhysicalKey,
        ScrollDelta, ScrollUnits, TextInput, ToEvent,
    },
    model::{Position, Size},
};

impl<P> From<PhysicalSize<P>> for Size<P> {
    fn from(s: PhysicalSize<P>) -> Self {
        Size::new(s.width, s.height)
    }
}

fn map_winit_logical(k: &WKey) -> LogicalKey {
    use ::winit::keyboard::NamedKey;
    match k {
        WKey::Character(s) => LogicalKey::Character(s.to_smolstr()),
        WKey::Named(n) => match n {
            NamedKey::Enter => LogicalKey::Enter,
            NamedKey::Escape => LogicalKey::Escape,
            NamedKey::Backspace => LogicalKey::Backspace,
            NamedKey::Tab => LogicalKey::Tab,
            NamedKey::Space => LogicalKey::Space,
            NamedKey::ArrowLeft => LogicalKey::ArrowLeft,
            NamedKey::ArrowRight => LogicalKey::ArrowRight,
            NamedKey::ArrowUp => LogicalKey::ArrowUp,
            NamedKey::ArrowDown => LogicalKey::ArrowDown,
            NamedKey::Home => LogicalKey::Home,
            NamedKey::End => LogicalKey::End,
            NamedKey::PageUp => LogicalKey::PageUp,
            NamedKey::PageDown => LogicalKey::PageDown,
            NamedKey::Insert => LogicalKey::Insert,
            NamedKey::Delete => LogicalKey::Delete,
            NamedKey::Shift => LogicalKey::Shift,
            NamedKey::Control => LogicalKey::Control,
            NamedKey::Alt => LogicalKey::Alt,
            NamedKey::AltGraph => LogicalKey::AltGraph,
            NamedKey::Super | NamedKey::Meta => LogicalKey::Super,
            NamedKey::CapsLock => LogicalKey::CapsLock,
            NamedKey::NumLock => LogicalKey::NumLock,
            NamedKey::ContextMenu => LogicalKey::ContextMenu,
            NamedKey::F1 => LogicalKey::F(1),
            NamedKey::F2 => LogicalKey::F(2),
            NamedKey::F3 => LogicalKey::F(3),
            NamedKey::F4 => LogicalKey::F(4),
            NamedKey::F5 => LogicalKey::F(5),
            NamedKey::F6 => LogicalKey::F(6),
            NamedKey::F7 => LogicalKey::F(7),
            NamedKey::F8 => LogicalKey::F(8),
            NamedKey::F9 => LogicalKey::F(9),
            NamedKey::F10 => LogicalKey::F(10),
            NamedKey::F11 => LogicalKey::F(11),
            NamedKey::F12 => LogicalKey::F(12),
            NamedKey::F13 => LogicalKey::F(13),
            NamedKey::F14 => LogicalKey::F(14),
            NamedKey::F15 => LogicalKey::F(15),
            NamedKey::F16 => LogicalKey::F(16),
            NamedKey::F17 => LogicalKey::F(17),
            NamedKey::F18 => LogicalKey::F(18),
            NamedKey::F19 => LogicalKey::F(19),
            NamedKey::F20 => LogicalKey::F(20),
            NamedKey::F21 => LogicalKey::F(21),
            NamedKey::F22 => LogicalKey::F(22),
            NamedKey::F23 => LogicalKey::F(23),
            NamedKey::F24 => LogicalKey::F(24),
            NamedKey::F25 => LogicalKey::F(25),
            NamedKey::F26 => LogicalKey::F(26),
            NamedKey::F27 => LogicalKey::F(27),
            NamedKey::F28 => LogicalKey::F(28),
            NamedKey::F29 => LogicalKey::F(29),
            NamedKey::F30 => LogicalKey::F(30),
            NamedKey::F31 => LogicalKey::F(31),
            NamedKey::F32 => LogicalKey::F(32),
            NamedKey::F33 => LogicalKey::F(33),
            NamedKey::F34 => LogicalKey::F(34),
            NamedKey::F35 => LogicalKey::F(35),
            _ => LogicalKey::Unknown,
        },
        _ => LogicalKey::Unknown,
    }
}

fn map_winit_physical(p: &WPhys) -> PhysicalKey {
    match p {
        WPhys::Code(code) => PhysicalKey::Code(*code as u32),
        WPhys::Unidentified(_) => PhysicalKey::Unidentified,
    }
}

fn map_winit_location(l: WLoc) -> KeyLocation {
    match l {
        WLoc::Standard => KeyLocation::Standard,
        WLoc::Left => KeyLocation::Left,
        WLoc::Right => KeyLocation::Right,
        WLoc::Numpad => KeyLocation::Numpad,
    }
}

impl<M> ToEvent<M, ::winit::event::WindowEvent> for ::winit::event::WindowEvent {
    fn to_event(&self) -> Event<M, Self> {
        use ::winit::event::{ElementState, WindowEvent as WE};

        match self {
            WE::RedrawRequested => Event::RedrawRequested,
            WE::Resized(size) => Event::Resized {
                size: (*size).into(),
            },
            WE::ScaleFactorChanged { scale_factor, .. } => Event::ScaleFactorChanged {
                factor: *scale_factor,
            },
            WE::CursorEntered { .. } => Event::CursorEntered,
            WE::CursorLeft { .. } => Event::CursorLeft,
            WE::CursorMoved { position, .. } => Event::CursorMoved {
                position: Position::new(position.x as f32, position.y as f32),
            },
            WE::MouseInput { state, button, .. } => {
                let mb = match button {
                    WMouseButton::Left => MouseButton::Left,
                    WMouseButton::Right => MouseButton::Right,
                    WMouseButton::Middle => MouseButton::Middle,
                    WMouseButton::Back => MouseButton::Back,
                    WMouseButton::Forward => MouseButton::Forward,
                    WMouseButton::Other(n) => MouseButton::Other(*n),
                };
                let ks = match state {
                    ElementState::Pressed => KeyState::Pressed,
                    ElementState::Released => KeyState::Released,
                };
                Event::MouseInput {
                    button: mb,
                    state: ks,
                }
            }
            WE::MouseWheel { delta, .. } => {
                let (dx, dy, units) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (*x, -*y, ScrollUnits::Lines),
                    MouseScrollDelta::PixelDelta(p) => {
                        (p.x as f32, -(p.y as f32), ScrollUnits::Pixels)
                    }
                };
                Event::MouseWheel(ScrollDelta { dx, dy, units })
            }
            WE::KeyboardInput { event, .. } => {
                let state = match event.state {
                    ElementState::Pressed => KeyState::Pressed,
                    ElementState::Released => KeyState::Released,
                };

                let logical_key = map_winit_logical(&event.logical_key);
                let physical_key = map_winit_physical(&event.physical_key);
                let location = map_winit_location(event.location);

                Event::Key(KeyEvent {
                    state,
                    repeat: event.repeat,
                    logical_key,
                    physical_key,
                    location,
                })
            }
            WE::Ime(winit::event::Ime::Commit(s)) => Event::Text(TextInput { text: s.clone() }),
            WE::ModifiersChanged(m) => Event::ModifiersChanged(Modifiers {
                shift: m.state().shift_key(),
                control: m.state().control_key(),
                alt: m.state().alt_key(),
                super_: m.state().super_key(),
                caps_lock: None,
                num_lock: None,
            }),
            WE::Focused(f) => Event::Focused(*f),
            WE::CloseRequested => Event::CloseRequested,
            _ => Event::Platform(self.clone()),
        }
    }
}
