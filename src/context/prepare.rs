use super::{Env, Id, ViewState};
use crate::{
    gpu::{Globals, Gpu},
    layout::LayoutEngine,
    model::{Position, Rect, Size},
    render::{
        pipeline::{Pipeline, PipelineRegistry},
        texture::TextureRegistry,
    },
    text::TextBackend,
    theme::Theme,
};

pub struct PrepareCtx<'a> {
    pub globals: &'a Globals,
    pub text: &'a mut dyn TextBackend,
    pub gpu: &'a Gpu,
    pub texture: &'a mut TextureRegistry,
    pub(crate) pipelines: &'a mut PipelineRegistry,
    pub(crate) surface_format: wgpu::TextureFormat,
    pub(crate) immediate_size: u32,
    pub(crate) layout: &'a LayoutEngine,
    pub(crate) current_node: usize,
    pub(crate) offset: Position<i32>,
    pub view_state: &'a mut ViewState,
    pub theme: &'a Theme,
    pub env: Env,
}

impl<'a> PrepareCtx<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        globals: &'a Globals,
        text: &'a mut dyn TextBackend,
        gpu: &'a Gpu,
        texture: &'a mut TextureRegistry,
        pipelines: &'a mut PipelineRegistry,
        surface_format: wgpu::TextureFormat,
        immediate_size: u32,
        layout: &'a LayoutEngine,
        view_state: &'a mut ViewState,
        theme: &'a Theme,
    ) -> Self {
        Self {
            globals,
            text,
            gpu,
            texture,
            pipelines,
            surface_format,
            immediate_size,
            layout,
            current_node: 0,
            offset: Position::splat(0),
            view_state,
            theme,
            env: theme.root_env(),
        }
    }
    /// The pipeline for `P`, building it if this is its first use.
    pub fn pipeline<P: Pipeline>(&mut self) -> &mut P {
        self.pipelines.get_or_insert::<P>(
            self.gpu,
            &self.surface_format,
            self.texture.layout(),
            self.immediate_size,
        )
    }

    /// Register `P` without needing a handle to it.
    pub fn ensure_pipeline<P: Pipeline>(&mut self) {
        let _ = self.pipeline::<P>();
    }

    pub(crate) fn __set_data(&mut self, current_node: usize, acc_tx: i32, acc_ty: i32, env: Env) {
        self.current_node = current_node;
        self.offset = Position::new(acc_tx, acc_ty);
        self.env = env;
    }
    pub fn current_node_id(&self) -> usize {
        self.current_node
    }
    pub fn rect(&self) -> Rect {
        let n = &self.layout.nodes[self.current_node];
        Rect::new(
            n.pos.x + self.offset.x,
            n.pos.y + self.offset.y,
            n.current_size.width,
            n.current_size.height,
        )
    }
    pub fn id(&self) -> Id {
        self.layout.nodes[self.current_node].id
    }
    pub fn first_child_node(&self) -> Option<usize> {
        self.layout.nodes[self.current_node].first_child
    }
    pub fn child_content_height(&self) -> i32 {
        if let Some(cid) = self.first_child_node() {
            self.layout.nodes[cid].content_size.height.max(0)
        } else {
            0
        }
    }
    pub fn physical_size(&self, logical: Size<u32>) -> Size<u32> {
        let sf = self.globals.scale;
        Size::new(
            (logical.width as f32 * sf).round() as u32,
            (logical.height as f32 * sf).round() as u32,
        )
    }
}
