#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub enum AllocatorKind {
    /// Row-based shelf packing. Fast, minimal bookkeeping, no individual free.
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

/// Simple left-to-right, top-to-bottom shelf packer.
///
/// Rects are placed in rows (shelves) of height equal to the tallest rect on
/// that row. Individual rects cannot be freed; the whole allocator must be
/// reset. Very low overhead - O(1) alloc.
pub(crate) struct ShelfAllocator {
    width: u32,
    height: u32,
    cursor_x: u32,
    cursor_y: u32,
    row_h: u32,
}

impl ShelfAllocator {
    pub(crate) fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            cursor_x: 0,
            cursor_y: 0,
            row_h: 0,
        }
    }

    pub(crate) fn alloc(&mut self, w: u32, h: u32) -> Option<AtlasRect> {
        if w > self.width || h > self.height {
            return None;
        }
        if self.cursor_x + w > self.width {
            self.cursor_x = 0;
            self.cursor_y += self.row_h;
            self.row_h = 0;
        }
        if self.cursor_y + h > self.height {
            return None;
        }
        let rect = AtlasRect {
            x: self.cursor_x,
            y: self.cursor_y,
            w,
            h,
        };
        self.cursor_x += w;
        if h > self.row_h {
            self.row_h = h;
        }
        Some(rect)
    }

    pub(crate) fn reset(&mut self) {
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.row_h = 0;
    }
}

/// A skyline segment: the horizontal span [x, next_segment.x) sits at height y.
/// The last segment always extends to the atlas width.
#[derive(Clone, Copy, Debug)]
struct Segment {
    x: u32,
    y: u32,
}

/// Skyline (lowest-horizontal-line-first) packer.
///
/// Tracks the upper edge of occupied space as a step function and places each
/// new rect in the lowest available notch wide enough to fit it. Significantly
/// better density than shelf packing for mixed-size workloads.
///
/// Freed rects are stored in a free list and checked on every subsequent
/// `alloc`. This gives O(n) free-list search, acceptable for user atlases
/// with a small number of textures.
pub(crate) struct SkylineAllocator {
    width: u32,
    height: u32,
    skyline: Vec<Segment>,
    freed: Vec<AtlasRect>,
}

