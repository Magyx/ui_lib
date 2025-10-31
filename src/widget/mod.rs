// TODO: something is wrong with MouseButton usage in widgets

use std::fmt::Debug;

use crate::{context::*, layout::Node, model::*, primitive::Instance};

#[derive(Clone, Copy, Debug)]
pub enum Length {
    Fit,
    Fixed(i32),
    Grow,
}

impl Default for Length {
    fn default() -> Self {
        Self::Fit
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Default for Axis {
    fn default() -> Self {
        Self::Horizontal
    }
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
    fn set_layout(&mut self, x: i32, y: i32, w: i32, h: i32);
    fn child_count(&self) -> usize;
    fn child_mut(&mut self, idx: usize) -> &mut dyn Widget<M>;

    fn min_height_for_width<'a>(
        &mut self,
        _ctx: &mut LayoutCtx<'a, M>,
        _width: i32,
    ) -> Option<i32> {
        None
    }

    /* ----- paint ----- */
    fn paint_descendants(&self) -> bool {
        true
    }
    fn paint(&mut self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>);

    /* ----- interaction ----- */
    fn handle(&mut self, _ctx: &mut EventCtx<M>) {}
}

pub struct Element<M>(Box<dyn Widget<M> + 'static>);

impl<M> Element<M> {
    pub fn new<W>(widget: W) -> Self
    where
        W: Widget<M> + 'static,
    {
        Element(Box::new(widget))
    }
}

impl<M, W> From<W> for Element<M>
where
    W: Widget<M> + IntoElement + 'static,
{
    fn from(w: W) -> Self {
        Element::new(w)
    }
}

impl<M> AsRef<dyn Widget<M> + 'static> for Element<M> {
    fn as_ref(&self) -> &(dyn Widget<M> + 'static) {
        self.0.as_ref()
    }
}

impl<M> AsMut<dyn Widget<M> + 'static> for Element<M> {
    fn as_mut(&mut self) -> &mut (dyn Widget<M> + 'static) {
        self.0.as_mut()
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
pub use text::Text;

mod slider;
pub use slider::Slider;

mod grid;
pub use grid::Grid;

mod text_input;
pub use text_input::{TextArea, TextColors, TextField, TextState};

mod scroll;
pub use scroll::{ScrollState, Scrollable};
