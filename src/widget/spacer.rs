use super::*;

pub struct Spacer {
    layout: Option<Layout>,

    id: Id,
    position: Position<i32>,
    size: Size<Length<i32>>,
}

impl Spacer {
    pub fn new(size: Size<Length<i32>>) -> Self {
        Self {
            layout: None,

            id: crate::context::next_id(),
            position: Position::splat(0),
            size,
        }
    }
}

impl<M> Widget<M> for Spacer {
    fn id(&self) -> Id {
        self.id
    }

    fn layout(&self) -> Layout {
        self.layout.expect(LAYOUT_ERROR)
    }

    fn fit_size(&mut self, _ctx: &mut FitCtx<M>) -> Layout {
        self.layout = Some(Layout {
            size: self.size,
            current_size: self.size.into_fixed(),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
        });

        self.layout.unwrap()
    }

    fn grow_size(&mut self, _ctx: &mut GrowCtx<M>, max: Size<i32>) {
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

        self.size.width = Length::Fixed(width);
        self.size.height = Length::Fixed(height);
        if let Some(layout) = self.layout.as_mut() {
            layout.current_size = self.size.into_fixed();
        }
    }

    fn place(&mut self, _ctx: &mut PlaceCtx<M>, position: Position<i32>) -> Size<i32> {
        self.position = position;
        self.size.into_fixed()
    }

    fn draw(&self, _ctx: &mut PaintCtx, instances: &mut Vec<Instance>) {}
}
