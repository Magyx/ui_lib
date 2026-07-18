use crate::{
    context::*,
    layout::Node,
    model::*,
    primitive::Instance,
    theme::{Env, Theme},
};

#[derive(Clone, Copy, Debug, Default)]
pub enum Length {
    #[default]
    Fit,
    Fixed(i32),
    Grow,
    Weighted(f32),
}

impl Length {
    pub(crate) fn weight(self) -> Option<f32> {
        match self {
            Length::Grow => Some(1.0),
            Length::Weighted(w) => Some(w),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    /// Cross-axis only: fill the container's cross size. Resolved in the
    /// assign pass (not `place`) so the stretched child's subtree reflows.
    Stretch,
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

pub trait Widget: IntoElement {
    fn layout<'a>(&mut self, ctx: &mut LayoutCtx<'a>) -> Node;
    fn key(&self) -> Option<u64> {
        None
    }
    fn child_count(&self) -> usize;
    fn child_mut(&mut self, idx: usize) -> &mut dyn Widget;
    fn child_env(&self, env: Env, theme: &Theme) -> Env {
        let _ = theme;
        env
    }

    fn focusable(&self) -> bool {
        false
    }
    fn focus_trap(&self) -> bool {
        false
    }

    fn min_height_for_width<'a>(&mut self, ctx: &mut LayoutCtx<'a>, width: i32) -> Option<i32> {
        let _ = (ctx, width);
        None
    }

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
    fn paint_focus_ring(&self, ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {
        ctx.focus_ring(instances, ctx.rect().xywh());
    }

    fn handle(&mut self, ctx: &mut EventCtx) {
        let _ = ctx;
    }
    fn handle_after(&mut self, ctx: &mut EventCtx) {
        let _ = ctx;
    }
}

pub struct Element {
    inner: Box<dyn Widget>,
}

impl Element {
    pub fn new<W>(widget: W) -> Self
    where
        W: Widget + 'static,
    {
        Self {
            inner: Box::new(widget),
        }
    }
}

impl<W> From<W> for Element
where
    W: Widget + IntoElement + 'static,
{
    fn from(w: W) -> Self {
        Self::new(w)
    }
}

impl AsRef<dyn Widget> for Element {
    fn as_ref(&self) -> &(dyn Widget + 'static) {
        self.inner.as_ref()
    }
}

impl AsMut<dyn Widget + 'static> for Element {
    fn as_mut(&mut self) -> &mut (dyn Widget + 'static) {
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
