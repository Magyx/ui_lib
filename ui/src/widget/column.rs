use super::*;

pub struct Column<M> {
    layout: Option<Layout>,

    id: Id,
    children: Vec<Element<M>>,
    spacing: i32,
    position: Position<i32>,
    size: Size<Length<i32>>,
    color: Color<f32>,
    padding: Vec4<i32>,
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

        let mut remaining_height = self.size.into_fixed().height
            - (self.children.len() as i32 - 1) * self.spacing
            - self.padding.y
            - self.padding.w;
        let mut grow_items = Vec::with_capacity(self.children.len());
        for (index, child) in self.children.iter().enumerate() {
            let Layout { size, current_size } = child.layout();
            remaining_height -= current_size.height;
            if matches!(size.height, Length::Grow) {
                grow_items.push((index, current_size));
            }
        }

        let width = self.size.into_fixed().width - self.padding.x - self.padding.z;
        if grow_items.len() > 1 {
            while remaining_height > grow_items.len() as i32 {
                let mut smallest = grow_items[0];
                let mut second_smallest = grow_items[1];
                let mut height_to_add = remaining_height;
                for child in grow_items.iter() {
                    if child.1.height < smallest.1.height {
                        second_smallest = smallest;
                        smallest = *child;
                    }

                    if child.1.height > smallest.1.height {
                        second_smallest.1.height = second_smallest.1.height.min(child.1.height);
                        height_to_add = second_smallest.1.height - smallest.1.height;
                    }
                }
                height_to_add = height_to_add.min(remaining_height / grow_items.len() as i32);

                for (_, size) in grow_items.iter_mut() {
                    if size.height == smallest.1.height {
                        size.height += height_to_add;
                        remaining_height -= height_to_add;
                    }
                }
            }

            for child in grow_items.iter() {
                self.children[child.0].grow_size(Size::new(width, child.1.height));
            }
        } else if !grow_items.is_empty() {
            let grow_size = Size::new(width, remaining_height);
            self.children[grow_items[0].0].grow_size(grow_size);
        }

        for i in 0..self.children.len() {
            let h = self.children[i].layout().current_size.height;
            self.children[i].grow_size(Size::new(width, h));
        }

        if let Some(layout) = self.layout.as_mut() {
            layout.current_size = self.size.into_fixed();
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

    fn draw(&self, primitives: &mut Vec<Primitive>) {
        primitives.push(Primitive::color(
            self.position,
            self.size.into_fixed(),
            self.color,
        ));
        for child in self.children.iter() {
            child.draw(primitives);
        }
    }

    fn handle(&mut self, ctx: &mut Context<M>) {
        for child in self.children.iter_mut() {
            child.handle(ctx);
        }
    }
}
