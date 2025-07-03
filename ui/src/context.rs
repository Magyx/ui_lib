use crate::model::{Position, Size};

pub type Id = u64;

#[macro_export]
macro_rules! ui_id {
    () => {{
        const STR: &str = concat!(file!(), ":", line!());
        const CRC: u32 = $crate::const_crc32::crc32(STR.as_bytes());
        CRC as u64
    }};
}

#[derive(Debug)]
pub struct ItemState {
    pub hovered: bool,
    pub active: bool,
    pub clicked: bool,
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
}

impl<'a, M> Context<M> {
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
        }
    }

    pub fn take(&mut self) -> Vec<M> {
        std::mem::take(&mut self.messages)
    }

    pub fn emit(&mut self, msg: M) {
        self.messages.push(msg);
    }

    pub fn item(&mut self, id: Id, pos: Position<i32>, size: Size<i32>) -> ItemState {
        dbg!(pos);
        dbg!(size);
        dbg!(self.mouse_pos);
        let hovered = {
            let p = self.mouse_pos;
            p.x >= pos.x as f32
                && p.y >= pos.y as f32
                && p.x < (pos.x + size.width) as f32
                && p.y < (pos.y + size.height) as f32
        };

        if hovered {
            self.hot_item = Some(id);
            if self.mouse_pressed {
                self.active_item = Some(id);
            }
        }

        let active = self.active_item == Some(id);
        let clicked = active && self.mouse_released && hovered;

        ItemState {
            hovered,
            active,
            clicked,
        }
    }
}
