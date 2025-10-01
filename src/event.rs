use smol_str::SmolStr;

use crate::model::{Position, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
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
    Resized { size: Size<u32> },
    CursorMoved { position: Position<f32> },
    MouseInput { mouse_down: bool },

    Key(KeyEvent),               // key press/release (with metadata)
    Text(TextInput),             // committed text (IME/composition)
    ModifiersChanged(Modifiers), // track a snapshot in your ctx

    Platform(E),
    Message(M),
}
