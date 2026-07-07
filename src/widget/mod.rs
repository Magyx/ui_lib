use crate::{context::*, layout::Node, model::*, primitive::Instance};

#[derive(Clone, Copy, Debug, Default)]
pub enum Length {
    #[default]
    Fit,
    Fixed(i32),
    Grow,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Axis {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Padding {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

pub trait IntoElement {}

pub trait Widget<M>: IntoElement {
    /* ----- layout ----- */
    fn layout<'a>(&mut self, ctx: &mut LayoutCtx<'a, M>) -> Node;
    fn key(&self) -> Option<u64> {
        None
    }
    fn child_count(&self) -> usize;
    fn child_mut(&mut self, idx: usize) -> &mut dyn Widget<M>;

    fn min_height_for_width<'a>(&mut self, ctx: &mut LayoutCtx<'a, M>, width: i32) -> Option<i32> {
        let _ = (ctx, width);
        None
    }

    /* ----- paint ----- */
    fn children_offset(&self, view_state: &mut ViewState, id: Id) -> (i32, i32) {
        let _ = (view_state, id);
        (0, 0)
    }
    fn prepare(&mut self, ctx: &mut PrepareCtx) {
        let _ = ctx;
    }
    fn prepare_overlay(&mut self, ctx: &mut PrepareCtx) {
        let _ = ctx;
    }
    fn paint(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>);
    fn paint_overlay(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        let _ = (ctx, instances);
    }

    /* ----- interaction ----- */
    fn handle(&mut self, ctx: &mut EventCtx<M>) {
        let _ = ctx;
    }
    fn handle_after(&mut self, ctx: &mut EventCtx<M>) {
        let _ = ctx;
    }
}

pub struct Element<M> {
    inner: Box<dyn Widget<M> + 'static>,
}

impl<M> Element<M> {
    pub fn new<W>(widget: W) -> Self
    where
        W: Widget<M> + 'static,
    {
        Self {
            inner: Box::new(widget),
        }
    }
}

impl<M, W> From<W> for Element<M>
where
    W: Widget<M> + IntoElement + 'static,
{
    fn from(w: W) -> Self {
        Self::new(w)
    }
}

impl<M> AsRef<dyn Widget<M> + 'static> for Element<M> {
    fn as_ref(&self) -> &(dyn Widget<M> + 'static) {
        self.inner.as_ref()
    }
}

impl<M> AsMut<dyn Widget<M> + 'static> for Element<M> {
    fn as_mut(&mut self) -> &mut (dyn Widget<M> + 'static) {
        self.inner.as_mut()
    }
}

#[macro_export]
macro_rules! el {
    ( $( $x:expr ),* $(,)? ) => {
        vec![ $( Element::from($x) ),* ]
    };
}

mod rectangle;
pub use rectangle::Rectangle;

mod spacer;
pub use spacer::Spacer;

mod row;
pub use row::Row;

mod column;
pub use column::Column;

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
pub use scroll::Scrollable;

#[cfg(feature = "svg")]
mod svg;
#[cfg(feature = "svg")]
pub use svg::Svg;

mod keyed;
pub use keyed::Keyed;
