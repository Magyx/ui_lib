use crate::{
    graphics::{Config, Globals},
    model::Position,
    render::{text::TextSystem, texture::TextureRegistry},
};

pub type Id = u64;

use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
pub fn next_id() -> Id {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

pub fn reset_ids_for_frame() {
    NEXT_ID.store(1, Ordering::Relaxed);
}

pub struct Context<M> {
    pub mouse_pos: Position<f32>,
    pub mouse_down: bool,
    pub mouse_pressed: bool,
    pub mouse_released: bool,

    pub hot_item: Option<Id>,
    pub active_item: Option<Id>,
    pub kbd_focus_item: Option<Id>,

    messages: Vec<M>,
    redraw_requested: bool,
}

impl<M> Default for Context<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Context<M> {
    pub fn new() -> Self {
        Self {
            mouse_pos: Position::splat(0.0),
            mouse_down: false,
            mouse_pressed: false,
            mouse_released: false,

            hot_item: None,
            active_item: None,
            kbd_focus_item: None,

            messages: Vec::new(),
            redraw_requested: false,
        }
    }

    pub fn take(&mut self) -> Vec<M> {
        std::mem::take(&mut self.messages)
    }

    pub fn emit(&mut self, msg: M) {
        self.messages.push(msg);
    }

    pub fn request_redraw(&mut self) {
        self.redraw_requested = true;
    }

    pub fn take_redraw(&mut self) -> bool {
        let r = self.redraw_requested;
        self.redraw_requested = false;
        r
    }
}

pub struct FitCtx<'a, M> {
    pub globals: &'a Globals,
    pub ui: &'a mut Context<M>,
    pub text: &'a mut TextSystem,
}

pub struct GrowCtx<'a, M> {
    pub globals: &'a Globals,
    pub ui: &'a mut Context<M>,
    pub text: &'a mut TextSystem,
}

pub struct PlaceCtx<'a, M> {
    pub globals: &'a Globals,
    pub ui: &'a mut Context<M>,
    pub text: &'a mut TextSystem,
}

pub struct PaintCtx<'a> {
    pub globals: &'a Globals,
    pub text: &'a mut TextSystem,
    pub config: &'a Config<'a>,
    pub texture: &'a mut TextureRegistry,
}

pub struct EventCtx<'a, M> {
    pub globals: &'a Globals,
    pub ui: &'a mut Context<M>,
}
