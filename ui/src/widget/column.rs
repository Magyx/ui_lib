use super::*;

pub struct Column<M: 'static> {
    pub id: Id,
    pub children: Vec<Element<M>>,
    pub layout: Layout,
    pub spacing: i32,
    pub border: BorderStyle,
    pub background_color: Color<f32>,
}

#[macro_export]
macro_rules! column {
    [] => {
        $crate::widget::Column {
            id: $crate::ui_id!(),
            children: vec![],
            layout: $crate::widget::Layout::default(),
            spacing: 0,
            border: $crate::widget::BorderStyle::default(),
            background_color: $crate::model::Color::WHITE,
        }
    };
    [ $($child:expr),* $(,)? ] => {
        $crate::widget::Column {
            id: $crate::ui_id!(),
            children: vec![$($child.into()),*],
            layout: $crate::widget::Layout::default(),
            spacing: 0,
            border: $crate::widget::BorderStyle::default(),
            background_color: $crate::model::Color::WHITE,
        }
    };
    ( $children:expr ) => {
        $crate::widget::Column {
            id: $crate::ui_id!(),
            children: $children,
            layout: $crate::widget::Layout::default(),
            spacing: 0,
            border: $crate::widget::BorderStyle::default(),
            background_color: $crate::model::Color::WHITE,
        }
    };
}

impl<M: 'static> Column<M> {
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

impl<M: 'static> From<Column<M>> for Element<M> {
    fn from(c: Column<M>) -> Self {
        Element {
            widget: Box::new(c),
        }
    }
}

impl<M: 'static> Widget for Column<M> {
    type Message = M;

    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        textures: &TextureArray,
        texts: &mut TextBundle,
        ctx: &mut Context<Self::Message>,
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
            } = child.as_primitive(content_size, textures, texts, ctx)?;

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
