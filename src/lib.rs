extern crate self as ui;

pub use bytemuck;
pub use error::{Error, Result};
pub use wgpu;

pub(crate) mod defaults;

pub mod builder;
pub mod context;
pub mod error;
pub mod event;
pub mod focus;
pub mod graphics;
pub mod layout;
pub mod model;
pub mod primitive;
pub mod profile;
pub mod render;
#[cfg(feature = "sctk")]
pub mod sctk;
pub mod task;
pub mod text;
pub mod theme;
pub mod tree;
pub mod widget;
#[cfg(feature = "winit")]
pub mod winit;

/// Everything an application author needs to build a UI.
///
/// Bring it into scope with `use ui::prelude::*;`. This is the app-facing
/// surface: widget constructors, [`Element`](crate::widget::Element), the
/// [`el!`](crate::el) macro, the layout primitives (`Length`, `Axis`,
/// `Padding`), and the core model/theming types (`Color`, `Size`, `Theme`).
pub mod prelude {
    pub use crate::el;

    pub use crate::layout::{Align, Axis, Length, Padding};
    pub use crate::widget::{
        Button, Center, CheckState, Checkbox, Column, ContentFit, Element, Image, Keyed, Mark,
        Overlay, ProgressBar, RadioGroup, Rectangle, Row, Scrollable, SimpleCanvas, Slider, Spacer,
        Spinner, Switch, Text, TextArea, TextField, TextRole, WrappingRows,
    };

    pub use crate::graphics::{Engine, TargetId};
    pub use crate::task::{RawImage, Task, TaskRunner, UploadCtx};

    #[cfg(feature = "svg")]
    pub use crate::widget::Svg;

    pub use crate::model::*;
    pub use crate::text::{FontStyle, Weight, Wrap};
    pub use crate::theme::Theme;
}

/// Build a list of pipelines for
/// [`pipelines`](crate::winit::WinitApp::pipelines), naming only the types.
///
/// ```ignore
/// .pipelines(ui::pipelines![PlanetPipeline, StarfieldPipeline])
/// ```
///
/// Equivalent to chaining `.pipeline::<P>()` once per type; use whichever
/// reads better. Neither requires importing anything beyond the pipeline
/// types themselves.
#[macro_export]
macro_rules! pipelines {
    ( $( $ty:path ),+ $(,)? ) => {
        [
            $(
                $crate::render::pipeline::PipelineRegistration::of::<$ty>(),
            )+
        ]
    };
}
