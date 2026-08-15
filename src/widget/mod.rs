pub mod prelude {
    pub use super::{
        Widget,
        element::{Element, IntoElement},
    };
    pub use crate::{
        context::{
            Env, EventCtx, Id, LayoutCtx, OnSweep, PaintCtx, PrepareCtx, SweepCtx, ViewState,
        },
        el,
        layout::{Align, Axis, Length, Node, Padding},
        model::*,
        primitive::{Instance, InstanceStore},
        theme::Theme,
    };
}

pub use ui_macros::Widget;
pub mod element;
pub use element::{Element, IntoElement, Widget};

mod rectangle;
pub use rectangle::Rectangle;

mod spacer;
pub use spacer::Spacer;

mod row;
pub use row::Row;

mod column;
pub use column::{Center, Column};

mod overlay;
pub use overlay::Overlay;

mod button;
pub use button::Button;

mod simple_canvas;
pub use simple_canvas::SimpleCanvas;

mod image;
pub use image::{ContentFit, Image};

mod text;
pub use text::{Text, TextRole};

mod slider;
pub use slider::Slider;

mod wrapping_rows;
pub use wrapping_rows::WrappingRows;

mod text_input;
pub use text_input::{TextArea, TextField};

mod scroll;
#[doc(hidden)]
pub use scroll::ScrollViewState;
pub use scroll::{ScrollBarBehavior, ScrollTo, Scrollable};

#[cfg(feature = "svg")]
mod svg;
#[cfg(feature = "svg")]
pub use svg::Svg;

mod keyed;
pub use keyed::Keyed;

mod card;
pub use card::Card;

mod checkbox;
pub use checkbox::{CheckState, Checkbox, Mark};

mod switch;
pub use switch::Switch;

mod progress;
pub use progress::ProgressBar;

mod spinner;
pub use spinner::Spinner;

mod radio;
pub use radio::{Radio, RadioGroup};
