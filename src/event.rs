use crate::model::{Position, Size};

pub trait ToEvent<M, E: ToEvent<M, E>> {
    fn to_event(&self) -> Event<M, E>;
}

#[derive(Debug)]
pub enum Event<M, E: ToEvent<M, E>> {
    RedrawRequested,
    Resized { size: Size<u32> },
    CursorMoved { position: Position<f32> },
    MouseInput { mouse_down: bool },
    KeyboardInput { char: u8 },
    Platform(E),
    Message(M),
}
