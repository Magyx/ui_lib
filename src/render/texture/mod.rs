use crate::model::Size;

mod atlas;
pub use atlas::*;

mod registry;
pub use registry::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct TextureHandle {
    pub slot_gen: u32,
    pub scale_packed: u32,
    pub offset_packed: u32,
    pub size_px: Size<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::pack::{pack_slot_gen, pack_unorm2x16};

    #[test]
    fn texture_handle_default_is_all_zero() {
        // slot_gen == 0 is the sentinel for "no texture" — relied on by
        // TextSystem::upload_glyph which returns TextureHandle::default()
        // for oversized or zero-size glyphs.
        let h = TextureHandle::default();
        assert_eq!(h.slot_gen, 0);
        assert_eq!(h.scale_packed, 0);
        assert_eq!(h.offset_packed, 0);
        assert_eq!(h.size_px, Size::new(0, 0));
    }

    #[test]
    fn texture_handle_copy_and_eq() {
        let h1 = TextureHandle {
            slot_gen: pack_slot_gen(3, 7),
            scale_packed: pack_unorm2x16([1.0, 1.0]),
            offset_packed: pack_unorm2x16([0.25, 0.5]),
            size_px: Size::new(32, 32),
        };
        let h2 = h1; // Copy
        assert_eq!(h1, h2);
        assert_ne!(h1, TextureHandle::default());
    }
}
