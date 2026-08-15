mod shelf;
use shelf::ShelfAllocator;

mod skyline;
use skyline::SkylineAllocator;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AllocatorKind {
    /// Row-based shelf packing. Fast, minimal bookkeeping, no individual free.
    #[default]
    Shelf,
    /// Skyline packing. Better density for mixed sizes; supports individual free.
    Skyline,
}

pub(crate) enum Allocator {
    Shelf(ShelfAllocator),
    Skyline(SkylineAllocator),
}

impl Allocator {
    pub(crate) fn new(kind: AllocatorKind, width: u32, height: u32) -> Self {
        match kind {
            AllocatorKind::Shelf => Self::Shelf(ShelfAllocator::new(width, height)),
            AllocatorKind::Skyline => Self::Skyline(SkylineAllocator::new(width, height)),
        }
    }

    /// Allocate a rect. Returns `None` if no space is available.
    pub(crate) fn alloc(&mut self, w: u32, h: u32) -> Option<AtlasRect> {
        match self {
            Self::Shelf(a) => a.alloc(w, h),
            Self::Skyline(a) => a.alloc(w, h),
        }
    }

    /// Mark a previously allocated rect as reclaimable.
    ///
    /// This is a no-op for `Shelf` — shelf packing cannot reclaim individual
    /// rects without resetting the entire atlas.
    pub(crate) fn free(&mut self, rect: AtlasRect) {
        match self {
            Self::Shelf(_) => {}
            Self::Skyline(a) => a.free(rect),
        }
    }

    /// Reset the allocator to its initial empty state.
    pub(crate) fn reset(&mut self) {
        match self {
            Self::Shelf(a) => a.reset(),
            Self::Skyline(a) => a.reset(),
        }
    }
}
