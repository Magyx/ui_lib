use crate::{
    event::{MouseButton, UiEventRef},
    graphics::{Globals, Gpu},
    layout::LayoutEngine,
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
    pub mouse_buttons_down: u32,
    pub mouse_buttons_pressed: u32,
    pub mouse_buttons_released: u32,

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
            mouse_buttons_down: 0,
            mouse_buttons_pressed: 0,
            mouse_buttons_released: 0,

            hot_item: None,
            active_item: None,
            kbd_focus_item: None,

            messages: Vec::new(),
            redraw_requested: false,
        }
    }
    #[inline]
    pub fn is_button_down(&self, b: MouseButton) -> bool {
        (self.mouse_buttons_down & (1 << b.bit())) != 0
    }
    #[inline]
    pub fn is_button_pressed(&self, b: MouseButton) -> bool {
        (self.mouse_buttons_pressed & (1 << b.bit())) != 0
    }
    #[inline]
    pub fn is_button_released(&self, b: MouseButton) -> bool {
        (self.mouse_buttons_released & (1 << b.bit())) != 0
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

pub struct LayoutCtx<'a, M> {
    pub globals: &'a Globals,
    pub ui: &'a mut Context<M>,
    pub text: &'a mut TextSystem,
}

pub struct PaintCtx<'a> {
    pub globals: &'a Globals,
    pub text: &'a mut TextSystem,
    pub gpu: &'a Gpu,
    pub texture: &'a mut TextureRegistry,
    pub(crate) layout: &'a LayoutEngine,
    pub(crate) current_node: usize,
}

impl<'a> PaintCtx<'a> {
    pub(crate) fn __set_current_node(&mut self, id: usize) {
        self.current_node = id;
    }

    pub fn current_node_id(&self) -> usize {
        self.current_node
    }

    pub fn first_child_node(&self) -> Option<usize> {
        self.layout.nodes[self.current_node].first_child
    }

    pub fn child_content_height(&self) -> i32 {
        if let Some(cid) = self.first_child_node() {
            self.layout.nodes[cid].content_height.max(0)
        } else {
            0
        }
    }
}

pub struct EventCtx<'a, M> {
    pub globals: &'a Globals,
    pub ui: &'a mut Context<M>,
    pub event: Option<UiEventRef<'a>>,
}
