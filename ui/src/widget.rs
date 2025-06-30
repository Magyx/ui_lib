use crate::{
    graphics::{TextBundle, TextureArray, TextureHandle},
    model::{Color, Position, Size, TextStyle, Vector2, Vector4},
    primitive::Primitive,
};

pub struct RenderOutput<'a> {
    pub primitives: Option<Vec<PrimitiveWithMeta>>,
    pub texts: Option<Vec<Text<'a>>>,
}

#[allow(unused_variables)]
pub trait Widget {
    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        textures: &TextureArray,
        texts: &mut TextBundle,
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
        texts: &mut TextBundle,
    ) -> Result<RenderOutput, &'static str> {
        self.widget.as_primitive(parent, textures, texts)
    }
}

impl From<Vec<Element>> for Element {
    fn from(children: Vec<Element>) -> Self {
        Container {
            children,
            layout: Layout {
                position: Position::splat(0),
                size: Length::Fill,
                margin: Size::splat(0),
                padding: Size::splat(0),
                align: Vector2::splat(0.0),
            },
            background_color: Color::WHITE,
            border: BorderStyle::default(),
        }
        .into()
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
            position: Position::from_scalar(0),
            size: Length::Fill,
            margin: Size::from_scalar(0),
            padding: Size::from_scalar(0),
            align: Vector2::from_scalar(0.0),
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
        _texts: &mut TextBundle,
    ) -> Result<RenderOutput, &'static str> {
        let (outer_position, outer_size, _, _) = resolve_layout(&self.layout, &parent_size);

        let self_min = min_for_length(&self.layout.size, Size::splat(0), parent_size);

        let rect = PrimitiveWithMeta {
            min_size: self_min.to_f32(),
            primitive: Primitive::color(
                outer_position,
                outer_size,
                self.background_color,
                Vector4::splat(self.border.radius),
                self.border.color,
                Vector4::splat(self.border.width),
            ),
        };

        Ok(RenderOutput {
            primitives: Some(vec![rect]),
            texts: None,
        })
    }
}

pub struct TextBox {
    pub content: String,
    pub text_style: TextStyle,
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
        texts: &mut TextBundle,
    ) -> Result<RenderOutput, &'static str> {
        let txt_min = texts.get_min_size(&self.text_style, &self.content);
        let content_min = Size::new(
            txt_min.width as i32 + self.layout.padding.width as i32 * 2,
            txt_min.height as i32 + self.layout.padding.height as i32 * 2,
        );

        let self_min = min_for_length(&self.layout.size, content_min, parent_size);
        let layout = layout_with_fit(&self.layout, self_min);

        let (outer_position, outer_size, inner_position, content_size) =
            resolve_layout(&layout, &parent_size);

        let mut background: PrimitiveWithMeta = Primitive::color(
            outer_position,
            outer_size,
            self.background_color,
            Vector4::splat(self.border.radius),
            self.border.color,
            Vector4::splat(self.border.width),
        )
        .into();
        background.min_size = self_min.to_f32();

        let text = Text {
            min_size: txt_min,
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
        texts: &mut TextBundle,
    ) -> Result<RenderOutput, &'static str> {
        let mut all_primitives = vec![];
        let mut all_texts = vec![];

        let (outer_position, outer_size, inner_position, content_size) =
            resolve_layout(&self.layout, &parent_size);

        all_primitives.push(
            Primitive::color(
                outer_position,
                outer_size,
                self.background_color,
                Vector4::splat(self.border.radius),
                self.border.color,
                Vector4::splat(self.border.width),
            )
            .into(),
        );

        let mut max_child_w = 0;
        let mut max_child_h = 0;

        for child in self.children.iter() {
            let RenderOutput { primitives, texts } =
                child.as_primitive(content_size, textures, texts)?;

            if let Some(mut primitives) = primitives {
                for p in primitives.iter_mut() {
                    p.primitive.position[0] += inner_position.x as f32;
                    p.primitive.position[1] += inner_position.y as f32;
                    max_child_w = max_child_w.max(p.min_size.width as i32);
                    max_child_h = max_child_h.max(p.min_size.height as i32);
                }

                all_primitives.extend(primitives);
            }

            if let Some(mut texts) = texts {
                for t in texts.iter_mut() {
                    t.position.x += inner_position.x;
                    t.position.y += inner_position.y;
                    max_child_w = max_child_w.max(t.min_size.width as i32);
                    max_child_h = max_child_h.max(t.min_size.height as i32);
                }
                all_texts.extend(texts);
            }
        }

        let content_min = Size::new(max_child_w, max_child_h);
        let self_min = min_for_length(&self.layout.size, content_min, parent_size);
        all_primitives[0].min_size = self_min.to_f32();

        Ok(RenderOutput {
            primitives: Some(all_primitives),
            texts: Some(all_texts),
        })
    }
}

pub struct Column {
    pub children: Vec<Element>,
    pub layout: Layout,
    pub spacing: i32,
    pub border: BorderStyle,
    pub background_color: Color<f32>,
}

