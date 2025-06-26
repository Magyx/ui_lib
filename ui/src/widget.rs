use crate::{
    graphics::{TextureArray, TextureHandle},
    model::{Color, Position, Size, Style, Vector2, Vector4},
    primitive::Primitive,
};

pub struct RenderOutput<'a> {
    pub primitives: Option<Vec<Primitive>>,
    pub texts: Option<Vec<Text<'a>>>,
}

pub trait Widget {
    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        textures: &TextureArray,
    ) -> Result<RenderOutput, &'static str>;
}
pub struct Element {
    pub widget: Box<dyn Widget>,
}

impl Widget for Element {
    fn as_primitive(
        &self,
        parent: Size<i32>,
        textures: &TextureArray,
    ) -> Result<RenderOutput, &'static str> {
        self.widget.as_primitive(parent, textures)
    }
}

impl From<Vec<Element>> for Element {
    fn from(children: Vec<Element>) -> Self {
        Container {
            children,
            layout: Layout {
                position: Position::from_scalar(0),
                size: Length::Fill,
                margin: Size::from_scalar(0),
                padding: Size::from_scalar(0),
            },
            background_color: Color::WHITE,
            border: BorderStyle::default(),
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

fn resolve_layout(
    layout: &Layout,
    parent_size: &Size<i32>,
) -> (Position<i32>, Size<i32>, Position<i32>, Size<i32>) {
    let Layout {
        position,
        size,
        margin,
        padding,
    } = layout;
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

pub enum Length {
    Fill,
    Fixed(Vector2<i32>),
}

pub struct Layout {
    pub position: Position<i32>,
    pub size: Length,
    pub margin: Size<u32>,
    pub padding: Size<u32>,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            position: Position::from_scalar(0),
            size: Length::Fill,
            margin: Size::from_scalar(0),
            padding: Size::from_scalar(0),
        }
    }
}

pub struct BorderStyle {
    pub radius: f32,
    pub color: Color<f32>,
    pub width: i32,
}

impl Default for BorderStyle {
    fn default() -> Self {
        Self {
            radius: 0.0,
            color: Color::from_scalar(0.0),
            width: 0,
        }
    }
}

pub struct Rectangle {
    pub layout: Layout,
    pub border: BorderStyle,
    pub background_color: Color<f32>,
}

impl From<Rectangle> for Element {
    fn from(rect: Rectangle) -> Self {
        Element {
            widget: Box::new(rect),
        }
    }
}

impl Widget for Rectangle {
    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        _textures: &TextureArray,
    ) -> Result<RenderOutput, &'static str> {
        let (outer_position, outer_size, _, _) = resolve_layout(&self.layout, &parent_size);

        let rect = Primitive::color(
            outer_position,
            outer_size,
            self.background_color,
            Vector4::splat(self.border.radius),
            self.border.color,
            Vector4::splat(self.border.width),
        );

        Ok(RenderOutput {
            primitives: Some(vec![rect]),
            texts: None,
        })
    }
}

pub struct TextBox {
    pub content: String,
    pub text_style: Style,
    pub layout: Layout,
    pub border: BorderStyle,
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
    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        _textures: &TextureArray,
    ) -> Result<RenderOutput, &'static str> {
        let (outer_position, outer_size, inner_position, content_size) =
            resolve_layout(&self.layout, &parent_size);

        let background = Primitive::color(
            outer_position,
            outer_size,
            self.background_color,
            Vector4::splat(self.border.radius),
            self.border.color,
            Vector4::splat(self.border.width),
        );

        let text = Text {
            content: &self.content,
            position: inner_position,
            size: content_size,
            style: self.text_style.clone(),
        };

        Ok(RenderOutput {
            primitives: Some(vec![background]),
            texts: Some(vec![text]),
        })
    }
}

