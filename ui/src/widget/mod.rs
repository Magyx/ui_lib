use std::ops::{Deref, DerefMut};

use crate::{
    context::{Context, Id},
    model::*,
    primitive::Primitive,
};

#[derive(Debug, Copy, Clone)]
pub struct Layout {
    pub size: Size<Length<i32>>,
    pub current_size: Size<i32>,
}

#[derive(Debug, Copy, Clone)]
pub enum Length<U> {
    Fit,
    Fixed(U),
    Grow,
}

impl<U> Size<Length<U>> {
    pub(crate) fn from_fixed(self) -> Size<U>
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
    fn draw(&self, primitives: &mut Vec<Primitive>);

    /* ----- interaction ----- */
    fn id(&self) -> Id;
    fn handle(&mut self, _ctx: &mut Context<M>) {}

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

    pub fn as_ref(&self) -> &dyn Widget<M> {
        self.0.as_ref()
    }

    pub fn as_mut(&mut self) -> &mut dyn Widget<M> {
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
