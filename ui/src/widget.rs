use crate::{
    model::{Color, Length, Position, Size, Style, Vector4},
    primitive::Primitive,
};

pub struct RenderOutput<'a> {
    pub primitives: Option<Vec<Primitive>>,
    pub texts: Option<Vec<Text<'a>>>,
}

pub trait Widget {
    fn as_primitive(&self, parent_size: Size<i32>) -> RenderOutput;
}
pub struct Element {
    pub widget: Box<dyn Widget>,
}

impl Widget for Element {
    fn as_primitive(&self, parent: Size<i32>) -> RenderOutput {
        self.widget.as_primitive(parent)
    }
}

impl From<Vec<Element>> for Element {
    fn from(children: Vec<Element>) -> Self {
        Container {
            children,
            position: Position::from_scalar(0),
            size: Length::Fill,
            margin: Size::from_scalar(0),
            padding: Size::from_scalar(0),
            border_radius: 0.0,
            border_color: Color::from_scalar(0.0),
            border_width: 0,
            background_color: Color::WHITE,
        }
        .into()
    }
}

pub struct Text<'a> {
    pub content: &'a str,
    pub position: Position<i32>,
    pub size: Size<i32>,
    pub style: Style,
}

fn size_from_len(len: &Length, parent_size: &Size<i32>) -> Size<i32> {
    match len {
        Length::Fill => *parent_size,
        Length::Fixed(size) => Size::new(size.x, size.y),
    }
}

pub fn resolve_layout(
    position: Position<i32>,
    size: &Length,
    parent_size: &Size<i32>,
    margin: &Size<u32>,
    padding: &Size<u32>,
) -> (Position<i32>, Size<i32>, Position<i32>, Size<i32>) {
    let mut outer_size = size_from_len(size, parent_size);

    outer_size.width -= margin.width as i32 * 2;
    outer_size.height -= margin.height as i32 * 2;

    let outer_position = Position::new(
        position.x + margin.width as i32,
        position.y + margin.height as i32,
    );

    let content_size = Size::new(
        outer_size.width - padding.width as i32 * 2,
        outer_size.height - padding.height as i32 * 2,
    );

    let inner_position = Position::new(
        outer_position.x + padding.width as i32,
        outer_position.y + padding.height as i32,
    );

    (outer_position, outer_size, inner_position, content_size)
}

pub struct TextBox {
    pub content: String,
    pub text_style: Style,
    pub position: Position<i32>,
    pub size: Length,
    pub margin: Size<u32>,
    pub padding: Size<u32>,
    pub border_radius: f32,
    pub border_color: Color<f32>,
    pub border_width: i32,
    pub background_color: Color<f32>,
}

impl From<TextBox> for Element {
    fn from(tb: TextBox) -> Self {
        Element {
            widget: Box::new(tb),
        }
    }
}

impl Widget for TextBox {
    fn as_primitive(&self, parent_size: Size<i32>) -> RenderOutput {
        let (outer_position, outer_size, inner_position, content_size) = resolve_layout(
            self.position,
            &self.size,
            &parent_size,
            &self.margin,
            &self.padding,
        );

        let background = Primitive::color(
            outer_position,
            outer_size,
            self.background_color,
            self.border_color,
            Vector4::splat(self.border_radius),
            Vector4::splat(self.border_width),
        );

        let text = Text {
            content: &self.content,
            position: inner_position,
            size: content_size,
            style: self.text_style.clone(),
        };

        RenderOutput {
            primitives: Some(vec![background]),
            texts: Some(vec![text]),
        }
    }
}

pub struct Container {
    pub children: Vec<Element>,
    pub position: Position<i32>,
    pub size: Length,
    pub margin: Size<u32>,
    pub padding: Size<u32>,
    pub border_radius: f32,
    pub border_color: Color<f32>,
    pub border_width: i32,
    pub background_color: Color<f32>,
}

impl From<Container> for Element {
    fn from(c: Container) -> Self {
        Element {
            widget: Box::new(c),
        }
    }
}

impl Widget for Container {
    fn as_primitive(&self, parent_size: Size<i32>) -> RenderOutput {
        let mut all_primitives = vec![];
        let mut all_texts = vec![];

        let (outer_position, outer_size, inner_position, content_size) = resolve_layout(
            self.position,
            &self.size,
            &parent_size,
            &self.margin,
            &self.padding,
        );

        let background = Primitive::color(
            outer_position,
            outer_size,
            self.background_color,
            self.border_color,
            Vector4::splat(self.border_radius),
            Vector4::splat(self.border_width),
        );
        all_primitives.push(background);

        for child in self.children.iter() {
            let RenderOutput { primitives, texts } = child.as_primitive(content_size);

            if let Some(mut primitives) = primitives {
                for primitive in primitives.iter_mut() {
                    primitive.position[0] += inner_position.x as f32;
                    primitive.position[1] += inner_position.y as f32;
                }
                all_primitives.extend(primitives);
            }

            if let Some(mut texts) = texts {
                for text in texts.iter_mut() {
                    text.position.x += inner_position.x;
                    text.position.y += inner_position.y;
                }
                all_texts.extend(texts);
            }
        }

        RenderOutput {
            primitives: Some(all_primitives),
            texts: Some(all_texts),
        }
    }
}