pub struct Container {
    pub children: Vec<Element>,
    pub layout: Layout,
    pub border: BorderStyle,
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
    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        textures: &TextureArray,
    ) -> Result<RenderOutput, &'static str> {
        let mut all_primitives = vec![];
        let mut all_texts = vec![];

        let (outer_position, outer_size, inner_position, content_size) =
            resolve_layout(&self.layout, &parent_size);

        let background = Primitive::color(
            outer_position,
            outer_size,
            self.background_color,
            Vector4::splat(self.border.radius),
            self.border.color,
            Vector4::splat(self.border.width),
        );
        all_primitives.push(background);

        for child in self.children.iter() {
            let RenderOutput { primitives, texts } = child.as_primitive(content_size, textures)?;

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

        Ok(RenderOutput {
            primitives: Some(all_primitives),
            texts: Some(all_texts),
        })
    }
}

#[derive(Clone, Copy)]
pub enum ContentFit {
    Fill = 0,
    Contain = 1,
    Cover = 2,
    Width = 3,
    Height = 4,
}

fn fit_image(box_size: Size<i32>, img_w: f32, img_h: f32, fit: ContentFit) -> [f32; 4] {
    let box_w = box_size.width as f32;
    let box_h = box_size.height as f32;
    let img_ar = img_w / img_h;
    let box_ar = box_w / box_h;

    match fit {
        ContentFit::Fill => [0.0, 0.0, 1.0, 1.0],
        ContentFit::Contain => {
            if img_ar > box_ar {
                let new_h = img_w / box_ar;
                let pad = (img_h - new_h) * 0.5;
                let v0 = pad / img_h;
                let v1 = 1.0 - v0 * 2.0;
                [0.0, v0, 1.0, v1]
            } else {
                let new_w = img_h * box_ar;
                let pad = (img_w - new_w) * 0.5;
                let u0 = pad / img_w;
                let u1 = 1.0 - u0 * 2.0;
                [u0, 0.0, u1, 1.0]
            }
        }
        ContentFit::Cover => {
            if img_ar > box_ar {
                let new_w = img_h * box_ar;
                let pad = (img_w - new_w) * 0.5;
                let u0 = pad / img_w;
                let u1 = (pad + new_w) / img_w - u0;
                [u0, 0.0, u1, 1.0]
            } else {
                let new_h = img_w / box_ar;
                let pad = (img_h - new_h) * 0.5;
                let v0 = pad / img_h;
                let v1 = (pad + new_h) / img_h - v0;
                [0.0, v0, 1.0, v1]
            }
        }
        ContentFit::Width => {
            let new_h = img_w / box_ar;
            let pad = (img_h - new_h) * 0.5;
            let v0 = pad / img_h;
            let v1 = 1.0 - v0 * 2.0;
            [0.0, v0, 1.0, v1]
        }
        ContentFit::Height => {
            let new_w = img_h * box_ar;
            let pad = (img_w - new_w) * 0.5;
            let u0 = pad / img_w;
            let u1 = 1.0 - u0 * 2.0;
            [u0, 0.0, u1, 1.0]
        }
    }
}

pub struct Image {
    pub texture_handle: TextureHandle,
    pub layout: Layout,
    pub border: BorderStyle,
    pub fit: ContentFit,
}

impl From<Image> for Element {
    fn from(img: Image) -> Self {
        Element {
            widget: Box::new(img),
        }
    }
}

impl Widget for Image {
    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        textures: &TextureArray,
    ) -> Result<RenderOutput, &'static str> {
        let (outer_position, outer_size, _, _) = resolve_layout(&self.layout, &parent_size);

        let (tex_id, img_dims) = textures.get_tex_info(&self.texture_handle)?;
        let uv_rect = fit_image(
            outer_size,
            img_dims.width as f32,
            img_dims.height as f32,
            self.fit,
        );

        let image = Primitive::texture(
            outer_position,
            outer_size,
            tex_id,
            uv_rect,
            Vector4::splat(self.border.radius),
            self.border.color,
            Vector4::splat(self.border.width),
        );

        Ok(RenderOutput {
            primitives: Some(vec![image]),
            texts: None,
        })
    }
}
