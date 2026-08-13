use crate::{
    focus::ScopeId,
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
pub struct Env {
    /// Tonal elevation for surface-color resolution.
    pub elevation: u8,
    /// Inherited foreground (text/icon) color; widgets resolve their default
    /// content color from this instead of hardcoding `theme.on_surface`.
    pub foreground: Color,
    /// Inherited default text style.
    pub text: TextStyle,
    /// Focus scope the subtree belongs to. Root subtree is
    /// [`ROOT_SEED`](crate::layout::ROOT_SEED); a trapping container (a modal
    /// overlay) rebinds this to its own id for its descendants.
    pub focus_scope: ScopeId,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    /// Root window / page background. The engine auto-fills this.
    pub bg: Color,
    /// Text and icons on `bg`.
    pub on_bg: Color,

    /// Default container surface (cards, panels, sidebars).
    pub surface: Color,
    /// Text and icons on `surface`.
    pub on_surface: Color,
    /// Elevated or variant surface (input fields, recessed areas).
    pub surface_variant: Color,
    /// Text and icons on `surface_variant`.
    pub on_surface_variant: Color,

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

    /// Error/danger: destructive actions, validation errors.
    pub error: Color,
    /// Text/icons on `error`.
    pub on_error: Color,

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

impl Theme {
    /// Surface color at a tonal `elevation`. Level 0 is the base `surface`;
    /// each level nudges the surface toward its foreground — lighter on dark
    /// themes, darker on light ones — so nested containers keep separating
    /// instead of capping at the old `surface`/`surface_variant` pair.
    pub fn surface_at(&self, elevation: u8) -> Color {
        if elevation == 0 {
            return self.surface;
        }
        // ~6% per level, capped so deep nesting doesn't wash out.
        let t = (elevation as f32 * 0.06).min(0.6);
        if self.surface.luma() < 0.5 {
            self.surface.lighten(t)
        } else {
            self.surface.darken(t)
        }
    }
    /// Readable foreground for [`surface_at(elevation)`](Self::surface_at):
    /// whichever of the theme's on-colors (falling back to black/white)
    /// contrasts most with the elevated surface.
    pub fn on_surface_at(&self, elevation: u8) -> Color {
        let s = self.surface_at(elevation).luma();
        [self.on_surface, self.on_bg, Color::BLACK, Color::WHITE]
            .into_iter()
            .max_by(|a, b| {
                (a.luma() - s)
                    .abs()
                    .total_cmp(&(b.luma() - s).abs())
            })
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
            bg: Color::rgb(24, 24, 30),
            on_bg: Color::rgb(230, 230, 240),

            surface: Color::rgb(40, 40, 48),
            on_surface: Color::rgb(220, 220, 230),
            surface_variant: Color::rgb(55, 55, 65),
            on_surface_variant: Color::rgb(180, 180, 195),

            primary: Color::rgb(45, 150, 245),
            on_primary: Color::rgb(255, 255, 255),
            primary_container: Color::rgb(30, 60, 100),
            on_primary_container: Color::rgb(180, 215, 255),

            secondary: Color::rgb(140, 140, 160),
            on_secondary: Color::rgb(255, 255, 255),

            error: Color::rgb(220, 60, 60),
            on_error: Color::rgb(255, 255, 255),

            outline: Color::rgb(70, 70, 85),
            outline_variant: Color::rgb(50, 50, 60),
            focus_outline: Color::rgb(90, 130, 220),

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
            bg: Color::rgb(224, 228, 235),
            on_bg: Color::rgb(26, 30, 39),

            surface: Color::rgb(249, 250, 253),
            on_surface: Color::rgb(28, 32, 42),
            surface_variant: Color::rgb(220, 224, 232),
            on_surface_variant: Color::rgb(90, 97, 112),

            primary: Color::rgb(40, 112, 222),
            on_primary: Color::rgb(255, 255, 255),
            primary_container: Color::rgb(210, 227, 250),
            on_primary_container: Color::rgb(10, 48, 108),

            secondary: Color::rgb(94, 103, 126),
            on_secondary: Color::rgb(255, 255, 255),

            error: Color::rgb(201, 58, 58),
            on_error: Color::rgb(255, 255, 255),

            outline: Color::rgb(196, 202, 213),
            outline_variant: Color::rgb(218, 223, 231),
            focus_outline: Color::rgb(58, 120, 230),

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
    with_color_property!(with_bg, bg);
    with_color_property!(with_on_bg, on_bg);
    with_color_property!(with_surface, surface);
    with_color_property!(with_on_surface, on_surface);
    with_color_property!(with_surface_variant, surface_variant);
    with_color_property!(with_primary, primary);
    with_color_property!(with_on_primary, on_primary);
    with_color_property!(with_secondary, secondary);
    with_color_property!(with_outline, outline);
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
        let modified = base
            .with_bg(Some(Color::RED))
            .with_corner_radius(Some(8.0))
            .with_surface(None);

        assert_eq!(modified.bg, Color::RED);
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
            .with_bg(None)
            .with_surface(None)
            .with_primary(None)
            .with_corner_radius(None)
            .with_border_width(None);
        assert_eq!(same, base);
    }

    #[test]
    fn light_theme_has_distinct_values() {
        let dark = Theme::dark();
        let light = Theme::light();
        assert_ne!(dark.bg, light.bg);
        assert_ne!(dark.surface, light.surface);
        assert_ne!(dark.on_bg, light.on_bg);
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
