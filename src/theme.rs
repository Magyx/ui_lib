use crate::{
    context::Env,
    layout::ROOT_SEED,
    model::Color,
    text::{FontStyle, Weight},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    pub weight: Weight,
    pub style: FontStyle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Typography {
    pub h1: TextStyle,
    pub h2: TextStyle,
    pub h3: TextStyle,
    pub body: TextStyle,
    pub label: TextStyle,
    pub caption: TextStyle,
}

impl Default for Typography {
    fn default() -> Self {
        let t = |font_size, line_height, weight, style| TextStyle {
            font_size,
            line_height,
            weight,
            style,
        };
        Self {
            h1: t(32.0, 1.20, Weight::BOLD, FontStyle::Normal),
            h2: t(24.0, 1.25, Weight::BOLD, FontStyle::Normal),
            h3: t(18.0, 1.30, Weight::SEMIBOLD, FontStyle::Normal),
            body: t(14.0, 1.20, Weight::NORMAL, FontStyle::Normal),
            label: t(13.0, 1.20, Weight::MEDIUM, FontStyle::Normal),
            caption: t(12.0, 1.30, Weight::NORMAL, FontStyle::Normal),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Style {
    pub fill: Option<Color>,
    pub border: Option<Color>,
    pub foreground: Option<Color>,
}

impl Style {
    /// Resolve the fill, falling back to `base` when not overridden.
    #[inline]
    pub fn fill_or(&self, base: Color) -> Color {
        self.fill.unwrap_or(base)
    }
    /// Resolve the border, falling back to `base` when not overridden.
    #[inline]
    pub fn border_or(&self, base: Color) -> Color {
        self.border.unwrap_or(base)
    }
    /// Resolve the foreground, falling back to `base` when not overridden.
    #[inline]
    pub fn foreground_or(&self, base: Color) -> Color {
        self.foreground.unwrap_or(base)
    }
}

// TODO: These should be fields in Theme.
pub const GAP: i32 = 2;
pub const RING_WIDTH: i32 = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    /// Default container surface (cards, panels, sidebars).
    pub surface: Color,
    /// Text and icons on `surface`.
    pub on_surface: Color,
    /// Elevated or variant surface (input fields, recessed areas).
    pub surface_variant: Color,
    /// Text and icons on `surface_variant`.
    pub on_surface_variant: Color,

    pub surface_container_lowest: Color,
    pub surface_container_low: Color,
    pub surface_container: Color,
    pub surface_container_high: Color,
    pub surface_container_highest: Color,

    /// Primary action color: filled buttons, active indicators, links.
    pub primary: Color,
    /// Text/icons on `primary`.
    pub on_primary: Color,
    /// Toned primary container: selected cards, active chips.
    pub primary_container: Color,
    /// Text/icons on `primary_container`.
    pub on_primary_container: Color,

    /// Secondary actions: filter chips, toggle buttons.
    pub secondary: Color,
    /// Text/icons on `secondary`.
    pub on_secondary: Color,
    /// Toned secondary container.
    pub secondary_container: Color,
    /// Text/icons on `secondary_container`.
    pub on_secondary_container: Color,

    /// Error/danger: destructive actions, validation errors.
    pub error: Color,
    /// Text/icons on `error`.
    pub on_error: Color,
    /// Low-emphasis error surface: validation banners, inline warnings.
    pub error_container: Color,
    /// Text/icons on `error_container`.
    pub on_error_container: Color,

    /// Default outline/border color (used when a widget opts into borders).
    pub outline: Color,
    /// Subtler variant for separators, dividers.
    pub outline_variant: Color,
    /// Border color for focused/active elements.
    pub focus_outline: Color,

    /// Default corner radius for all widgets (logical pixels).
    pub corner_radius: f32,
    /// Default border width when a widget opts into having a border.
    pub border_width: i32,

    /// Color shift multiplier (0..1) applied to interactive fills on hover
    /// based on luminance.
    pub hover_shift: f32,
    /// Color shift multiplier (0..1) applied to interactive fills on press
    /// based on luminance.
    pub pressed_shift: f32,

    /// Global typography styles for headers, body text, and labels.
    pub typography: Typography,
}

pub const TEXT_CONTRAST_MIN: f32 = 4.5;

pub fn contrast_ratio(a: Color, b: Color) -> f32 {
    let (x, y) = (a.relative_luminance(), b.relative_luminance());
    let (hi, lo) = if x > y { (x, y) } else { (y, x) };
    (hi + 0.05) / (lo + 0.05)
}

impl Theme {
    /// Surface color at a tonal `elevation`, from the container ramp.
    pub fn surface_at(&self, elevation: u8) -> Color {
        match elevation % 6 {
            0 => self.surface,
            1 => self.surface_container_low,
            2 => self.surface_container,
            3 => self.surface_container_high,
            4 => self.surface_container_highest,
            _ => self.surface_container_lowest,
        }
    }
    /// Readable foreground for [`surface_at(elevation)`](Self::surface_at):
    /// whichever of the theme's on-colors contrasts most with the elevated surface.
    pub fn on_surface_at(&self, elevation: u8) -> Color {
        let bg = self.surface_at(elevation);

        if contrast_ratio(self.on_surface, bg) >= TEXT_CONTRAST_MIN {
            return self.on_surface;
        }

        [self.on_surface, Color::BLACK, Color::WHITE]
            .into_iter()
            .max_by(|a, b| contrast_ratio(*a, bg).total_cmp(&contrast_ratio(*b, bg)))
            .unwrap_or(self.on_surface)
    }
    /// Fill shade for a hovered interactive surface. (intensity = `hover_shift`).
    pub fn hovered(&self, base: Color) -> Color {
        self.interact(base, self.hover_shift)
    }
    /// Fill shade for a pressed interactive surface (intensity = `pressed_shift`).
    pub fn pressed(&self, base: Color) -> Color {
        self.interact(base, self.pressed_shift)
    }
    fn interact(&self, base: Color, t: f32) -> Color {
        if base.luma() < 0.5 {
            base.lighten(t)
        } else {
            base.darken(t)
        }
    }

    /// The ambient [`Env`] at the root of the tree: elevation 0, foreground on
    /// the page background, and the body text style.
    pub(crate) fn root_env(&self) -> Env {
        Env {
            elevation: 0,
            foreground: self.on_surface,
            text: self.typography.body,
            focus_scope: ROOT_SEED,
        }
    }

    /// Dark theme.
    pub fn dark() -> Self {
        Self {
            surface: Color::from_hex(0x111418),
            on_surface: Color::from_hex(0xE1E2E8),
            surface_variant: Color::from_hex(0x43474E),
            on_surface_variant: Color::from_hex(0xC3C6CF),

            surface_container_lowest: Color::from_hex(0x0B0E13),
            surface_container_low: Color::from_hex(0x191C20),
            surface_container: Color::from_hex(0x1D2024),
            surface_container_high: Color::from_hex(0x272A2F),
            surface_container_highest: Color::from_hex(0x32353A),

            primary: Color::from_hex(0xA2C9FE),
            on_primary: Color::from_hex(0x00325B),
            primary_container: Color::from_hex(0x1D4875),
            on_primary_container: Color::from_hex(0xD3E4FF),

            secondary: Color::from_hex(0xBBC7DB),
            on_secondary: Color::from_hex(0x263141),
            secondary_container: Color::from_hex(0x3C4858),
            on_secondary_container: Color::from_hex(0xD7E3F8),

            error: Color::from_hex(0xFFB4AB),
            on_error: Color::from_hex(0x690005),
            error_container: Color::from_hex(0x93000A),
            on_error_container: Color::from_hex(0xFFDAD6),

            outline: Color::from_hex(0x8D9199),
            outline_variant: Color::from_hex(0x43474E),
            focus_outline: Color::from_hex(0xA2C9FE),

            corner_radius: 0.0,
            border_width: 1,

            hover_shift: 0.08,
            pressed_shift: 0.16,

            typography: Typography::default(),
        }
    }

    /// Light theme.
    pub fn light() -> Self {
        Self {
            surface: Color::from_hex(0xF8F9FF),
            on_surface: Color::from_hex(0x191C20),
            surface_variant: Color::from_hex(0xDFE2EB),
            on_surface_variant: Color::from_hex(0x43474E),

            surface_container_lowest: Color::from_hex(0xFFFFFF),
            surface_container_low: Color::from_hex(0xF2F3FA),
            surface_container: Color::from_hex(0xECEDF4),
            surface_container_high: Color::from_hex(0xE7E8EE),
            surface_container_highest: Color::from_hex(0xE1E2E8),

            primary: Color::from_hex(0x38608F),
            on_primary: Color::from_hex(0xFFFFFF),
            primary_container: Color::from_hex(0xD3E4FF),
            on_primary_container: Color::from_hex(0x001C38),

            secondary: Color::from_hex(0x545F70),
            on_secondary: Color::from_hex(0xFFFFFF),
            secondary_container: Color::from_hex(0xD7E3F8),
            on_secondary_container: Color::from_hex(0x101C2B),

            error: Color::from_hex(0xBA1A1A),
            on_error: Color::from_hex(0xFFFFFF),
            error_container: Color::from_hex(0xFFDAD6),
            on_error_container: Color::from_hex(0x410002),

            outline: Color::from_hex(0x73777F),
            outline_variant: Color::from_hex(0xC3C6CF),
            focus_outline: Color::from_hex(0x38608F),

            corner_radius: 0.0,
            border_width: 1,

            hover_shift: 0.08,
            pressed_shift: 0.16,

            typography: Typography::default(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

macro_rules! with_color_property {
    ($func_name:ident, $field_name:ident) => {
        with_property!($func_name, $field_name, Option<Color>);
    };
}

macro_rules! with_property {
    ($func_name:ident, $field_name:ident, $field_type:ty) => {
        #[inline]
        pub fn $func_name(self, over: $field_type) -> Self {
            match over {
                Some(o) => Self {
                    $field_name: o,
                    ..self
                },
                None => self,
            }
        }
    };
}

impl Theme {
    with_color_property!(with_surface, surface);
    with_color_property!(with_on_surface, on_surface);
    with_color_property!(with_surface_variant, surface_variant);
    with_color_property!(with_surface_container_lowest, surface_container_lowest);
    with_color_property!(with_surface_container_low, surface_container_low);
    with_color_property!(with_surface_container, surface_container);
    with_color_property!(with_surface_container_high, surface_container_high);
    with_color_property!(with_surface_container_highest, surface_container_highest);
    with_color_property!(with_primary, primary);
    with_color_property!(with_on_primary, on_primary);
    with_color_property!(with_secondary, secondary);
    with_color_property!(with_secondary_container, secondary_container);
    with_color_property!(with_error, error);
    with_color_property!(with_error_container, error_container);
    with_color_property!(with_outline, outline);
    with_color_property!(with_outline_variant, outline_variant);
    with_color_property!(with_focus_outline, focus_outline);
    with_property!(with_corner_radius, corner_radius, Option<f32>);
    with_property!(with_border_width, border_width, Option<i32>);
    with_property!(with_typography, typography, Option<Typography>);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_is_copy() {
        let a = Theme::dark();
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn with_chain_overrides_only_specified_fields() {
        let base = Theme::dark();
        let modified = base.with_corner_radius(Some(8.0)).with_surface(None);

        assert_eq!(modified.corner_radius, 8.0);
        assert_eq!(
            modified.surface, base.surface,
            "None should not change the value"
        );
    }

    #[test]
    fn with_chain_all_none_is_identity() {
        let base = Theme::dark();
        let same = base
            .with_surface(None)
            .with_primary(None)
            .with_corner_radius(None)
            .with_border_width(None);
        assert_eq!(same, base);
    }

    const DIVIDER_MIN: f32 = 1.5;
    const FOCUS_MIN: f32 = 3.0;
    const STEP_MIN: f32 = 1.03;
    const RAMP_PERIOD: u8 = 5;

    fn check(name: &str, t: &Theme) {
        for (label, fg, bg) in [
            ("on_surface/surface", t.on_surface, t.surface),
            (
                "on_surface_variant/surface_variant",
                t.on_surface_variant,
                t.surface_variant,
            ),
            ("on_primary/primary", t.on_primary, t.primary),
            (
                "on_primary_container/primary_container",
                t.on_primary_container,
                t.primary_container,
            ),
            ("on_secondary/secondary", t.on_secondary, t.secondary),
            (
                "on_secondary_container/secondary_container",
                t.on_secondary_container,
                t.secondary_container,
            ),
            ("on_error/error", t.on_error, t.error),
            (
                "on_error_container/error_container",
                t.on_error_container,
                t.error_container,
            ),
        ] {
            let r = contrast_ratio(fg, bg);
            assert!(
                r >= TEXT_CONTRAST_MIN,
                "{name}: {label} is {r:.2}, need {TEXT_CONTRAST_MIN}"
            );
        }

        for (label, c) in [
            ("outline", t.outline),
            ("outline_variant", t.outline_variant),
        ] {
            let r = contrast_ratio(c, t.surface);
            assert!(
                r >= DIVIDER_MIN,
                "{name}: {label} is {r:.2} against surface, need {DIVIDER_MIN}"
            );
        }

        let r = contrast_ratio(t.focus_outline, t.surface);
        assert!(
            r >= FOCUS_MIN,
            "{name}: focus_outline is {r:.2} against bg, need {FOCUS_MIN}"
        );
    }

    #[test]
    fn dark_theme_meets_contrast_targets() {
        check("dark", &Theme::dark());
    }

    #[test]
    fn light_theme_meets_contrast_targets() {
        check("light", &Theme::light());
    }

    // elevation ramp

    /// The property that makes cycling safe: every level separates from the
    /// one below, *including across the wrap*. Saturating failed exactly here.
    #[test]
    fn every_adjacent_elevation_separates() {
        for (name, t) in [("dark", Theme::dark()), ("light", Theme::light())] {
            // Two full turns, so the wrap is exercised more than once.
            for level in 1..=(RAMP_PERIOD * 2 + 1) {
                let r = contrast_ratio(t.surface_at(level - 1), t.surface_at(level));
                assert!(
                    r >= STEP_MIN,
                    "{name}: elevation {level} is {r:.3} against {}, levels have merged",
                    level - 1
                );
            }
        }
    }

    /// Every tone the cycle claims to use is reachable. A defined token that no
    /// elevation produces is dead weight — this caught `surface_container_highest`
    /// being stranded by an off-by-one in the ramp order.
    #[test]
    fn the_cycle_reaches_every_tone_it_claims() {
        let t = Theme::dark();
        let seen: Vec<Color> = (0..RAMP_PERIOD).map(|l| t.surface_at(l)).collect();
        for (name, c) in [
            ("surface", t.surface),
            ("surface_container_low", t.surface_container_low),
            ("surface_container", t.surface_container),
            ("surface_container_high", t.surface_container_high),
            ("surface_container_highest", t.surface_container_highest),
        ] {
            assert!(seen.contains(&c), "{name} is never produced by surface_at");
        }
    }

    // foreground selection

    /// An overridden ramp must still get a readable foreground.
    #[test]
    fn foreground_falls_back_when_the_ramp_is_overridden() {
        // Elevation 3 maps to `surface_container_high`.
        let t = Theme::dark().with_surface_container_high(Some(Color::WHITE));
        let bg = t.surface_at(3);
        assert_eq!(bg, Color::WHITE, "the override never reached the ramp");

        let fg = t.on_surface_at(3);
        let r = contrast_ratio(fg, bg);
        assert!(
            r >= TEXT_CONTRAST_MIN,
            "a white container kept a light foreground ({fg:?}, {r:.2})"
        );
    }

    #[test]
    fn light_theme_has_distinct_values() {
        let dark = Theme::dark();
        let light = Theme::light();
        assert_ne!(dark.surface, light.surface);
    }

    #[test]
    fn default_is_dark() {
        assert_eq!(Theme::default(), Theme::dark());
    }

    #[test]
    fn border_width_default_is_one() {
        assert_eq!(Theme::dark().border_width, 1);
        assert_eq!(Theme::light().border_width, 1);
    }
}
