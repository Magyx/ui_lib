use winit::{
    dpi::PhysicalSize,
    event::{ElementState, WindowEvent},
};

use crate::{
    Size,
    event::{Event, ToEvent},
    model::Position,
};

impl<P> From<PhysicalSize<P>> for Size<P> {
    fn from(s: PhysicalSize<P>) -> Self {
        Size::new(s.width, s.height)
    }
}

impl<M> ToEvent<M, winit::event::WindowEvent> for winit::event::WindowEvent {
    fn to_event(&self) -> Event<M, Self> {
        match self {
            WindowEvent::RedrawRequested => Event::RedrawRequested,
            WindowEvent::Resized(size) => Event::Resized {
                size: (*size).into(),
            },
            WindowEvent::CursorMoved { position, .. } => Event::CursorMoved {
                position: Position::new(position.x as f32, position.y as f32),
            },
            WindowEvent::MouseInput { state, .. } => Event::MouseInput {
                mouse_down: *state == ElementState::Pressed,
            },
            _ => Event::Platform(self.clone()),
        }
    }
}
