use super::*;
use crate::{render::texture::TextureHandle, widget::helpers::clamp_size};

pub struct Image {
    layout: Option<Layout>,
    id: Id,
    position: Position<i32>,
    size: Size<Length<i32>>,
    min: Size<i32>,
    max: Size<i32>,

    handle: TextureHandle,
    tint: Color<f32>,
}

impl Image {
    pub fn new(size: Size<Length<i32>>, handle: TextureHandle) -> Self {
        Self {
            layout: None,
            id: crate::context::next_id(),
            position: Position::splat(0),
            size,
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
            handle,
            tint: Color::WHITE,
        }
    }
    pub fn tint(mut self, tint: Color<f32>) -> Self {
        self.tint = tint;
        self
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

impl<M> Widget<M> for Image {
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
        let w = match self.size.width {
            Length::Grow => max.width,
            Length::Fixed(x) => x,
            _ => 0,
        };
        let h = match self.size.height {
            Length::Grow => max.height,
            Length::Fixed(x) => x,
            _ => 0,
        };
        let clamped = clamp_size(Size::new(w, h), self.min, self.max);
        self.size.width = Length::Fixed(clamped.width);
        self.size.height = Length::Fixed(clamped.height);
        if let Some(l) = self.layout.as_mut() {
            l.current_size = clamped;
        }
    }

    fn place(&mut self, position: Position<i32>) -> Size<i32> {
        self.position = position;
        self.size.into_fixed()
    }

    fn draw(&self, instances: &mut Vec<Instance>) {
        instances.push(Instance::ui_tex(
            self.position,
            self.size.into_fixed(),
            self.tint,
            self.handle,
        ));
    }
}
