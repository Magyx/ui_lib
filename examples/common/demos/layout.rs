use super::*;

use Length::{Fit, Fixed, Grow};

pub fn view(_state: &State) -> Element<Message> {
    let main = Column::new(vec![
        view_fixed_fixed(),
        view_fixed_grow_fixed(),
        view_multiple_grow(),
        view_column_grow(),
        view_fit_sizing(),
        view_nested_grow(),
        view_spacing_extremes(),
        view_many_children(),
        view_clamping(),
        view_transparent_container(),
        view_grid(),
    ])
    .padding(Vec4::splat(16))
    .spacing(14)
    .size(Size::new(Grow, Fit));

    Scrollable::new(main)
        .size(Size::new(Grow, Grow))
        .bg(Color::rgb(100, 80, 100))
        .into()
}

/// 1) Fixed + Fixed, zero padding baseline
fn view_fixed_fixed() -> Element<Message> {
    Row::new(el![
        Rectangle::new(Size::new(Fixed(80), Fixed(40)), Color::RED),
        Rectangle::new(Size::new(Fixed(120), Fixed(40)), Color::GREEN),
    ])
    .spacing(8)
    .padding(Vec4::splat(0))
    .color(Color::rgb(240, 240, 240))
    .size(Size::new(Grow, Fixed(70)))
    .into()
}

/// 2) Fixed + Grow + Fixed; height fixed, width grow
fn view_fixed_grow_fixed() -> Element<Message> {
    Row::new(el![
        Rectangle::new(Size::new(Fixed(60), Fixed(60)), Color::rgb(255, 200, 0)),
        Rectangle::new(Size::new(Grow, Grow), Color::rgb(0, 180, 180)),
        Rectangle::new(Size::new(Fixed(60), Fixed(60)), Color::rgb(255, 200, 0)),
    ])
    .spacing(10)
    .padding(Vec4::splat(10))
    .color(Color::rgb(240, 240, 240))
    .size(Size::new(Grow, Fixed(80)))
    .into()
}

/// 3) Multiple Grow children in a Row (checks equalization)
fn view_multiple_grow() -> Element<Message> {
    Row::new(el![
        Rectangle::new(Size::new(Grow, Fixed(50)), Color::rgb(200, 50, 50)),
        Rectangle::new(Size::new(Grow, Fixed(50)), Color::rgb(50, 200, 50)),
        Rectangle::new(Size::new(Grow, Fixed(50)), Color::rgb(50, 50, 200)),
    ])
    .spacing(6)
    .padding(Vec4::splat(10))
    .color(Color::rgb(240, 240, 240))
    .size(Size::new(Grow, Fixed(70)))
    .into()
}

/// 4) Column with Grow height distribution and fixed caps
fn view_column_grow() -> Element<Message> {
    Column::new(el![
        Rectangle::new(Size::new(Grow, Fixed(20)), Color::rgb(80, 80, 80)),
        Rectangle::new(Size::new(Grow, Grow), Color::rgb(100, 200, 100)).min_y(20),
        Rectangle::new(Size::new(Grow, Fixed(20)), Color::rgb(80, 80, 150)),
    ])
    .spacing(8)
    .padding(Vec4::splat(10))
    .color(Color::rgb(240, 240, 240))
    .size(Size::new(Grow, Fixed(120)))
    .into()
}

/// 5) Fit sizing demo: Column(Fit,Fit) measured by fixed children
fn view_fit_sizing() -> Element<Message> {
    use Length::{Fit, Fixed, Grow};
    Row::new(el![
        Column::new(el![
            Rectangle::new(Size::new(Fixed(70), Fixed(20)), Color::rgb(100, 0, 100)),
            Rectangle::new(Size::new(Fixed(40), Fixed(30)), Color::rgb(140, 0, 140)),
        ])
        .spacing(4)
        .padding(Vec4::splat(4))
        .size(Size::new(Fit, Fit))
        .color(Color::rgb(230, 200, 230)),
        Rectangle::new(Size::new(Grow, Fixed(60)), Color::rgb(180, 180, 180)),
    ])
    .spacing(10)
    .padding(Vec4::splat(10))
    .color(Color::rgb(240, 240, 240))
    .size(Size::new(Grow, Fixed(80)))
    .into()
}

