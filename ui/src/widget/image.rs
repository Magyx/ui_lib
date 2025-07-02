use super::*;
use crate::graphics::TextureHandle;

pub struct Image<M: 'static> {
    pub id: Id,
    pub texture_handle: TextureHandle,
    pub layout: Layout,
    pub border: BorderStyle,
    pub fit: ContentFit,

    pub _marker: std::marker::PhantomData<M>,
}

#[macro_export]
macro_rules! image {
    ($texture_handle:expr) => {
        $crate::widget::Image {
            id: $crate::ui_id!(),
            texture_handle: $texture_handle,
            layout: $crate::widget::Layout::default(),
            border: $crate::widget::BorderStyle::default(),
            fit: $crate::widget::ContentFit::Cover,
            _marker: std::marker::PhantomData,
        }
    };
}

impl<M: 'static> Image<M> {
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = border;
        self
    }

    pub fn fit(mut self, fit: ContentFit) -> Self {
        self.fit = fit;
        self
    }
}

impl<M: 'static> From<Image<M>> for Element<M> {
    fn from(img: Image<M>) -> Self {
        Element {
            widget: Box::new(img),
        }
    }
}

impl<M: 'static> Widget for Image<M> {
    type Message = M;

    fn as_primitive(
        &self,
        parent_size: Size<i32>,
        textures: &TextureArray,
        _texts: &mut TextBundle,
        _ctx: &mut Context<Self::Message>,
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
