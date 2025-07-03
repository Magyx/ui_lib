use super::*;

pub struct Button<M> {
    pub id: Id,
    pub label: String,
    pub text_style: TextStyle,
    pub layout: Layout,
    pub border: BorderStyle,
    pub background_color: Color<f32>,
    pub hover_color: Color<f32>,
    pub pressed_color: Color<f32>,
    pub on_click: Option<M>,
}

#[macro_export]
macro_rules! button {
    ($label:expr) => {
        $crate::widget::Button {
            id: $crate::ui_id!(),
            label: $label.to_string(),
            text_style: $crate::widget::TextStyle::default(),
            layout: $crate::widget::Layout::default(),
            border: $crate::widget::BorderStyle::default(),
            background_color: $crate::model::Color::from_rgb(200, 200, 200),
            hover_color: $crate::model::Color::from_rgb(220, 220, 220),
            pressed_color: $crate::model::Color::from_rgb(180, 180, 180),
            on_click: None,
        }
    };
}

impl<M: Clone + 'static> Button<M> {
    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = text_style;
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

    pub fn hover_color(mut self, color: Color<f32>) -> Self {
        self.background_color = color;
        self
    }

    pub fn pressed_color(mut self, color: Color<f32>) -> Self {
        self.background_color = color;
        self
    }

    pub fn on_click(mut self, message: M) -> Self {
        self.on_click = Some(message);
        self
    }
}

impl<M: Clone + std::fmt::Debug + 'static> From<Button<M>> for Element<M> {
    fn from(btn: Button<M>) -> Self {
        Element {
            widget: Box::new(btn),
        }
    }
}

impl<M: Clone + std::fmt::Debug + 'static> Widget for Button<M> {
    type Message = M;

    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        _textures: &TextureArray,
        texts: &mut TextBundle,
        ctx: &mut Context<Self::Message>,
    ) -> Result<RenderOutput, &'static str> {
        let txt_min = texts.get_min_size(&self.text_style, &self.label);
        let content_min = Size::new(
            txt_min.width as i32 + self.layout.padding.width as i32 * 2,
            txt_min.height as i32 + self.layout.padding.height as i32 * 2,
        );

        let self_min = min_for_length(&self.layout.size, content_min, parent_size);
        let layout = layout_with_fit(&self.layout, self_min);

        let (outer_position, outer_size, inner_position, content_size) =
            resolve_layout(&layout, &parent_size);

        let state = ctx.item(self.id, outer_position, outer_size);

        let fill_color = if state.active {
            self.pressed_color
        } else if state.hovered {
            self.hover_color
        } else {
            self.background_color
        };
        dbg!(fill_color);

        if state.clicked {
            if let Some(ref on_click) = self.on_click {
                ctx.emit(on_click.clone());
            }
        }

        let background = PrimitiveWithMeta {
            primitive: Primitive::color(
                outer_position,
                outer_size,
                fill_color,
                Vector4::splat(self.border.radius),
                self.border.color,
                Vector4::splat(self.border.width),
            ),
            min_size: self_min.to_f32(),
        };

        let text = Text {
            min_size: txt_min,
            content: &self.label,
            position: inner_position,
            size: content_size,
            style: self.text_style,
        };

        Ok(RenderOutput {
            primitives: Some(vec![background]),
            texts: Some(vec![text]),
        })
    }
}
