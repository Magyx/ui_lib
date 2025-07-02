use super::*;

pub struct Rectangle<M> {
    pub id: Id,
    pub layout: Layout,
    pub border: BorderStyle,
    pub color: Color<f32>,
    pub _marker: std::marker::PhantomData<M>,
}

#[macro_export]
macro_rules! rectangle {
    ( $color:expr ) => {
        $crate::widget::Rectangle {
            id: $crate::ui_id!(),
            layout: $crate::widget::Layout::default(),
            border: $crate::widget::BorderStyle {
                color: $color,
                ..Default::default()
            },
            color: $color,
            _marker: std::marker::PhantomData,
        }
    };
}

impl<M: 'static> Rectangle<M> {
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = border;
        self
    }

    pub fn color(mut self, color: Color<f32>) -> Self {
        self.color = color;
        self
    }
}

impl<M: 'static> From<Rectangle<M>> for Element<M> {
    fn from(rect: Rectangle<M>) -> Self {
        Element {
            widget: Box::new(rect),
        }
    }
}

impl<M> Widget for Rectangle<M> {
    type Message = M;

    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        _textures: &TextureArray,
        _texts: &mut TextBundle,
        _ctx: &mut Context<Self::Message>,
    ) -> Result<RenderOutput, &'static str> {
        let (outer_position, outer_size, _, _) = resolve_layout(&self.layout, &parent_size);

        let self_min = min_for_length(&self.layout.size, Size::splat(0), parent_size);

        let rect = PrimitiveWithMeta {
            min_size: self_min.to_f32(),
            primitive: Primitive::color(
                outer_position,
                outer_size,
                self.color,
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
