use super::*;

use Length::{Fit, Fixed, Grow};

pub fn view(state: &State) -> Element<Message> {
    let t = &state.theme;

    let main = Column::new(vec![
        view_fixed_fixed(t),
        view_fixed_grow_fixed(t),
        view_multiple_grow(t),
        view_column_grow(t),
        view_fit_sizing(t),
        view_nested_grow(t),
        view_spacing_extremes(t),
        view_many_children(t),
        view_clamping(t),
        view_transparent_container(),
        view_grid(t),
    ])
    .padding(Vec4::splat(space::LG))
    .spacing(space::MD)
    .size(Size::new(Grow, Fit));

    Scrollable::new(main).size(Size::new(Grow, Grow)).into()
}

/// 1) Fixed + Fixed, zero padding baseline
fn view_fixed_fixed(t: &Theme) -> Element<Message> {
    Row::new(el![
        Rectangle::new(Size::new(Fixed(80), Fixed(40)), swatch(0)),
        Rectangle::new(Size::new(Fixed(120), Fixed(40)), swatch(5)),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(0))
    .color(t.surface_variant)
    .size(Size::new(Grow, Fixed(70)))
    .into()
}

/// 2) Fixed + Grow + Fixed; height fixed, width grow
fn view_fixed_grow_fixed(t: &Theme) -> Element<Message> {
    Row::new(el![
        Rectangle::new(Size::new(Fixed(60), Fixed(60)), swatch(2)),
        Rectangle::new(Size::new(Grow, Grow), swatch(4)),
        Rectangle::new(Size::new(Fixed(60), Fixed(60)), swatch(2)),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Grow, Fixed(80)))
    .into()
}

/// 3) Multiple Grow children in a Row (checks equalization)
fn view_multiple_grow(t: &Theme) -> Element<Message> {
    Row::new(el![
        Rectangle::new(Size::new(Grow, Fixed(50)), swatch(0)),
        Rectangle::new(Size::new(Grow, Fixed(50)), swatch(3)),
        Rectangle::new(Size::new(Grow, Fixed(50)), swatch(5)),
    ])
    .spacing(space::XS + 2)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Grow, Fixed(70)))
    .into()
}

/// 4) Column with Grow height distribution and fixed caps
fn view_column_grow(t: &Theme) -> Element<Message> {
    Column::new(el![
        Rectangle::new(Size::new(Grow, Fixed(20)), swatch(6)),
        Rectangle::new(Size::new(Grow, Grow), swatch(3)).min_y(20),
        Rectangle::new(Size::new(Grow, Fixed(20)), swatch(5)),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Grow, Fixed(120)))
    .into()
}

/// 5) Fit sizing demo: Column(Fit,Fit) measured by fixed children
fn view_fit_sizing(t: &Theme) -> Element<Message> {
    use Length::{Fit, Fixed, Grow};
    Row::new(el![
        Column::new(el![
            Rectangle::new(Size::new(Fixed(70), Fixed(20)), swatch(7)),
            Rectangle::new(Size::new(Fixed(40), Fixed(30)), swatch(1)),
        ])
        .spacing(space::XS)
        .padding(Vec4::splat(space::XS))
        .size(Size::new(Fit, Fit))
        .color(t.surface),
        Rectangle::new(Size::new(Grow, Fixed(60)), swatch(4)),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Grow, Fixed(80)))
    .into()
}

/// 6) Nested grow: Row of two Columns
fn view_nested_grow(t: &Theme) -> Element<Message> {
    Row::new(el![
        Column::new(el![
            Rectangle::new(Size::new(Grow, Fixed(18)), swatch(2)),
            Rectangle::new(Size::new(Grow, Grow), swatch(1)).min_y(20),
        ])
        .spacing(space::XS + 2)
        .padding(Vec4::splat(space::XS + 2))
        .size(Size::new(Fixed(200), Grow))
        .color(t.surface),
        Column::new(el![
            Rectangle::new(Size::new(Grow, Grow), swatch(4)).min_y(20),
            Rectangle::new(Size::new(Grow, Fixed(24)), swatch(5)),
        ])
        .spacing(space::XS + 2)
        .padding(Vec4::splat(space::XS + 2))
        .size(Size::new(Grow, Grow))
        .color(t.surface),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Grow, Fixed(100)))
    .into()
}

/// 7) Spacing extremes: zero vs nonzero, plus a Grow filler
fn view_spacing_extremes(t: &Theme) -> Element<Message> {
    let block = || {
        [
            Rectangle::new(Size::new(Fixed(40), Fixed(40)), swatch(6)),
            Rectangle::new(Size::new(Fixed(40), Fixed(40)), swatch(7)),
        ]
    };
    Row::new(el![
        Row::new(block())
            .spacing(0)
            .padding(Vec4::splat(0))
            .size(Size::new(Fixed(100), Fixed(40)))
            .color(t.surface),
        Row::new(block())
            .spacing(space::MD)
            .padding(Vec4::splat(0))
            .size(Size::new(Fixed(120), Fixed(40)))
            .color(t.surface),
        Rectangle::new(Size::new(Grow, Fixed(40)), swatch(4)),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Grow, Fixed(60)))
    .into()
}

/// 8) Many children + padding stress
fn view_many_children(t: &Theme) -> Element<Message> {
    Row::new((0..8).map(|i| small_block(swatch(i))).collect::<Vec<_>>())
        .spacing(space::XS + 2)
        .padding(Vec4::splat(space::LG))
        .color(t.surface_variant)
        .size(Size::new(Grow, Fixed(56)))
        .into()
}

/// 9) Test clamping (min/max)
fn view_clamping(t: &Theme) -> Element<Message> {
    Row::new(el![
        Rectangle::new(Size::new(Length::Grow, Length::Fixed(24)), swatch(3))
            .min(Size::new(120, 24)) // >= 120px wide
            .max(Size::new(300, 24)), // <= 300px wide
        Rectangle::new(Size::new(Length::Fixed(100), Length::Grow), swatch(5))
            .min(Size::new(100, 60)) // >= 60px tall
            .max(Size::new(100, 120)), // <= 120px tall
    ])
    .spacing(space::XS + 2)
    .padding(Vec4::splat(space::LG))
    .color(t.surface_variant)
    .size(Size::new(Length::Grow, Length::Grow))
    .into()
}

/// 10) Transparent container background
fn view_transparent_container() -> Element<Message> {
    Column::new(el![
        Rectangle::new(Size::new(Grow, Fixed(20)), swatch(3)),
        Rectangle::new(Size::new(Grow, Fixed(20)), swatch(5)),
    ])
    .spacing(space::XS + 2)
    .padding(Vec4::splat(space::SM))
    .color(Color::TRANSPARENT)
    .size(Size::new(Grow, Fixed(60)))
    .into()
}

/// 11) Grid layout example
fn view_grid(t: &Theme) -> Element<Message> {
    use std::num::NonZero;

    let cells: Vec<_> = (0..12).map(|i| small_block(swatch(i))).collect();

    WrappingRows::new(NonZero::new(4).unwrap(), cells)
        .col_spacing(space::SM)
        .row_spacing(space::SM)
        .padding(Vec4::splat(space::SM))
        .color(t.surface_variant)
        .size(Size::new(Grow, Fit))
        .into()
}
