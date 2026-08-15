pub use ::winit;
pub use ::winit::{application, dpi, error, keyboard, monitor, platform, window};
pub use ::winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{CursorIcon, Fullscreen, Icon, Window, WindowAttributes, WindowButtons, WindowLevel},
};

pub use app::WinitApp;

mod app;
mod convert;

/// App-facing winit entry points, re-exported from
/// [`ui::prelude`](crate::prelude) when the `winit` feature is on.
pub mod prelude {
    pub use super::{
        ActiveEventLoop, ControlFlow, Window, WindowAttributes, WindowEvent, WinitApp,
    };
}
