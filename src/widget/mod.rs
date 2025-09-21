use std::ops::{Deref, DerefMut};

use crate::{
    context::{Context, Id},
    graphics::Globals,
    model::*,
    primitive::Instance,
};

mod helpers;

pub const LAYOUT_ERROR: &str = "Layout not set during fit_size!";

#[derive(Debug, Copy, Clone)]
pub struct Layout {
    pub size: Size<Length<i32>>,
    pub current_size: Size<i32>,
    pub min: Size<i32>,
    pub max: Size<i32>,
}

impl Layout {
    pub fn unconstrained(size: Size<Length<i32>>, current: Size<i32>) -> Self {
        Self {
            size,
            current_size: current,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Length<U> {
    Fit,
    Fixed(U),
    Grow,
}

impl<U> Size<Length<U>> {
    pub fn into_fixed(self) -> Size<U>
    where
        U: Default,
    {
        Size {
            width: match self.width {
                Length::Fixed(x) => x,
                _ => U::default(),
            },
            height: match self.height {
                Length::Fixed(x) => x,
                _ => U::default(),
            },
        }
    }
}

pub trait Widget<M> {
    fn layout(&self) -> Layout;

    /* ----- layout ----- */
    fn fit_size(&mut self) -> Layout;
    fn grow_size(&mut self, max: Size<i32>);
    fn place(&mut self, position: Position<i32>) -> Size<i32>;

    /* ----- paint ----- */
    fn draw(&self, instances: &mut Vec<Instance>);

    /* ----- interaction ----- */
    fn id(&self) -> Id;
    fn handle(&mut self, _globals: &Globals, _ctx: &mut Context<M>) {}

    fn einto(self) -> Element<M>
    where
        Self: Sized + 'static,
    {
        Element::new(self)
    }
}

pub struct Element<M>(Box<dyn Widget<M>>);

impl<M> Element<M> {
    pub fn new<W>(widget: W) -> Self
    where
        W: Widget<M> + 'static,
    {
        Element(Box::new(widget))
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

impl<M> Deref for Element<M> {
    type Target = dyn Widget<M>;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<M> DerefMut for Element<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

mod rectangle;
pub use rectangle::Rectangle;

mod row;
pub use row::Row;

mod column;
pub use column::Column;

mod container;
pub use container::Container;

mod button;
pub use button::Button;

mod simple_canvas;
pub use simple_canvas::SimpleCanvas;
