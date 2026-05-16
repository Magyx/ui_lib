use smol_str::SmolStr;

use crate::model::{Position, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

impl MouseButton {
    #[inline]
    pub fn bit(self) -> u32 {
        match self {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
            MouseButton::Back => 3,
            MouseButton::Forward => 4,
            MouseButton::Other(n) => 5 + (n as u32).min(26),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollUnits {
    Lines,
    Pixels,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollDelta {
    pub dx: f32,
    pub dy: f32,
    pub units: ScrollUnits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub super_: bool,
    pub caps_lock: Option<bool>,
    pub num_lock: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyLocation {
    Standard,
    Left,
    Right,
    Numpad,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LogicalKey {
    Character(SmolStr),
    Enter,
    Escape,
    Backspace,
    Tab,
    Space,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    F(u8),
    Dead,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PhysicalKey {
    Code(u32), // platform keycode/scancode/USB code if available
    Unidentified,
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub state: KeyState,           // pressed or released
    pub repeat: bool,              // true for auto-repeat events
    pub logical_key: LogicalKey,   // what the OS thinks the key “means”
    pub physical_key: PhysicalKey, // where on the keyboard (scan code)
    pub location: KeyLocation,     // left/right/numpad if known
    pub modifiers: Modifiers,      // snapshot at the event time
}

#[derive(Debug, Clone)]
pub struct TextInput {
    pub text: String, // full UTF-8
}

pub trait ToEvent<M, E: ToEvent<M, E>> {
    fn to_event(&self) -> Event<M, E>;
}

#[derive(Debug)]
pub enum Event<M, E: ToEvent<M, E>> {
    RedrawRequested,
    Resized {
        size: Size<u32>,
    },
    ScaleFactorChanged {
        factor: f64,
    },
    CursorMoved {
        position: Position<f32>,
    },
    MouseInput {
        button: MouseButton,
        state: KeyState,
    },
    MouseWheel(ScrollDelta),

    Key(KeyEvent),               // key press/release (with metadata)
    Text(TextInput),             // committed text (IME/composition)
    ModifiersChanged(Modifiers), // track a snapshot in your ctx

    Platform(E),
    Message(M),
}

#[derive(Debug, Clone, Copy)]
pub enum UiEventRef<'a> {
    RedrawRequested,
    Resized {
        size: Size<u32>,
    },
    CursorMoved {
        position: Position<f32>,
    },
    MouseButton {
        button: MouseButton,
        state: KeyState,
    },
    MouseWheel(ScrollDelta),
    Key(&'a KeyEvent),
    Text(&'a TextInput),
    ModifiersChanged(&'a Modifiers),
}

#[cfg(test)]
mod tests {
    use super::*;

    // MouseButton bit packing
    //
    // Context uses `1 << MouseButton::bit()` to index into the mouse_buttons_*
    // bitfields, which are u32. That's the invariant we're protecting here.

    #[test]
    fn mouse_button_bits_distinct_for_named() {
        let bits = [
            MouseButton::Left.bit(),
            MouseButton::Right.bit(),
            MouseButton::Middle.bit(),
            MouseButton::Back.bit(),
            MouseButton::Forward.bit(),
        ];
        assert_eq!(bits, [0, 1, 2, 3, 4]);
    }

    #[test]
    fn mouse_button_bits_fit_in_u32() {
        // Any bit produced must be < 32 so `1u32 << bit` stays defined.
        let samples = [
            MouseButton::Left,
            MouseButton::Right,
            MouseButton::Middle,
            MouseButton::Back,
            MouseButton::Forward,
            MouseButton::Other(0),
            MouseButton::Other(5),
            MouseButton::Other(26),
            MouseButton::Other(9999), // saturated
        ];
        for b in samples {
            assert!(b.bit() < 32, "{:?} bit={} must be < 32", b, b.bit());
        }
    }

    #[test]
    fn mouse_button_other_is_saturated() {
        // Per the impl, Other(n) -> 5 + min(n, 26). 26 is the cap.
        assert_eq!(MouseButton::Other(0).bit(), 5);
        assert_eq!(MouseButton::Other(1).bit(), 6);
        assert_eq!(MouseButton::Other(26).bit(), 31);
        assert_eq!(MouseButton::Other(100).bit(), 31);
        assert_eq!(MouseButton::Other(u16::MAX).bit(), 31);
    }

    #[test]
    fn mouse_button_equality_and_hash() {
        use std::collections::HashSet;
        let mut s: HashSet<MouseButton> = HashSet::new();
        s.insert(MouseButton::Left);
        s.insert(MouseButton::Left);
        s.insert(MouseButton::Other(3));
        s.insert(MouseButton::Other(3));
        assert_eq!(s.len(), 2);
    }

    // Modifiers

    #[test]
    fn modifiers_default_is_all_off() {
        let m = Modifiers::default();
        assert!(!m.shift);
        assert!(!m.control);
        assert!(!m.alt);
        assert!(!m.super_);
        assert_eq!(m.caps_lock, None);
        assert_eq!(m.num_lock, None);
    }

    // ScrollDelta

    #[test]
    fn scroll_delta_is_copy() {
        let d = ScrollDelta {
            dx: 1.0,
            dy: -2.0,
            units: ScrollUnits::Pixels,
        };
        let d2 = d; // Copy
        assert_eq!(d.dx, 1.0);
        assert_eq!(d2.dx, 1.0);
    }

    // LogicalKey

    #[test]
    fn logical_key_f_variant_carries_number() {
        let k = LogicalKey::F(5);
        if let LogicalKey::F(n) = k {
            assert_eq!(n, 5);
        } else {
            panic!("expected F variant");
        }
    }

    #[test]
    fn logical_key_equality_on_character() {
        use smol_str::SmolStr;
        let a = LogicalKey::Character(SmolStr::new("a"));
        let a2 = LogicalKey::Character(SmolStr::new("a"));
        let b = LogicalKey::Character(SmolStr::new("b"));
        assert_eq!(a, a2);
        assert_ne!(a, b);
    }

    // KeyState

    #[test]
    fn key_state_pressed_and_released_differ() {
        assert_ne!(KeyState::Pressed, KeyState::Released);
    }
}
