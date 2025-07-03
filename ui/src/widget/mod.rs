pub use crate::{button, column, container, image, rectangle, row, text_box};
use crate::{
    context::{Context, Id},
    model::Size,
    model::*,
    primitive::Primitive,
    text::TextBundle,
    texture::TextureArray,
};

pub use button::Button;
pub use column::Column;
pub use container::Container;
pub use image::Image;
pub use rectangle::Rectangle;
pub use row::Row;
pub use textbox::TextBox;

mod button;
mod column;
mod container;
mod image;
mod rectangle;
mod row;
mod textbox;

pub struct RenderOutput<'a> {
    pub primitives: Option<Vec<PrimitiveWithMeta>>,
    pub texts: Option<Vec<Text<'a>>>,
}

pub trait Widget {
    type Message;

    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        textures: &TextureArray,
        texts: &mut TextBundle,
        ctx: &mut Context<Self::Message>,
    ) -> Result<RenderOutput, &'static str>;
}

pub struct Element<M: 'static> {
    pub widget: Box<dyn Widget<Message = M>>,
}

impl<M> Widget for Element<M> {
    type Message = M;

    fn as_primitive(
        &self,
        parent: Size<i32>,
        textures: &TextureArray,
        texts: &mut TextBundle,
        ctx: &mut Context<Self::Message>,
    ) -> Result<RenderOutput, &'static str> {
        self.widget.as_primitive(parent, textures, texts, ctx)
    }
}

impl<M> From<Vec<Element<M>>> for Element<M> {
    fn from(children: Vec<Element<M>>) -> Self {
        Element {
            widget: Box::new(Container {
                id: crate::ui_id!(),
                children,
                layout: Layout {
                    position: Position::splat(0),
                    size: Length::Fill,
                    margin: Size::splat(0),
                    padding: Size::splat(0),
                    align: Vector2::splat(0.0),
                },
                border: BorderStyle::default(),
                background_color: Color::WHITE,
            }),
        }
    }
}

pub struct PrimitiveWithMeta {
    pub min_size: Size<f32>,
    pub primitive: Primitive,
}

impl From<Primitive> for PrimitiveWithMeta {
    fn from(value: Primitive) -> Self {
        Self {
            min_size: value.size.into(),
            primitive: value,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TextStyle {
    pub color: Color<f32>,
    pub font: glyphon::Family<'static>,
    pub font_size: f32,
    pub weight: glyphon::Weight,
    pub italic: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            font_size: 16.0,
            font: glyphon::Family::SansSerif,
            weight: glyphon::Weight::NORMAL,
            italic: false,
        }
    }
}

pub struct Text<'a> {
    pub(crate) min_size: Size<f32>,
    pub content: &'a str,
    pub position: Position<i32>,
    pub size: Size<i32>,
    pub style: TextStyle,
}

#[derive(Clone, Copy)]
pub enum Length {
    Fit,
    Fill,
    Fixed(Size<Option<i32>>),
    Portion(Vector2<Option<u8>>),
}

fn size_from_len(len: &Length, parent_size: &Size<i32>) -> Size<i32> {
    match len {
        Length::Fit => Size::splat(0),
        Length::Fill => *parent_size,
        Length::Fixed(Size { width, height }) => Size {
            width: width.unwrap_or(parent_size.width),
            height: height.unwrap_or(parent_size.height),
        },
        Length::Portion(Vector2 { x, y }) => {
            let width_portion = x.unwrap_or(12).clamp(1, 12);
            let height_portion = y.unwrap_or(12).clamp(1, 12);
            Size {
                width: parent_size.width * width_portion as i32 / 12,
                height: parent_size.height * height_portion as i32 / 12,
            }
        }
    }
}

fn min_for_length(len: &Length, content: Size<i32>, parent: Size<i32>) -> Size<i32> {
    match len {
        Length::Fit => content,
        Length::Fill => Size::splat(0),
        Length::Fixed(Size { width, height }) => Size {
            width: width.unwrap_or(content.width),
            height: height.unwrap_or(content.height),
        },
        Length::Portion(Vector2 { x, y }) => Size {
            width: parent.width * x.unwrap_or(12) as i32 / 12,
            height: parent.height * y.unwrap_or(12) as i32 / 12,
        },
    }
}

#[derive(Clone, Copy)]
pub struct Layout {
    pub position: Position<i32>,
    pub size: Length,
    pub margin: Size<u32>,
    pub padding: Size<u32>,
    pub align: Vector2<f32>,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            position: Position::splat(0),
            size: Length::Fill,
            margin: Size::splat(0),
            padding: Size::splat(0),
            align: Vector2::splat(0.0),
        }
    }
}
fn layout_with_fit(layout: &Layout, measured: Size<i32>) -> Layout {
    let mut out = layout.clone();
    if matches!(out.size, Length::Fit) {
        out.size = Length::Fixed(Size::new(Some(measured.width), Some(measured.height)));
    }
    out
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
        align,
    } = layout;
    let mut outer_size = size_from_len(size, parent_size);
    outer_size.width -= margin.width as i32 * 2;
    outer_size.height -= margin.height as i32 * 2;

    let aligned_x = ((parent_size.width - outer_size.width) as f32 * align.x) as i32;
    let aligned_y = ((parent_size.height - outer_size.height) as f32 * align.y) as i32;

    let outer_position = Position::new(
        position.x + margin.width as i32 + aligned_x,
        position.y + margin.height as i32 + aligned_y,
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

fn measure_children<'a, M>(
    children: &'a [Element<M>],
    content_size: Size<i32>,
    textures: &TextureArray,
    texts: &mut TextBundle,
    ctx: &mut Context<M>,
) -> Result<
    (
        Vec<(
            Option<Vec<PrimitiveWithMeta>>,
            Option<Vec<Text<'a>>>,
            i32,
            i32,
        )>,
        i32,
        i32,
        i32,
        i32,
    ),
    &'static str,
> {
    let mut child_data = Vec::new();

    let mut total_w = 0;
    let mut total_h = 0;
    let mut max_w = 0;
    let mut max_h = 0;

    for child in children {
        let RenderOutput { primitives, texts } =
            child.as_primitive(content_size, textures, texts, ctx)?;

        let mut child_w = 0;
        let mut child_h = 0;

        if let Some(ref prims) = primitives {
            for p in prims.iter() {
                child_w = child_w.max(p.min_size.width as i32);
                child_h = child_h.max(p.min_size.height as i32);
            }
        }
        if let Some(ref txts) = texts {
            for t in txts.iter() {
                child_w = child_w.max(t.min_size.width as i32);
                child_h = child_h.max(t.min_size.height as i32);
            }
        }

        total_w += child_w;
        total_h += child_h;
        max_w = max_w.max(child_w);
        max_h = max_h.max(child_h);

        child_data.push((primitives, texts, child_w, child_h));
    }

    Ok((child_data, total_w, total_h, max_w, max_h))
}

#[derive(Clone, Copy)]
pub struct BorderStyle {
    pub radius: f32,
    pub color: Color<f32>,
    pub width: i32,
}

impl Default for BorderStyle {
    fn default() -> Self {
        Self {
            radius: 0.0,
            color: Color::splat(0.0),
            width: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub enum ContentFit {
    Fill,
    Contain,
    Cover,
    Width,
    Height,
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
