use super::*;
use crate::{render::pipeline::PipelineKey, widget::helpers::clamp_size};

pub struct Rectangle {
    layout: Option<Layout>,

    id: Id,
    position: Position<i32>,
    size: Size<Length<i32>>,
    color: Color<f32>,

    min: Size<i32>,
    max: Size<i32>,
}

impl Rectangle {
    pub fn new(size: Size<Length<i32>>, color: Color<f32>) -> Self {
        Self {
            layout: None,

            id: crate::context::next_id(),
            position: Position::splat(0),
            size,
            color,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
        }
    }

    pub fn min(mut self, size: Size<i32>) -> Self {
        self.min = size;
        self
    }
    pub fn max(mut self, size: Size<i32>) -> Self {
        self.max = size;
        self
    }
}

impl<M> Widget<M> for Rectangle {
    fn id(&self) -> Id {
        self.id
    }

    fn layout(&self) -> Layout {
        self.layout.expect(LAYOUT_ERROR)
    }

    fn fit_size(&mut self) -> Layout {
        self.layout = Some(Layout {
            size: self.size,
            current_size: clamp_size(self.size.into_fixed(), self.min, self.max),
            min: self.min,
            max: self.max,
        });

        self.layout.unwrap()
    }

    fn grow_size(&mut self, max: Size<i32>) {
        let width = match self.size.width {
            Length::Grow => max.width,
            Length::Fixed(x) => x,
            _ => 0,
        };
        let height = match self.size.height {
            Length::Grow => max.height,
            Length::Fixed(x) => x,
            _ => 0,
        };
        let clamped = clamp_size(Size::new(width, height), self.min, self.max);

        self.size.width = Length::Fixed(clamped.width);
        self.size.height = Length::Fixed(clamped.height);
        if let Some(layout) = self.layout.as_mut() {
            layout.current_size = clamped;
        }
    }

    fn place(&mut self, position: Position<i32>) -> Size<i32> {
        self.position = position;
        self.size.into_fixed()
    }

    fn draw(&self, instances: &mut Vec<Instance>) {
        instances.push(Instance::new(
            PipelineKey::Ui,
            self.position,
            self.size.into_fixed(),
            [self.color.r, self.color.g, self.color.b, self.color.a],
            [0, 0, 0, 0],
        ));
    }
}
