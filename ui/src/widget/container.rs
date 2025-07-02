use super::*;

pub struct Container<M: 'static> {
    pub id: Id,
    pub children: Vec<Element<M>>,
    pub layout: Layout,
    pub border: BorderStyle,
    pub background_color: Color<f32>,
}

#[macro_export]
macro_rules! container {
    [] => {
        $crate::widget::Container {
            id: $crate::ui_id!(),
            children: vec![],
            layout: $crate::widget::Layout::default(),
            border: $crate::widget::BorderStyle::default(),
            background_color: $crate::model::Color::WHITE,
        }
    };
    [ $( $child:expr ),* $(,)? ] => {
        $crate::widget::Container {
            id: $crate::ui_id!(),
            children: vec![$($child.into()),*],
            layout: $crate::widget::Layout::default(),
            border: $crate::widget::BorderStyle::default(),
            background_color: $crate::model::Color::WHITE,
        }
    };
    ( $children:expr ) => {
        $crate::widget::Container {
            id: $crate::ui_id!(),
            children: $children,
            layout: $crate::widget::Layout::default(),
            border: $crate::widget::BorderStyle::default(),
            background_color: $crate::model::Color::WHITE,
        }
    };
}

impl<M: 'static> Container<M> {
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

impl<M: 'static> From<Container<M>> for Element<M> {
    fn from(c: Container<M>) -> Self {
        Element {
            widget: Box::new(c),
        }
    }
}

impl<M: 'static> Widget for Container<M> {
    type Message = M;

    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        textures: &TextureArray,
        texts: &mut TextBundle,
        ctx: &mut Context<Self::Message>,
    ) -> Result<RenderOutput, &'static str> {
        let (_, _, _, content_size) = resolve_layout(&self.layout, &parent_size);

        let (child_data, _total_w, _total_h, max_w, max_h) =
            measure_children(&self.children, content_size, textures, texts, ctx)?;

        let content_min = Size::new(
            max_w + self.layout.padding.width as i32 * 2,
            max_h + self.layout.padding.height as i32 * 2,
        );

        let self_min = min_for_length(&self.layout.size, content_min, parent_size);
        let layout = layout_with_fit(&self.layout, self_min);
        let (outer_pos, outer_size, inner_pos, _) = resolve_layout(&layout, &parent_size);

        let background = PrimitiveWithMeta {
            primitive: Primitive::color(
                outer_pos,
                outer_size,
                self.background_color,
                Vector4::splat(self.border.radius),
                self.border.color,
                Vector4::splat(self.border.width),
            ),
            min_size: self_min.to_f32(),
        };

        let mut all_primitives = vec![background];
        let mut all_texts = Vec::new();

        for (primitives, texts, _child_w, _child_h) in child_data {
            if let Some(mut prims) = primitives {
                for p in prims.iter_mut() {
                    p.primitive.position[0] += inner_pos.x as f32;
                    p.primitive.position[1] += inner_pos.y as f32;
                }
                all_primitives.extend(prims);
            }
            if let Some(mut txts) = texts {
                for t in txts.iter_mut() {
                    t.position.x += inner_pos.x;
                    t.position.y += inner_pos.y;
                }
                all_texts.extend(txts);
            }
        }

        Ok(RenderOutput {
            primitives: Some(all_primitives),
            texts: Some(all_texts),
        })
    }
}
