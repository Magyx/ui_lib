use super::*;

pub struct Row<M: 'static> {
    pub id: Id,
    pub children: Vec<Element<M>>,
    pub layout: Layout,
    pub spacing: i32,
    pub border: BorderStyle,
    pub background_color: Color<f32>,
}

#[macro_export]
macro_rules! row {
    [] => {
        $crate::widget::Row {
            id: $crate::ui_id!(),
            children: vec![],
            layout: $crate::widget::Layout::default(),
            spacing: 0,
            border: $crate::widget::BorderStyle::default(),
            background_color: $crate::model::Color::WHITE,
        }
    };
    [ $($child:expr),* $(,)? ] => {
        $crate::widget::Row {
            id: $crate::ui_id!(),
            children: vec![$($child.into()),*],
            layout: $crate::widget::Layout::default(),
            spacing: 0,
            border: $crate::widget::BorderStyle::default(),
            background_color: $crate::model::Color::WHITE,
        }
    };
    ( $children:expr ) => {
        $crate::widget::Row {
            id: $crate::ui_id!(),
            children: $children,
            layout: $crate::widget::Layout::default(),
            spacing: 0,
            border: $crate::widget::BorderStyle::default(),
            background_color: $crate::model::Color::WHITE,
        }
    };
}

impl<M: 'static> Row<M> {
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    pub fn spacing(mut self, spacing: i32) -> Self {
        self.spacing = spacing;
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

impl<M: 'static> From<Row<M>> for Element<M> {
    fn from(r: Row<M>) -> Self {
        Element {
            widget: Box::new(r),
        }
    }
}

impl<M: 'static> Widget for Row<M> {
    type Message = M;

    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        textures: &TextureArray,
        texts: &mut TextBundle,
        ctx: &mut Context<Self::Message>,
    ) -> Result<RenderOutput, &'static str> {
        let (_, _, _, content_size) = resolve_layout(&self.layout, &parent_size);

        let (child_data, total_w, _total_h, _max_w, max_h) =
            measure_children(&self.children, content_size, textures, texts, ctx)?;

        let content_min = Size::new(
            total_w
                + self.spacing * self.children.len().saturating_sub(1) as i32
                + self.layout.padding.width as i32 * 2,
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

        let mut cursor_x = inner_pos.x;

        for (primitives, texts, child_w, _child_h) in child_data {
            if let Some(mut prims) = primitives {
                for p in prims.iter_mut() {
                    p.primitive.position[0] += cursor_x as f32;
                    p.primitive.position[1] += inner_pos.y as f32;
                }
                all_primitives.extend(prims);
            }
            if let Some(mut txts) = texts {
                for t in txts.iter_mut() {
                    t.position.x += cursor_x;
                    t.position.y += inner_pos.y;
                }
                all_texts.extend(txts);
            }
            cursor_x += child_w + self.spacing;
        }

        Ok(RenderOutput {
            primitives: Some(all_primitives),
            texts: Some(all_texts),
        })
    }
}
