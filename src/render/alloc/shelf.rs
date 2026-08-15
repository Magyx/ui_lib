use super::AtlasRect;

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
}
