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
                child.as_primitive(content_size, textures, texts, ctx)?;

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

        let layout = layout_with_fit(&self.layout, self_min);
        let (outer_pos, outer_size, _, _content_size) = resolve_layout(&layout, &parent_size);

        all_primitives[0] = Primitive::color(
            outer_pos,
            outer_size,
            self.background_color,
            Vector4::splat(self.border.radius),
            self.border.color,
            Vector4::splat(self.border.width),
        )
        .into();
        all_primitives[0].min_size = self_min.to_f32();

        Ok(RenderOutput {
            primitives: Some(all_primitives),
            texts: Some(all_texts),
        })
    }
}