/// 6) Nested grow: Row of two Columns
fn view_nested_grow() -> Element<Message> {
    Row::new(el![
        Column::new(el![
            Rectangle::new(Size::new(Grow, Fixed(18)), Color::rgb(160, 160, 0)),
            Rectangle::new(Size::new(Grow, Grow), Color::rgb(160, 100, 0)).min_y(20),
        ])
        .spacing(6)
        .padding(Vec4::splat(6))
        .size(Size::new(Fixed(200), Grow))
        .color(Color::rgb(250, 240, 200)),
        Column::new(el![
            Rectangle::new(Size::new(Grow, Grow), Color::rgb(0, 120, 160)).min_y(20),
            Rectangle::new(Size::new(Grow, Fixed(24)), Color::rgb(0, 80, 120)),
        ])
        .spacing(6)
        .padding(Vec4::splat(6))
        .size(Size::new(Grow, Grow))
        .color(Color::rgb(200, 240, 250)),
    ])
    .spacing(10)
    .padding(Vec4::splat(10))
    .color(Color::rgb(240, 240, 240))
    .size(Size::new(Grow, Fixed(100)))
    .into()
}

/// 7) Spacing extremes: zero vs nonzero, plus a Grow filler
fn view_spacing_extremes() -> Element<Message> {
    Row::new(el![
        Row::new(el![
            Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::rgb(0, 0, 0)),
            Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::rgb(80, 80, 80))
        ])
        .spacing(0)
        .padding(Vec4::splat(0))
        .size(Size::new(Fixed(100), Fixed(40)))
        .color(Color::rgb(220, 220, 220)),
        Row::new(el![
            Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::rgb(0, 0, 0)),
            Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::rgb(80, 80, 80))
        ])
        .spacing(12)
        .padding(Vec4::splat(0))
        .size(Size::new(Fixed(120), Fixed(40)))
        .color(Color::rgb(220, 220, 220)),
        Rectangle::new(Size::new(Grow, Fixed(40)), Color::rgb(200, 200, 200)),
    ])
    .spacing(10)
    .padding(Vec4::splat(10))
    .color(Color::rgb(240, 240, 240))
    .size(Size::new(Grow, Fixed(60)))
    .into()
}

/// 8) Many children + padding stress
fn view_many_children() -> Element<Message> {
    Row::new(
        (0..8)
            .map(|i| {
                let c = (i * 30 + 40) as u8;
                small_block(c, 30, 200u8.saturating_sub(c))
            })
            .collect::<Vec<_>>(),
    )
    .spacing(6)
    .padding(Vec4::splat(16))
    .color(Color::rgb(240, 240, 240))
    .size(Size::new(Grow, Fixed(56)))
    .into()
}

/// 9) Test clamping (min/max)
fn view_clamping() -> Element<Message> {
    Row::new(el![
        Rectangle::new(Size::new(Length::Grow, Length::Fixed(24)), Color::GREEN)
            .min(Size::new(120, 24)) // >= 120px wide
            .max(Size::new(300, 24)), // <= 300px wide
        Rectangle::new(Size::new(Length::Fixed(100), Length::Grow), Color::BLUE)
            .min(Size::new(100, 60)) // >= 60px tall
            .max(Size::new(100, 120)), // <= 120px tall
    ])
    .spacing(6)
    .padding(Vec4::splat(16))
    .color(Color::rgb(240, 240, 240))
    .size(Size::new(Length::Grow, Length::Grow))
    .into()
}

/// 10) Transparent container background
fn view_transparent_container() -> Element<Message> {
    Column::new(el![
        Rectangle::new(Size::new(Grow, Fixed(20)), Color::rgb(30, 200, 30)),
        Rectangle::new(Size::new(Grow, Fixed(20)), Color::rgb(30, 30, 200)),
    ])
    .spacing(6)
    .padding(Vec4::splat(10))
    .color(Color::TRANSPARENT)
    .size(Size::new(Grow, Fixed(60)))
    .into()
}

/// 11) Grid layout example
fn view_grid() -> Element<Message> {
    use std::num::NonZero;

    let cells: Vec<_> = (0..12)
        .map(|i| {
            let r = 60 + (i * 13) as u8;
            let g = 180u8.saturating_sub((i * 11) as u8);
            let b = 80 + (i * 7) as u8;
            small_block(r, g, b)
        })
        .collect();

    Grid::new(NonZero::new(4).unwrap(), cells)
        .col_spacing(10)
        .row_spacing(10)
        .padding(Vec4::splat(10))
        .color(Color::rgb(240, 240, 240))
        .size(Size::new(Grow, Fit))
        .into()
}
