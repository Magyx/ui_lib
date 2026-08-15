/// sRGB byte -> linear light in `[0, 1]`. Mirrors `srgb_to_linear` in
/// `ui_shader.wgsl`.
#[inline]
pub fn srgb_to_linear(c: u8) -> f32 {
    let f = c as f32 / 255.0;
    if f <= 0.04045 {
        f / 12.92
    } else {
        ((f + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear light in `[0, 1]` -> sRGB byte. Inverse of [`srgb_to_linear`].
#[inline]
pub fn linear_to_srgb(f: f32) -> u8 {
    let f = f.clamp(0.0, 1.0);
    let s = if f <= 0.0031308 {
        f * 12.92
    } else {
        1.055 * f.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Color(pub u32);

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const WHITE: Self = Self::rgba(255, 255, 255, 255);
    pub const BLACK: Self = Self::rgba(0, 0, 0, 255);
    pub const RED: Self = Self::rgba(255, 0, 0, 255);
    pub const GREEN: Self = Self::rgba(0, 255, 0, 255);
    pub const BLUE: Self = Self::rgba(0, 0, 255, 255);

    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 0xFF)
    }
    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self((r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24))
    }
    #[inline]
    pub const fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as u8;
        let g = ((hex >> 8) & 0xFF) as u8;
        let b = (hex & 0xFF) as u8;

        Self::rgb(r, g, b)
    }
    #[inline]
    pub const fn from_hexa(hex: u32) -> Self {
        let r = ((hex >> 24) & 0xFF) as u8;
        let g = ((hex >> 16) & 0xFF) as u8;
        let b = ((hex >> 8) & 0xFF) as u8;
        let a = (hex & 0xFF) as u8;

        Self::rgba(r, g, b, a)
    }

    #[inline]
    pub fn as_rgba_tuple(self) -> (u8, u8, u8, u8) {
        (self.r(), self.g(), self.b(), self.a())
    }

    #[inline]
    pub fn as_rgba(self) -> [u8; 4] {
        [self.r(), self.g(), self.b(), self.a()]
    }

    #[inline]
    pub fn r(&self) -> u8 {
        (self.0 & 0x00_00_00_FF) as u8
    }

    #[inline]
    pub fn g(&self) -> u8 {
        ((self.0 & 0x00_00_FF_00) >> 8) as u8
    }

    #[inline]
    pub fn b(&self) -> u8 {
        ((self.0 & 0x00_FF_00_00) >> 16) as u8
    }

    #[inline]
    pub fn a(&self) -> u8 {
        ((self.0 & 0xFF_00_00_00) >> 24) as u8
    }

    /// Blend from `self` toward `other` by `t` in `[0, 1]`, in **gamma
    /// space**. Alpha is kept from `self` (tonal steps shouldn't change
    /// opacity).
    ///
    /// Not physically correct, but usually what a designer means by "halfway
    /// between these two colours" — the same trade-off egui makes for
    /// `Color32`. Note this does **not** match a shader-side gradient
    /// (`with_gradient_end`), which interpolates in linear space; use
    /// [`Color::mix_linear`] to match that.
    #[inline]
    pub fn mix(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Color::rgba(
            lerp(self.r(), other.r()),
            lerp(self.g(), other.g()),
            lerp(self.b(), other.b()),
            self.a(),
        )
    }
    /// Nudge toward white by `t` in `[0, 1]`.
    #[inline]
    pub fn lighten(self, t: f32) -> Color {
        self.mix(Color::WHITE, t)
    }
    /// Nudge toward black by `t` in `[0, 1]`.
    #[inline]
    pub fn darken(self, t: f32) -> Color {
        self.mix(Color::BLACK, t)
    }
    /// Blend from `self` toward `other` by `t` in `[0, 1]` in **linear
    /// space**, i.e. physically correct light mixing.
    ///
    /// Matches what the shader does for gradients. Perceptually this front-
    /// loads the transition, so [`Color::mix`] is usually the better choice
    /// for tonal steps and hover states.
    #[inline]
    pub fn mix_linear(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| {
            let (a, b) = (srgb_to_linear(a), srgb_to_linear(b));
            linear_to_srgb(a + (b - a) * t)
        };
        Color::rgba(
            lerp(self.r(), other.r()),
            lerp(self.g(), other.g()),
            lerp(self.b(), other.b()),
            self.a(),
        )
    }

    /// Rec. 601 **luma** in `[0, 1]`, computed on gamma-encoded components.
    ///
    /// A perceptual-lightness heuristic — good for "is this surface dark or
    /// light?". This is luma (Y'), not luminance (Y): for a WCAG contrast
    /// ratio use [`Color::relative_luminance`] instead.
    #[inline]
    pub fn luma(self) -> f32 {
        (0.299 * self.r() as f32 + 0.587 * self.g() as f32 + 0.114 * self.b() as f32) / 255.0
    }

    /// WCAG relative luminance in `[0, 1]`: Rec. 709 weights on linearised
    /// components.
    ///
    /// Use for contrast ratios, which are defined as
    /// `(lighter + 0.05) / (darker + 0.05)`. For "does this look dark?"
    /// prefer [`Color::luma`] — linear luminance is deliberately not
    /// perceptual.
    #[inline]
    pub fn relative_luminance(self) -> f32 {
        0.2126 * srgb_to_linear(self.r())
            + 0.7152 * srgb_to_linear(self.g())
            + 0.0722 * srgb_to_linear(self.b())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_constants() {
        assert_eq!(Color::TRANSPARENT.as_rgba(), [0, 0, 0, 0]);
        assert_eq!(Color::WHITE.as_rgba(), [255, 255, 255, 255]);
        assert_eq!(Color::BLACK.as_rgba(), [0, 0, 0, 255]);
        assert_eq!(Color::RED.as_rgba(), [255, 0, 0, 255]);
        assert_eq!(Color::GREEN.as_rgba(), [0, 255, 0, 255]);
        assert_eq!(Color::BLUE.as_rgba(), [0, 0, 255, 255]);
    }

    #[test]
    fn color_rgb_defaults_alpha_to_ff() {
        let c = Color::rgb(10, 20, 30);
        assert_eq!(c.a(), 0xFF);
        assert_eq!(c.r(), 10);
        assert_eq!(c.g(), 20);
        assert_eq!(c.b(), 30);
    }

    #[test]
    fn color_rgba_channel_order_is_abgr_packed_u32() {
        // r | g<<8 | b<<16 | a<<24
        let c = Color::rgba(0x11, 0x22, 0x33, 0x44);
        assert_eq!(c.0, 0x44_33_22_11);
        assert_eq!(c.as_rgba_tuple(), (0x11, 0x22, 0x33, 0x44));
    }

    #[test]
    fn color_channel_extraction_at_boundaries() {
        let c = Color::rgba(0, 255, 0, 255);
        assert_eq!(c.r(), 0);
        assert_eq!(c.g(), 255);
        assert_eq!(c.b(), 0);
        assert_eq!(c.a(), 255);
    }
}

#[cfg(test)]
mod color_space_tests {
    use super::*;

    #[test]
    fn transfer_functions_round_trip() {
        for b in 0u8..=255 {
            assert_eq!(
                linear_to_srgb(srgb_to_linear(b)),
                b,
                "byte {b} did not survive"
            );
        }
    }

    /// 0x80 is mid grey *perceptually*; in linear light it is much darker.
    /// If this ever reads ~0.5 the decode has been dropped somewhere.
    #[test]
    fn mid_grey_is_not_half_linear() {
        let l = srgb_to_linear(0x80);
        assert!((l - 0.2159).abs() < 0.001, "expected ~0.216, got {l}");
    }

    #[test]
    fn endpoints_are_exact() {
        assert_eq!(srgb_to_linear(0), 0.0);
        assert_eq!(srgb_to_linear(255), 1.0);
        assert_eq!(linear_to_srgb(0.0), 0);
        assert_eq!(linear_to_srgb(1.0), 255);
    }

    /// The two mixes disagree by construction — that is the point of having
    /// both. Linear mixing of black and white lands lighter.
    #[test]
    fn gamma_and_linear_mix_differ() {
        let g = Color::BLACK.mix(Color::WHITE, 0.5);
        let l = Color::BLACK.mix_linear(Color::WHITE, 0.5);
        assert_eq!(g.r(), 128, "gamma mix is the arithmetic midpoint");
        assert!(
            l.r() > 180,
            "linear mix should be much lighter, got {}",
            l.r()
        );
    }

    #[test]
    fn mix_keeps_self_alpha() {
        let a = Color::rgba(0, 0, 0, 0x40);
        assert_eq!(
            a.mix_linear(Color::rgba(255, 255, 255, 0xFF), 0.5).a(),
            0x40
        );
    }

    /// Luma is computed on gamma bytes, relative luminance on linear values,
    /// so mid grey reads very differently through the two.
    #[test]
    fn luma_and_relative_luminance_differ() {
        let grey = Color::rgb(0x80, 0x80, 0x80);
        assert!((grey.luma() - 0.502).abs() < 0.01);
        assert!((grey.relative_luminance() - 0.216).abs() < 0.01);
    }

    #[test]
    fn white_and_black_agree_in_both_measures() {
        assert!((Color::WHITE.luma() - 1.0).abs() < 1e-6);
        assert!((Color::WHITE.relative_luminance() - 1.0).abs() < 1e-6);
        assert_eq!(Color::BLACK.luma(), 0.0);
        assert_eq!(Color::BLACK.relative_luminance(), 0.0);
    }
}
