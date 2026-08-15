#[inline]
pub fn pack_unorm2x16(xy: [f32; 2]) -> u32 {
    let q = |v: f32| -> u32 { (v.clamp(0.0, 1.0) * 65535.0 + 0.5).floor() as u32 };
    q(xy[0]) | (q(xy[1]) << 16)
}
#[inline]
pub fn unpack_unorm2x16(packed: u32) -> (f32, f32) {
    let x = (packed & 0xFFFF) as f32 / 65535.0;
    let y = (packed >> 16) as f32 / 65535.0;
    (x, y)
}

#[inline]
pub fn pack_unorm4x8(xyzw: [f32; 4]) -> u32 {
    let q = |v: f32| -> u32 { (v.clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u32 };
    q(xyzw[0]) | (q(xyzw[1]) << 8) | (q(xyzw[2]) << 16) | (q(xyzw[3]) << 24)
}

pub mod shape_flags {
    pub const HAS_BORDER: u8 = 1 << 0;
    pub const HAS_SHADOW: u8 = 1 << 1;
    pub const GRADIENT_V: u8 = 1 << 2;
}

#[inline]
pub fn pack_shape_params(
    corner_radius: f32,
    border_width: f32,
    shadow_radius: f32,
    flags: u8,
) -> u32 {
    let r = corner_radius.clamp(0.0, 255.0) as u32;
    let b = (border_width * 4.0).clamp(0.0, 255.0) as u32;
    let s = shadow_radius.clamp(0.0, 255.0) as u32;
    r | (b << 8) | (s << 16) | ((flags as u32) << 24)
}

#[inline]
pub fn unpack_shape_params(packed: u32) -> (f32, f32, f32, u8) {
    let r = (packed & 0xFF) as f32;
    let b = ((packed >> 8) & 0xFF) as f32 * 0.25;
    let s = ((packed >> 16) & 0xFF) as f32;
    let flags = ((packed >> 24) & 0xFF) as u8;
    (r, b, s, flags)
}

// up to 4096 textures (12 bits), 20-bit generations
pub const SLOT_BITS: u32 = 12;
pub const SLOT_MASK: u32 = (1 << SLOT_BITS) - 1;

#[inline]
pub fn pack_slot_gen(slot: usize, generation: u32) -> u32 {
    (generation << SLOT_BITS) | ((slot + 1) as u32 & SLOT_MASK)
}
#[inline]
pub fn unpack_slot_gen(packed: u32) -> (usize, u32) {
    let slot_plus_one = packed & SLOT_MASK;
    let generation = packed >> SLOT_BITS;
    (slot_plus_one as usize - 1, generation)
}

#[cfg(test)]
mod tests {
    use super::*;

    // pack_unorm2x16 / pack_unorm4x8
    //
    // These are the GPU-format contracts used to pack atlas UV
    // offsets/scales and content-fit ratios into u32. A rounding bug
    // here silently corrupts UVs on screen — worth careful coverage.

    #[test]
    fn pack_unorm2x16_corners() {
        assert_eq!(pack_unorm2x16([0.0, 0.0]), 0);
        // 1.0 * 65535 + 0.5 -> 65535.5, floor -> 65535 = 0xFFFF.
        assert_eq!(pack_unorm2x16([1.0, 1.0]), 0xFFFF_FFFF);
        // x only: low 16 bits set.
        assert_eq!(pack_unorm2x16([1.0, 0.0]), 0x0000_FFFF);
        // y only: high 16 bits set.
        assert_eq!(pack_unorm2x16([0.0, 1.0]), 0xFFFF_0000);
    }

    #[test]
    fn pack_unorm2x16_halfway() {
        // 0.5 * 65535 + 0.5 = 32768.0, floor = 32768 = 0x8000.
        let p = pack_unorm2x16([0.5, 0.5]);
        assert_eq!(p & 0xFFFF, 0x8000);
        assert_eq!(p >> 16, 0x8000);
    }

    #[test]
    fn pack_unorm2x16_clamps_out_of_range_inputs() {
        // Negative values clamp to 0.
        assert_eq!(pack_unorm2x16([-1.0, -100.0]), 0);
        // Values above 1.0 clamp to 1.0 -> 0xFFFF each.
        assert_eq!(pack_unorm2x16([2.0, 1_000.0]), 0xFFFF_FFFF);
    }

    #[test]
    fn pack_unorm4x8_corners() {
        assert_eq!(pack_unorm4x8([0.0, 0.0, 0.0, 0.0]), 0);
        // 1.0 * 255 + 0.5 = 255.5, floor = 255.
        assert_eq!(pack_unorm4x8([1.0, 1.0, 1.0, 1.0]), 0xFFFF_FFFF);
    }

    #[test]
    fn pack_unorm4x8_each_channel_independent() {
        // Only low byte.
        assert_eq!(pack_unorm4x8([1.0, 0.0, 0.0, 0.0]), 0x0000_00FF);
        // Only second byte.
        assert_eq!(pack_unorm4x8([0.0, 1.0, 0.0, 0.0]), 0x0000_FF00);
        // Only third byte.
        assert_eq!(pack_unorm4x8([0.0, 0.0, 1.0, 0.0]), 0x00FF_0000);
        // Only high byte.
        assert_eq!(pack_unorm4x8([0.0, 0.0, 0.0, 1.0]), 0xFF00_0000);
    }

    #[test]
    fn pack_unorm4x8_clamps_out_of_range_inputs() {
        assert_eq!(pack_unorm4x8([-0.5, -1.0, -10.0, -100.0]), 0);
        assert_eq!(pack_unorm4x8([2.0, 3.0, 100.0, f32::INFINITY]), 0xFFFF_FFFF);
    }

    // pack_slot_gen / unpack_slot_gen
    //
    // The encoding stores (slot + 1) in the low SLOT_BITS and the
    // generation in the upper bits. Note the +1/-1 offset: slot 0 is
    // encoded as 1 in the low bits, which lets a fully-zero packed
    // value mean "no texture".

    #[test]
    fn slot_gen_roundtrip_slot_zero() {
        let packed = pack_slot_gen(0, 0);
        assert_eq!(
            packed, 1,
            "slot 0 + gen 0 should encode as 1 (zero means unset)"
        );
        let (slot, r#gen) = unpack_slot_gen(packed);
        assert_eq!((slot, r#gen), (0, 0));
    }

    #[test]
    fn slot_gen_roundtrip_named_slots() {
        for slot in [0, 1, 7, 255, 1000, SLOT_MASK as usize - 1] {
            for r#gen in [0u32, 1, 42, 1_000_000] {
                let packed = pack_slot_gen(slot, r#gen);
                let (s, g) = unpack_slot_gen(packed);
                assert_eq!(
                    (s, g),
                    (slot, r#gen),
                    "roundtrip failed for slot={slot} gen={gen}"
                );
            }
        }
    }

    #[test]
    fn slot_gen_layout_is_low_slot_bits_for_slot() {
        // Generation bits should not leak into the slot portion.
        let packed = pack_slot_gen(5, 0xABCD);
        assert_eq!(packed & SLOT_MASK, 5 + 1);
        assert_eq!(packed >> SLOT_BITS, 0xABCD);
    }

    #[test]
    fn slot_gen_max_slot_fits_in_slot_bits() {
        // SLOT_MASK - 1 is the largest slot index whose (slot + 1)
        // still fits in SLOT_BITS (because (SLOT_MASK - 1) + 1 == SLOT_MASK).
        let max_slot = (SLOT_MASK as usize) - 1;
        let packed = pack_slot_gen(max_slot, 0);
        let (s, _) = unpack_slot_gen(packed);
        assert_eq!(s, max_slot);
    }

    // shape_params

    #[test]
    fn pack_shape_params_roundtrip() {
        let packed = pack_shape_params(8.0, 1.5, 12.0, shape_flags::HAS_BORDER);
        let (r, b, s, f) = unpack_shape_params(packed);
        assert_eq!(r, 8.0);
        assert_eq!(b, 1.5);
        assert_eq!(s, 12.0);
        assert_eq!(f, shape_flags::HAS_BORDER);
    }

    #[test]
    fn pack_shape_params_zero_is_zero() {
        assert_eq!(pack_shape_params(0.0, 0.0, 0.0, 0), 0);
    }

    #[test]
    fn pack_shape_params_clamps_overflow() {
        let packed = pack_shape_params(300.0, 100.0, 999.0, 0xFF);
        let (r, b, s, f) = unpack_shape_params(packed);
        assert_eq!(r, 255.0);
        assert_eq!(b, 63.75); // 255 * 0.25
        assert_eq!(s, 255.0);
        assert_eq!(f, 0xFF);
    }

    #[test]
    fn pack_shape_params_quarter_pixel_precision() {
        // 0.25px border → stored as 1 in the byte
        let packed = pack_shape_params(0.0, 0.25, 0.0, 0);
        let (_, b, _, _) = unpack_shape_params(packed);
        assert_eq!(b, 0.25);

        // 2.75px border
        let packed = pack_shape_params(0.0, 2.75, 0.0, 0);
        let (_, b, _, _) = unpack_shape_params(packed);
        assert_eq!(b, 2.75);
    }
}
