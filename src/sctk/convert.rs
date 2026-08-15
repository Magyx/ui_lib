use smithay_client_toolkit::seat::keyboard::Keysym;
use smol_str::ToSmolStr;

use super::SurfaceId;
use crate::{
    event::{
        Event, KeyEvent, KeyLocation, KeyState, LogicalKey, Modifiers, MouseButton, PhysicalKey,
        ScrollDelta, ScrollUnits, ToEvent,
    },
    model::{Position, Size},
};

/// Platform event type for the SCTK backend.
#[derive(Debug, Clone)]
pub enum SctkEvent {
    Redraw,
    Resized {
        surface: SurfaceId,
        size: Size<u32>,
    },
    ScaleChanged {
        surface: SurfaceId,
        factor: i32,
    },
    PointerMoved {
        surface: SurfaceId,
        pos: Position<f32>,
    },
    PointerButton {
        surface: SurfaceId,
        button: u32, // linux input BTN_* code
        pressed: bool,
    },
    PointerAxis {
        surface: SurfaceId,
        h: f64,
        v: f64,
    },

    Key {
        surface: SurfaceId,
        raw_code: u32,
        keysym: smithay_client_toolkit::seat::keyboard::Keysym,
        utf8: Option<String>,
        pressed: bool,
        repeat: bool,
    },

    Modifiers(SurfaceId, smithay_client_toolkit::seat::keyboard::Modifiers),
    Closed,
}

impl SctkEvent {
    pub fn surface_id(&self) -> Option<SurfaceId> {
        match self {
            SctkEvent::Resized { surface, .. }
            | SctkEvent::ScaleChanged { surface, .. }
            | SctkEvent::PointerMoved { surface, .. }
            | SctkEvent::PointerButton { surface, .. }
            | SctkEvent::PointerAxis { surface, .. }
            | SctkEvent::Key { surface, .. }
            | SctkEvent::Modifiers(surface, ..) => Some(*surface),
            _ => None,
        }
    }
}

impl<M> ToEvent<M, SctkEvent> for SctkEvent {
    fn to_event(&self) -> Event<M, SctkEvent> {
        match self {
            SctkEvent::Redraw => Event::RedrawRequested,
            SctkEvent::Resized { size, .. } => Event::Resized { size: *size },
            SctkEvent::ScaleChanged { factor, .. } => Event::ScaleFactorChanged {
                factor: *factor as f64,
            },
            SctkEvent::PointerMoved { pos, .. } => Event::CursorMoved { position: *pos },
            SctkEvent::PointerButton {
                button, pressed, ..
            } => {
                // Map common BTN_* codes; unknown -> Other(code)
                let mb = match *button {
                    272 => MouseButton::Left,    // BTN_LEFT
                    273 => MouseButton::Right,   // BTN_RIGHT
                    274 => MouseButton::Middle,  // BTN_MIDDLE
                    275 => MouseButton::Back,    // BTN_SIDE
                    276 => MouseButton::Forward, // BTN_EXTRA
                    n => MouseButton::Other(n as u16),
                };
                let ks = if *pressed {
                    KeyState::Pressed
                } else {
                    KeyState::Released
                };
                Event::MouseInput {
                    button: mb,
                    state: ks,
                }
            }
            SctkEvent::PointerAxis { h, v, .. } => Event::MouseWheel(ScrollDelta {
                dx: *h as f32,
                dy: *v as f32,
                units: ScrollUnits::Pixels,
            }),

            SctkEvent::Key {
                raw_code,
                keysym,
                utf8,
                pressed,
                repeat,
                ..
            } => {
                let state = if *pressed {
                    KeyState::Pressed
                } else {
                    KeyState::Released
                };
                let logical_key = map_keysym_to_logical(*keysym, utf8.as_deref());
                let physical_key = PhysicalKey::Code(*raw_code);

                Event::Key(KeyEvent {
                    state,
                    repeat: *repeat,
                    logical_key,
                    physical_key,
                    location: KeyLocation::Standard,
                })
            }

            SctkEvent::Modifiers(_, m) => Event::ModifiersChanged(Modifiers {
                shift: m.shift,
                control: m.ctrl,
                alt: m.alt,
                super_: m.logo,
                caps_lock: Some(m.caps_lock),
                num_lock: Some(m.num_lock),
            }),

            SctkEvent::Closed => Event::Platform(SctkEvent::Closed),
        }
    }
}

pub fn map_keysym_to_logical(k: Keysym, utf8: Option<&str>) -> LogicalKey {
    use smithay_client_toolkit::seat::keyboard::Keysym as KS;
    match k {
        KS::Return | KS::KP_Enter | KS::ISO_Enter | KS::Linefeed => LogicalKey::Enter,
        KS::Escape => LogicalKey::Escape,
        KS::BackSpace => LogicalKey::Backspace,
        KS::Tab | KS::ISO_Left_Tab | KS::KP_Tab => LogicalKey::Tab,
        KS::space | KS::KP_Space => LogicalKey::Space,

        KS::Left | KS::KP_Left => LogicalKey::ArrowLeft,
        KS::Right | KS::KP_Right => LogicalKey::ArrowRight,
        KS::Up | KS::KP_Up => LogicalKey::ArrowUp,
        KS::Down | KS::KP_Down => LogicalKey::ArrowDown,
        KS::Home | KS::KP_Home => LogicalKey::Home,
        KS::End | KS::KP_End => LogicalKey::End,
        KS::Page_Up | KS::KP_Page_Up => LogicalKey::PageUp,
        KS::Page_Down | KS::KP_Page_Down => LogicalKey::PageDown,
        KS::Insert | KS::KP_Insert => LogicalKey::Insert,
        KS::Delete | KS::KP_Delete => LogicalKey::Delete,

        KS::Shift_L | KS::Shift_R => LogicalKey::Shift,
        KS::Control_L | KS::Control_R => LogicalKey::Control,
        KS::Alt_L | KS::Alt_R => LogicalKey::Alt,
        KS::ISO_Level3_Shift | KS::Mode_switch => LogicalKey::AltGraph,
        KS::Super_L | KS::Super_R | KS::Meta_L | KS::Meta_R => LogicalKey::Super,
        KS::Caps_Lock => LogicalKey::CapsLock,
        KS::Num_Lock => LogicalKey::NumLock,
        KS::Menu => LogicalKey::ContextMenu,

        other if other.is_function_key() => {
            let n = other.raw() - KS::F1.raw() + 1;
            LogicalKey::F(n as u8)
        }

        other if (KS::dead_grave.raw()..=0xfe93).contains(&other.raw()) => LogicalKey::Dead,

        _ => utf8
            .map(|s| LogicalKey::Character(s.to_smolstr()))
            .unwrap_or(LogicalKey::Unknown),
    }
}
