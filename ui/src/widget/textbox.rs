use super::*;

pub struct TextBox<M> {
    pub id: Id,
    pub content: String,
    pub text_style: TextStyle,
    pub layout: Layout,
    pub border: BorderStyle,
    pub background_color: Color<f32>,

    pub _marker: std::marker::PhantomData<M>,
}

#[macro_export]
macro_rules! text_box {
    ($content:expr) => {
        $crate::widget::TextBox {
            id: $crate::ui_id!(),
            content: $content,
            text_style: $crate::model::TextStyle::default(),
            layout: $crate::widget::Layout::default(),
            border: $crate::widget::BorderStyle::default(),
            background_color: $crate::model::Color::WHITE,
            _marker: std::marker::PhantomData,
        }
    };
}

impl<M: 'static> TextBox<M> {
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = style;
        self
    }

    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = border;
        self
    }

    pub fn background_color(mut self, color: Color<f32>) -> Self {
        self.background_color = color;
        self
    }
}

impl<M: 'static> From<TextBox<M>> for Element<M> {
    fn from(tb: TextBox<M>) -> Self {
        Element {
            widget: Box::new(tb),
        }
    }
}

impl<M> Widget for TextBox<M> {
    type Message = M;

    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        _textures: &TextureArray,
        texts: &mut TextBundle,
        _ctx: &mut Context<Self::Message>,
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
