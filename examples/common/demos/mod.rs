use super::{Message, State};

use ui::prelude::*;

/// Spacing scale, in logical pixels.
#[allow(dead_code)]
pub mod space {
    pub const XS: i32 = 4;
    pub const SM: i32 = 8;
    pub const MD: i32 = 12;
    pub const LG: i32 = 16;
    pub const XL: i32 = 24;
}

/// Common control dimensions, in logical pixels.
#[allow(dead_code)]
pub mod size {
    pub const CONTROL_H: i32 = 36;
    pub const SLIDER_H: i32 = 28;
    pub const ROW_H: i32 = 36;
    pub const HEADER_H: i32 = 56;
    pub const BLOCK: i32 = 24;
}

/// A fixed, categorical palette of distinct fills.
pub mod palette {
    use ui::model::Color;

    pub const COLORS: [Color; 8] = [
        Color::rgb(231, 76, 60),  // red
        Color::rgb(230, 126, 34), // orange
        Color::rgb(241, 196, 15), // yellow
        Color::rgb(46, 204, 113), // green
        Color::rgb(26, 188, 156), // teal
        Color::rgb(52, 152, 219), // blue
        Color::rgb(91, 105, 224), // indigo
        Color::rgb(155, 89, 182), // purple
    ];
}

/// Pick a marker color by index (wraps around the palette).
pub fn swatch(i: usize) -> Color {
    palette::COLORS[i % palette::COLORS.len()]
}

/// Apply a translucent alpha to a color (handy for overlays/scrim).
pub fn with_alpha(c: Color, a: u8) -> Color {
    Color::rgba(c.r(), c.g(), c.b(), a)
}

fn small_block(color: Color) -> Element<Message> {
    Rectangle::new(
        Size::new(Length::Fixed(size::BLOCK), Length::Fixed(size::BLOCK)),
        color,
    )
    .into()
}

pub mod interaction;
pub mod layout;
pub mod pipeline;
pub mod scrollable;
pub mod text;
pub mod texture;
pub mod theme_editor;
