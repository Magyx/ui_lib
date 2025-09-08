use super::*;

pub struct Rectangle {
    layout: Option<Layout>,

    id: Id,
    position: Position<i32>,
    size: Size<Length<i32>>,
    color: Color<f32>,
}

impl Rectangle {
    pub fn new(size: Size<Length<i32>>, color: Color<f32>) -> Self {
        Self {
            layout: None,

            id: crate::context::next_id(),
            position: Position::splat(0),
            size,
            color,
        }
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
            current_size: self.size.into_fixed(),
        });

        self.layout.unwrap()
    }

    fn grow_size(&mut self, max: Size<i32>) {
        if let Length::Grow = self.size.width {
            self.size.width = Length::Fixed(max.width);
        }
        if let Length::Grow = self.size.height {
            self.size.height = Length::Fixed(max.height);
        }

        if let Some(layout) = self.layout.as_mut() {
            layout.current_size = self.size.into_fixed();
        }
    }

    fn place(&mut self, position: Position<i32>) -> Size<i32> {
        self.position = position;
        self.size.into_fixed()
    }

    fn draw(&self, primitives: &mut Vec<Primitive>) {
        primitives.push(Primitive::color(
            self.position,
            self.size.into_fixed(),
            self.color,
        ));
    }
}
