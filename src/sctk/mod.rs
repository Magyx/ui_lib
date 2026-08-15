use std::sync::atomic::AtomicBool;

pub use error::SctkError;
pub use smithay_client_toolkit;
pub use smithay_client_toolkit::shell::{
    wlr_layer::{Anchor, KeyboardInteractivity, Layer},
    xdg::window::WindowDecorations,
};

pub use app::*;
pub use convert::SctkEvent;
pub use options::*;

pub mod app;
pub mod convert;
pub mod erased;
mod error;
pub mod handler;
mod helpers;
pub mod options;
pub mod raw;
pub mod runner;
pub mod state;

/// App-facing SCTK entry points, re-exported from
/// [`ui::prelude`](crate::prelude) when the `sctk` feature is on.
pub mod prelude {
    pub use super::{
        Anchor, KeyboardInteractivity, Layer, LayerOptions, LockOptions, Options, OutputSelector,
        OutputSet, SctkApp, SctkEvent, SctkLoop, SurfaceId, WindowDecorations, XdgOptions,
    };
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SurfaceId(u32);

#[derive(Default)]
pub struct SctkLoop {
    exit: AtomicBool,
}
impl SctkLoop {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn exit(&self) {
        self.exit.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn should_exit(&self) -> bool {
        self.exit.load(std::sync::atomic::Ordering::Relaxed)
    }
}
