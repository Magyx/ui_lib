use super::*;
use crate::widget::helpers::{Height, clamp_size, equalize_sizes};

pub struct Column<M> {
    layout: Option<Layout>,

    id: Id,
    children: Vec<Element<M>>,
    spacing: i32,
    position: Position<i32>,
    size: Size<Length<i32>>,
    color: Color<f32>,
    padding: Vec4<i32>,
    min: Size<i32>,
    max: Size<i32>,
}

impl<M> Column<M> {
    pub fn new(children: Vec<Element<M>>) -> Self {
        Self {
            layout: None,

            id: crate::context::next_id(),
            children,
            spacing: 0,
            position: Position::splat(0),
            size: Size::splat(Length::Fit),
            color: Color::TRANSPARENT,
            padding: Vec4::splat(0),
            min: Size::splat(0),
            max: Size::splat(i32::MAX),
        }
    }

    pub fn spacing(mut self, amount: i32) -> Self {
        self.spacing = amount;
        self
    }

    pub fn size(mut self, size: Size<Length<i32>>) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Color<f32>) -> Self {
        self.color = color;
        self
    }

    pub fn padding(mut self, amount: Vec4<i32>) -> Self {
        self.padding = amount;
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

impl<M: 'static> Widget<M> for Column<M> {
    fn id(&self) -> Id {
        self.id
    }

    fn layout(&self) -> Layout {
        self.layout.expect(LAYOUT_ERROR)
    }

    fn fit_size(&mut self) -> Layout {
        let width_padding = self.padding.x + self.padding.z;
        let height_padding = self.padding.y + self.padding.w;

        let mut min_width = 0;
        let mut min_height = (self.children.len() as i32 - 1) * self.spacing + height_padding;
        for child in self.children.iter_mut() {
            let Layout { current_size, .. } = child.fit_size();
            if min_width < current_size.width {
                min_width = current_size.width;
            }
            min_height += current_size.height;
        }
        min_width += width_padding;

        if matches!(self.size.width, Length::Fit) {
            self.size.width = Length::Fixed(min_width);
        }
        if matches!(self.size.height, Length::Fit) {
            self.size.height = Length::Fixed(min_height);
        }

        let intrinsic_min = Size::new(min_width, min_height);
        let min = Size::new(
            intrinsic_min.width.max(self.min.width),
            intrinsic_min.height.max(self.min.height),
        );
        self.layout = Some(Layout {
            size: self.size,
            current_size: helpers::clamp_size(self.size.into_fixed(), min, self.max),
            min,
            max: self.max,
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

        let (min, max) = {
            let layout = self.layout.expect(LAYOUT_ERROR);
            (layout.min, layout.max)
        };
        let outer_size = clamp_size(self.size.into_fixed(), min, max);
        self.size.width = Length::Fixed(outer_size.width);
        self.size.height = Length::Fixed(outer_size.height);

        let inner_width = outer_size.width - self.padding.x - self.padding.z;
        let inner_height = outer_size.height
            - (self.children.len() as i32 - 1) * self.spacing
            - self.padding.y
            - self.padding.w;

        let equalized_sizes = equalize_sizes(&self.children, Height, Height, inner_height);

        for (i, current) in equalized_sizes {
            self.children[i].grow_size(Size::new(inner_width, current));
        }

        if let Some(layout) = self.layout.as_mut() {
            layout.current_size = outer_size;
        }
    }

    fn place(&mut self, position: Position<i32>) -> Size<i32> {
        self.position = position;

        let mut cursor = Position::new(
            self.position.x + self.padding.x,
            self.position.y + self.padding.y,
        );
        for child in self.children.iter_mut() {
            let child_size = child.place(cursor);
            cursor.y += child_size.height + self.spacing;
        }

        self.size.into_fixed()
    }

    fn draw(&self, instances: &mut Vec<Instance>) {
        instances.push(Instance::ui(
            self.position,
            self.size.into_fixed(),
            self.color,
        ));
        for child in self.children.iter() {
            child.draw(instances);
        }
    }

    fn handle(&mut self, globals: &Globals, ctx: &mut Context<M>) {
        for child in self.children.iter_mut() {
            child.handle(globals, ctx);
        }
    }
}
