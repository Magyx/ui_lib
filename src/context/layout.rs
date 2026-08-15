use super::{Env, Id, ViewState};
use crate::{graphics::Globals, model::Size, text::TextBackend, theme::Theme};

pub struct LayoutCtx<'a> {
    pub globals: &'a Globals,
    pub view_state: &'a mut ViewState,
    pub text: &'a mut dyn TextBackend,
    pub theme: &'a Theme,
    pub env: Env,
    pub(crate) current_id: Id,
}

impl<'a> LayoutCtx<'a> {
    pub fn new(
        globals: &'a Globals,
        view_state: &'a mut ViewState,
        text: &'a mut dyn TextBackend,
        theme: &'a Theme,
    ) -> Self {
        Self {
            globals,
            view_state,
            text,
            theme,
            env: theme.root_env(),
            current_id: 0,
        }
    }
    pub(crate) fn __set_id(&mut self, id: Id) {
        self.current_id = id;
    }
    pub(crate) fn __set_env(&mut self, env: Env) {
        self.env = env;
    }
    pub fn id(&self) -> Id {
        self.current_id
    }
    pub fn physical_size(&self, logical: Size<u32>) -> Size<u32> {
        let sf = self.globals.scale;
        Size::new(
            (logical.width as f32 * sf).round() as u32,
            (logical.height as f32 * sf).round() as u32,
        )
    }
}
