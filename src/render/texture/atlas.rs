use super::TextureHandle;
use crate::{
    model::Size,
    render::{
        AllocatorKind,
        alloc::{Allocator, AtlasRect},
        pack::unpack_unorm2x16,
    },
};

pub struct Atlas {
    pub(crate) slot_index: usize,
    pub(crate) generation: u32,
    pub(crate) size_px: Size<u32>,
    pub(crate) allocator: Allocator,
}

impl Atlas {
    pub(crate) fn new(
        slot_index: usize,
        generation: u32,
        size_px: Size<u32>,
        kind: AllocatorKind,
    ) -> Self {
        Self {
            slot_index,
            generation,
            size_px,
            allocator: Allocator::new(kind, size_px.width, size_px.height),
        }
    }

    /// Allocate a `w × h` rect inside this atlas. Returns `None` when full.
    pub(crate) fn alloc(&mut self, w: u32, h: u32) -> Option<AtlasRect> {
        self.allocator.alloc(w, h)
    }

    /// Mark the region occupied by `handle` as reclaimable.
    ///
    /// Only meaningful for atlases created with [`AllocatorKind::Skyline`];
    /// calling this on a `Shelf` atlas is a safe no-op (shelf packing cannot
    /// reclaim individual rects without resetting the whole atlas).
    pub fn free(&mut self, handle: TextureHandle) {
        let (ox, oy) = unpack_unorm2x16(handle.offset_packed);
        let x = (ox * self.size_px.width as f32).round() as u32;
        let y = (oy * self.size_px.height as f32).round() as u32;
        self.allocator.free(AtlasRect {
            x,
            y,
            w: handle.size_px.width,
            h: handle.size_px.height,
        });
    }
}
