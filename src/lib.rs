pub use bytemuck;
pub use error::{Error, Result};
pub use wgpu;

pub(crate) mod consts;

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

    pub use crate::widget::{
        Align, Axis, Button, Center, CheckState, Checkbox, Column, ContentFit, Element, Image,
        Keyed, Length, Mark, Overlay, Padding, ProgressBar, RadioGroup, Rectangle, Row, Scrollable,
        SimpleCanvas, Slider, Spacer, Spinner, Switch, Text, TextArea, TextField, TextRole,
        WrappingRows,
    };

    pub use crate::graphics::{Engine, TargetId};
    pub use crate::task::{RawImage, Task, TaskRunner, UploadCtx};

    #[cfg(feature = "svg")]
    pub use crate::widget::Svg;

    pub use crate::model::*;
    pub use crate::text::{FontStyle, Weight, Wrap};
    pub use crate::theme::Theme;
}

#[macro_export]
macro_rules! pipeline_factory {
    ( $ty:path ) => {{
        fn __factory(
            gpu: &$crate::graphics::Gpu,
            surface_format: &$crate::wgpu::TextureFormat,
            buffers: &[$crate::wgpu::VertexBufferLayout],
            texture_bgl: &$crate::wgpu::BindGroupLayout,
            ranges: &[$crate::wgpu::PushConstantRange],
        ) -> ::std::boxed::Box<dyn $crate::render::pipeline::Pipeline> {
            ::std::boxed::Box::new(<$ty as $crate::render::pipeline::Pipeline>::new(
                gpu,
                surface_format,
                buffers,
                texture_bgl,
                ranges,
            ))
        }
        __factory as $crate::render::PipelineFactoryFn
    }};
}

#[macro_export]
macro_rules! pipeline_factories {
    ( $( $name:literal => $ty:path ),+ $(,)? ) => {{
        [
            $(
                ($name, $crate::pipeline_factory!($ty)),
            )+
        ]
    }};
}