impl SkylineAllocator {
    pub(crate) fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            // Initially the entire width is at height 0.
            skyline: vec![Segment { x: 0, y: 0 }],
            freed: Vec::new(),
        }
    }

    pub(crate) fn alloc(&mut self, w: u32, h: u32) -> Option<AtlasRect> {
        // Degenerate: zero-size rects succeed without touching the skyline,
        // matching shelf allocator behaviour.
        if w == 0 || h == 0 {
            return Some(AtlasRect { x: 0, y: 0, w, h });
        }
        if w > self.width || h > self.height {
            return None;
        }

        // Check free list first (first-fit). Best-fit would require a full
        // scan anyway; first-fit keeps it simple.
        if let Some(pos) = self.freed.iter().position(|r| r.w >= w && r.h >= h) {
            let r = self.freed.swap_remove(pos);
            return Some(AtlasRect {
                x: r.x,
                y: r.y,
                w,
                h,
            });
        }

        // Find the skyline position with the lowest baseline that still fits.
        let mut best_x: Option<u32> = None;
        let mut best_baseline = u32::MAX;

        for seg in &self.skyline {
            let x = seg.x;
            if x + w > self.width {
                continue;
            }
            let baseline = self.max_height_in_range(x, w);
            if baseline + h <= self.height && baseline < best_baseline {
                best_baseline = baseline;
                best_x = Some(x);
            }
        }

        let x = best_x?;
        let y = best_baseline;
        self.raise_skyline(x, w, y + h);
        Some(AtlasRect { x, y, w, h })
    }

    pub(crate) fn free(&mut self, rect: AtlasRect) {
        if rect.w > 0 && rect.h > 0 {
            self.freed.push(rect);
        }
    }

    pub(crate) fn reset(&mut self) {
        self.skyline = vec![Segment { x: 0, y: 0 }];
        self.freed.clear();
    }

    /// Returns the maximum skyline height across the horizontal span [x, x+w).
    fn max_height_in_range(&self, x: u32, w: u32) -> u32 {
        let end = x + w;
        let n = self.skyline.len();
        let mut max_h = 0u32;
        for i in 0..n {
            let seg_start = self.skyline[i].x;
            let seg_end = if i + 1 < n {
                self.skyline[i + 1].x
            } else {
                self.width
            };
            // Segment [seg_start, seg_end) overlaps [x, end) when:
            if seg_start < end && seg_end > x {
                max_h = max_h.max(self.skyline[i].y);
            }
        }
        max_h
    }

    /// Raise the skyline profile over [x, x+w) to `new_h`.
    fn raise_skyline(&mut self, x: u32, w: u32, new_h: u32) {
        let end = x + w;
        let n = self.skyline.len();
        let mut result: Vec<Segment> = Vec::with_capacity(n + 2);
        let mut raised = false;

        for i in 0..n {
            let seg = self.skyline[i];
            let seg_end = if i + 1 < n {
                self.skyline[i + 1].x
            } else {
                self.width
            };

            if seg_end <= x {
                // Entirely before the raised region.
                result.push(seg);
                continue;
            }

            if seg.x >= end {
                // Entirely after the raised region. Insert the raised segment
                // before this one if we haven't already.
                if !raised {
                    result.push(Segment { x, y: new_h });
                    raised = true;
                }
                result.push(seg);
                continue;
            }

            // This segment overlaps [x, end).
            if seg.x < x {
                // Keep the portion before x.
                result.push(seg);
            }
            // Insert the raised segment once.
            if !raised {
                result.push(Segment { x, y: new_h });
                raised = true;
            }
            // If this segment extends past end, restore its original height there.
            if seg_end > end {
                result.push(Segment { x: end, y: seg.y });
            }
        }

        if !raised {
            result.push(Segment { x, y: new_h });
        }

        // Coalesce consecutive segments with the same height.
        let mut coalesced: Vec<Segment> = Vec::with_capacity(result.len());
        for seg in result {
            if coalesced.last().is_some_and(|s: &Segment| s.y == seg.y) {
                continue;
            }
            coalesced.push(seg);
        }

        self.skyline = coalesced;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shelf(w: u32, h: u32) -> ShelfAllocator {
        ShelfAllocator::new(w, h)
    }

    #[test]
    fn shelf_single_fits_at_origin() {
        let mut a = shelf(256, 256);
        let r = a.alloc(32, 16).unwrap();
        assert_eq!((r.x, r.y, r.w, r.h), (0, 0, 32, 16));
    }

    #[test]
    fn shelf_second_fits_next_to_first() {
        let mut a = shelf(256, 256);
        let r1 = a.alloc(32, 16).unwrap();
        let r2 = a.alloc(24, 16).unwrap();
        assert_eq!((r1.x, r1.y), (0, 0));
        assert_eq!((r2.x, r2.y), (32, 0));
    }

    #[test]
    fn shelf_row_h_tracks_tallest() {
        let mut a = shelf(100, 100);
        let _ = a.alloc(10, 5).unwrap();
        let _ = a.alloc(10, 20).unwrap();
        let _ = a.alloc(10, 5).unwrap();
        let _ = a.alloc(70, 1).unwrap(); // fills rest of row
        let wrap = a.alloc(5, 5).unwrap();
        assert_eq!(wrap.y, 20);
        assert_eq!(wrap.x, 0);
    }

    #[test]
    fn shelf_wraps_to_new_row() {
        let mut a = shelf(10, 20);
        let _ = a.alloc(6, 4).unwrap();
        let r = a.alloc(6, 4).unwrap();
        assert_eq!(r.x, 0);
        assert_eq!(r.y, 4);
    }

    #[test]
    fn shelf_rejects_oversized() {
        let mut a = shelf(32, 32);
        assert!(a.alloc(33, 10).is_none());
        assert!(a.alloc(10, 33).is_none());
    }

    #[test]
    fn shelf_rejects_when_vertical_full() {
        let mut a = shelf(5, 5);
        let _ = a.alloc(5, 5).unwrap();
        assert!(a.alloc(1, 1).is_none());
    }

    #[test]
    fn shelf_exact_fit() {
        let mut a = shelf(100, 50);
        let r = a.alloc(100, 50).unwrap();
        assert_eq!((r.x, r.y, r.w, r.h), (0, 0, 100, 50));
        assert!(a.alloc(1, 1).is_none());
    }

    #[test]
    fn shelf_zero_size_does_not_advance_cursor() {
        let mut a = shelf(100, 100);
        let _ = a.alloc(0, 0).unwrap();
        let r2 = a.alloc(10, 10).unwrap();
        assert_eq!((r2.x, r2.y), (0, 0));
    }

    #[test]
    fn shelf_fills_16_tiles_in_40x40() {
        let mut a = shelf(40, 40);
        let mut n = 0;
        while a.alloc(10, 10).is_some() {
            n += 1;
        }
        assert_eq!(n, 16);
    }

    #[test]
    fn shelf_wastes_vertical_space_on_mixed_heights() {
        // Regression guard: shelf packing deliberately wastes the space below
        // short rects on a tall row.
        let mut a = shelf(100, 100);
        let _ = a.alloc(100, 80).unwrap();
        let r = a.alloc(1, 1).unwrap();
        assert_eq!(r.y, 80);
    }

    #[test]
    fn shelf_reset_reuses_space() {
        let mut a = shelf(10, 10);
        let _ = a.alloc(10, 10).unwrap();
        assert!(a.alloc(1, 1).is_none());
        a.reset();
        let r = a.alloc(10, 10).unwrap();
        assert_eq!((r.x, r.y), (0, 0));
    }

    // SkylineAllocator

    fn sky(w: u32, h: u32) -> SkylineAllocator {
        SkylineAllocator::new(w, h)
    }

    #[test]
    fn skyline_single_at_origin() {
        let mut a = sky(256, 256);
        let r = a.alloc(32, 16).unwrap();
        assert_eq!((r.x, r.y), (0, 0));
        assert_eq!((r.w, r.h), (32, 16));
    }

    #[test]
    fn skyline_rejects_oversized() {
        let mut a = sky(32, 32);
        assert!(a.alloc(33, 10).is_none());
        assert!(a.alloc(10, 33).is_none());
    }

    #[test]
    fn skyline_zero_size_succeeds() {
        let mut a = sky(100, 100);
        let r = a.alloc(0, 0).unwrap();
        assert_eq!((r.w, r.h), (0, 0));
        // Should still be able to alloc real space afterwards.
        assert!(a.alloc(10, 10).is_some());
    }

    #[test]
    fn skyline_packs_tighter_than_shelf_on_mixed_heights() {
        // A 100x100 atlas gets a tall rect (100x80) then a small one (1x1).
        // Shelf puts the small one at y=80 (wasting 79 rows).
        // Skyline should also put it at y=80 since that's the only space left
        // after a full-width tall rect.  This test checks the small rect fits.
        let mut a = sky(100, 100);
        let _ = a.alloc(100, 80).unwrap();
        // 20 rows remain: a 1x1 should land there.
        let r = a.alloc(1, 1).unwrap();
        assert_eq!(r.y, 80);
    }

    #[test]
    fn skyline_uses_notch_efficiently() {
        // Place a 30x20 rect, then a 20x10 rect beside it, leaving a notch.
        // A 20x5 rect should fit in the lower notch at x=30, not at the top.
        let mut a = sky(100, 100);
        let _ = a.alloc(30, 20).unwrap(); // [0..30) raised to 20
        let _ = a.alloc(20, 10).unwrap(); // [30..50) raised to 10
        // Notch at x=30, y=10. A small rect should land there.
        let r = a.alloc(20, 5).unwrap();
        assert_eq!((r.x, r.y), (30, 10), "should fill the lower notch");
    }

    #[test]
    fn skyline_full_returns_none() {
        let mut a = sky(10, 10);
        let _ = a.alloc(10, 10).unwrap();
        assert!(a.alloc(1, 1).is_none());
    }

    #[test]
    fn skyline_free_makes_space_reclaimable() {
        let mut a = sky(50, 50);
        let r1 = a.alloc(50, 50).unwrap(); // fills atlas
        assert!(a.alloc(10, 10).is_none());
        a.free(r1);
        // After free, the free list should satisfy the next alloc.
        let r2 = a.alloc(10, 10);
        assert!(r2.is_some(), "freed space should be reclaimable");
    }

    #[test]
    fn skyline_reset_reclaims_all_space() {
        let mut a = sky(100, 100);
        let _ = a.alloc(100, 100).unwrap();
        assert!(a.alloc(1, 1).is_none());
        a.reset();
        let r = a.alloc(100, 100).unwrap();
        assert_eq!((r.x, r.y), (0, 0));
    }

    #[test]
    fn skyline_raise_skyline_correct_after_three_rects() {
        // Verify internal skyline shape through observable placement.
        let mut a = sky(60, 60);
        let r1 = a.alloc(20, 10).unwrap();
        let r2 = a.alloc(20, 30).unwrap();
        let r3 = a.alloc(20, 5).unwrap();
        // r1 at (0,0), r2 at (20,0) [tallest], r3 should land in the notch
        // left of r2 at x=40 if skyline correctly tracks the 10-high notch.
        assert_eq!((r1.x, r1.y), (0, 0));
        assert_eq!((r2.x, r2.y), (20, 0));
        // r3 has h=5; notch at x=40, y=0 is 60-40=20 wide and 60 tall.
        // Notch at x=0 after r1 is y=10, notch at x=40 is y=0.
        // Skyline should prefer x=40 (lower baseline).
        assert_eq!(r3.y, 0, "skyline should use the lowest notch");
        assert_eq!(r3.x, 40);
    }
}