impl From<Column> for Element {
    fn from(c: Column) -> Self {
        Element {
            widget: Box::new(c),
        }
    }
}

impl Widget for Column {
    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        textures: &TextureArray,
        texts: &mut TextBundle,
    ) -> Result<RenderOutput, &'static str> {
        let mut all_primitives = Vec::new();
        let mut all_texts = Vec::new();

        let (outer_pos, outer_size, inner_pos, content_size) =
            resolve_layout(&self.layout, &parent_size);

        all_primitives.push(
            Primitive::color(
                outer_pos,
                outer_size,
                self.background_color,
                Vector4::splat(self.border.radius),
                self.border.color,
                Vector4::splat(self.border.width),
            )
            .into(),
        );

        let mut cursor_y = inner_pos.y;
        let mut max_child_w = 0;
        let mut content_h = 0;

        for child in &self.children {
            let RenderOutput {
                primitives,
                texts: child_texts,
            } = child.as_primitive(content_size, textures, texts)?;

            let mut child_height: i32 = 0;
            let mut child_width: i32 = 0;

            if let Some(mut prims) = primitives {
                for p in prims.iter_mut() {
                    p.primitive.position[0] += inner_pos.x as f32;
                    p.primitive.position[1] += cursor_y as f32;
                    child_height = child_height.max(p.min_size.height as i32);
                    child_width = child_width.max(p.min_size.width as i32);
                }
                all_primitives.extend(prims);
            }

            if let Some(mut txts) = child_texts {
                for t in txts.iter_mut() {
                    t.position.x += inner_pos.x;
                    t.position.y += cursor_y;
                    child_height = child_height.max(t.min_size.height as i32);
                    child_width = child_width.max(t.min_size.width as i32);
                }
                all_texts.extend(txts);
            }

            cursor_y += child_height + self.spacing;
            max_child_w = max_child_w.max(child_width);
            content_h += child_height;
        }

        let content_min = Size::new(
            max_child_w,
            content_h + self.spacing * self.children.len().saturating_sub(1) as i32,
        );
        let self_min = min_for_length(&self.layout.size, content_min, parent_size);
        all_primitives[0].min_size = self_min.to_f32();

        Ok(RenderOutput {
            primitives: Some(all_primitives),
            texts: Some(all_texts),
        })
    }
}

pub struct Row {
    pub children: Vec<Element>,
    pub layout: Layout,
    pub spacing: i32,
    pub border: BorderStyle,
    pub background_color: Color<f32>,
}

impl From<Row> for Element {
    fn from(r: Row) -> Self {
        Element {
            widget: Box::new(r),
        }
    }
}

impl Widget for Row {
    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        textures: &TextureArray,
        texts: &mut TextBundle,
    ) -> Result<RenderOutput, &'static str> {
        let mut all_primitives = Vec::new();
        let mut all_texts = Vec::new();

        let (outer_pos, outer_size, inner_pos, content_size) =
            resolve_layout(&self.layout, &parent_size);

        all_primitives.push(
            Primitive::color(
                outer_pos,
                outer_size,
                self.background_color,
                Vector4::splat(self.border.radius),
                self.border.color,
                Vector4::splat(self.border.width),
            )
            .into(),
        );

        let mut cursor_x = inner_pos.x;
        let mut max_child_h = 0;
        let mut content_w = 0;

        for child in &self.children {
            let RenderOutput {
                primitives,
                texts: child_texts,
            } = child.as_primitive(content_size, textures, texts)?;

            let mut child_width: i32 = 0;
            let mut child_height: i32 = 0;

            if let Some(mut prims) = primitives {
                for p in prims.iter_mut() {
                    p.primitive.position[0] += cursor_x as f32;
                    p.primitive.position[1] += inner_pos.y as f32;
                    child_width = child_width.max(p.min_size.width as i32);
                    child_height = child_height.max(p.min_size.height as i32);
                }
                all_primitives.extend(prims);
            }

            if let Some(mut txts) = child_texts {
                for t in txts.iter_mut() {
                    t.position.x += cursor_x;
                    t.position.y += inner_pos.y;
                    child_width = child_width.max(t.min_size.width as i32);
                    child_height = child_height.max(t.min_size.height as i32);
                }
                all_texts.extend(txts);
            }

            cursor_x += child_width + self.spacing;
            max_child_h = max_child_h.max(child_height);
            content_w += child_width;
        }

        let content_min = Size::new(
            content_w + self.spacing * self.children.len().saturating_sub(1) as i32,
            max_child_h,
        );
        let self_min = min_for_length(&self.layout.size, content_min, parent_size);
        all_primitives[0].min_size = self_min.to_f32();

        Ok(RenderOutput {
            primitives: Some(all_primitives),
            texts: Some(all_texts),
        })
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
        _texts: &mut TextBundle,
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
        )
        .into();

        Ok(RenderOutput {
            primitives: Some(vec![image]),
            texts: None,
        })
    }
}
