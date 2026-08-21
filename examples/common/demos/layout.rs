use super::*;

use Length::{Fill, Fit, Fixed};

pub fn view(state: &State) -> Element {
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
        view_main_alignment(t),
        view_cross_alignment(t),
        view_center_helper(t),
    ])
    .padding(Vec4::splat(space::LG))
    .spacing(space::MD)
    .size(Size::new(Fill(1.0), Fit));

    Scrollable::new(main)
        .size(Size::new(Fill(1.0), Fill(1.0)))
        .into()
}

/// 1) Fixed + Fixed, zero padding baseline
fn view_fixed_fixed(t: &Theme) -> Element {
    Row::new(el![
        Rectangle::new(Size::new(Fixed(80), Fixed(40)), swatch(0)),
        Rectangle::new(Size::new(Fixed(120), Fixed(40)), swatch(5)),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(0))
    .color(t.surface_variant)
    .size(Size::new(Fill(1.0), Fixed(70)))
    .into()
}

/// 2) Fixed + Fill(1.0) + Fixed; height fixed, width grow
fn view_fixed_grow_fixed(t: &Theme) -> Element {
    Row::new(el![
        Rectangle::new(Size::new(Fixed(60), Fixed(60)), swatch(2)),
        Rectangle::new(Size::new(Fill(1.0), Fill(1.0)), swatch(4)),
        Rectangle::new(Size::new(Fixed(60), Fixed(60)), swatch(2)),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Fill(1.0), Fixed(80)))
    .into()
}

/// 3) Multiple Fill(1.0) children in a Row (checks equalization)
fn view_multiple_grow(t: &Theme) -> Element {
    Row::new(el![
        Rectangle::new(Size::new(Fill(1.0), Fixed(50)), swatch(0)),
        Rectangle::new(Size::new(Fill(1.0), Fixed(50)), swatch(3)),
        Rectangle::new(Size::new(Fill(1.0), Fixed(50)), swatch(5)),
    ])
    .spacing(space::XS + 2)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Fill(1.0), Fixed(70)))
    .into()
}

/// 4) Column with Fill(1.0) height distribution and fixed caps
fn view_column_grow(t: &Theme) -> Element {
    Column::new(el![
        Rectangle::new(Size::new(Fill(1.0), Fixed(20)), swatch(6)),
        Rectangle::new(Size::new(Fill(1.0), Fill(1.0)), swatch(3)).min_y(20),
        Rectangle::new(Size::new(Fill(1.0), Fixed(20)), swatch(5)),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Fill(1.0), Fixed(120)))
    .into()
}

/// 5) Fit sizing demo: Column(Fit,Fit) measured by fixed children
fn view_fit_sizing(t: &Theme) -> Element {
    use Length::{Fit, Fixed};
    Row::new(el![
        Column::new(el![
            Rectangle::new(Size::new(Fixed(70), Fixed(20)), swatch(7)),
            Rectangle::new(Size::new(Fixed(40), Fixed(30)), swatch(1)),
        ])
        .spacing(space::XS)
        .padding(Vec4::splat(space::XS))
        .size(Size::new(Fit, Fit))
        .color(t.surface),
        Rectangle::new(Size::new(Fill(1.0), Fixed(60)), swatch(4)),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Fill(1.0), Fixed(80)))
    .into()
}

/// 6) Nested grow: Row of two Columns
fn view_nested_grow(t: &Theme) -> Element {
    Row::new(el![
        Column::new(el![
            Rectangle::new(Size::new(Fill(1.0), Fixed(18)), swatch(2)),
            Rectangle::new(Size::new(Fill(1.0), Fill(1.0)), swatch(1)).min_y(20),
        ])
        .spacing(space::XS + 2)
        .padding(Vec4::splat(space::XS + 2))
        .size(Size::new(Fixed(200), Fill(1.0)))
        .color(t.surface),
        Column::new(el![
            Rectangle::new(Size::new(Fill(1.0), Fill(1.0)), swatch(4)).min_y(20),
            Rectangle::new(Size::new(Fill(1.0), Fixed(24)), swatch(5)),
        ])
        .spacing(space::XS + 2)
        .padding(Vec4::splat(space::XS + 2))
        .size(Size::new(Fill(1.0), Fill(1.0)))
        .color(t.surface),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Fill(1.0), Fixed(100)))
    .into()
}

/// 7) Spacing extremes: zero vs nonzero, plus a Fill(1.0) filler
fn view_spacing_extremes(t: &Theme) -> Element {
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
        Rectangle::new(Size::new(Fill(1.0), Fixed(40)), swatch(4)),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Fill(1.0), Fixed(60)))
    .into()
}

/// 8) Many children + padding stress
fn view_many_children(t: &Theme) -> Element {
    Row::new((0..8).map(|i| small_block(swatch(i))).collect::<Vec<_>>())
        .spacing(space::XS + 2)
        .padding(Vec4::splat(space::LG))
        .color(t.surface_variant)
        .size(Size::new(Fill(1.0), Fixed(56)))
        .into()
}

/// 9) Test clamping (min/max)
fn view_clamping(t: &Theme) -> Element {
    Row::new(el![
        Rectangle::new(Size::new(Length::Fill(1.0), Length::Fixed(24)), swatch(3))
            .min(Size::new(120, 24)) // >= 120px wide
            .max(Size::new(300, 24)), // <= 300px wide
        Rectangle::new(Size::new(Length::Fixed(100), Length::Fill(1.0)), swatch(5))
            .min(Size::new(100, 60)) // >= 60px tall
            .max(Size::new(100, 120)), // <= 120px tall
    ])
    .spacing(space::XS + 2)
    .padding(Vec4::splat(space::LG))
    .color(t.surface_variant)
    .size(Size::new(Length::Fill(1.0), Length::Fill(1.0)))
    .into()
}

/// 10) Transparent container background
fn view_transparent_container() -> Element {
    Column::new(el![
        Rectangle::new(Size::new(Fill(1.0), Fixed(20)), swatch(3)),
        Rectangle::new(Size::new(Fill(1.0), Fixed(20)), swatch(5)),
    ])
    .spacing(space::XS + 2)
    .padding(Vec4::splat(space::SM))
    .color(Color::TRANSPARENT)
    .size(Size::new(Fill(1.0), Fixed(60)))
    .into()
}

/// 11) Grid layout example
fn view_grid(t: &Theme) -> Element {
    use std::num::NonZero;

    let cells: Vec<_> = (0..12).map(|i| small_block(swatch(i))).collect();

    WrappingRows::new(NonZero::new(4).unwrap(), cells)
        .col_spacing(space::SM)
        .row_spacing(space::SM)
        .padding(Vec4::splat(space::SM))
        .color(t.surface_variant)
        .size(Size::new(Fill(1.0), Fit))
        .into()
}

/// 12) Main-axis alignment: identical children under each distribution.
///     Because the container grows wider than its content, each row has free
///     space for `main()` to arrange. (A `Fill(1.0)` child would eat that slack and
///     make alignment a no-op — see `view_multiple_grow`.)
fn view_main_alignment(t: &Theme) -> Element {
    let modes = [
        Main::At(Align::START),
        Main::At(Align::CENTER),
        Main::At(Align::END),
        Main::Between,
        Main::Around,
        Main::Evenly,
    ];

    let rows: Vec<Element> = modes
        .into_iter()
        .map(|mode| {
            Row::new(el![
                Rectangle::new(Size::new(Fixed(40), Fixed(24)), swatch(0)),
                Rectangle::new(Size::new(Fixed(60), Fixed(24)), swatch(3)),
                Rectangle::new(Size::new(Fixed(50), Fixed(24)), swatch(5)),
            ])
            .spacing(space::SM)
            .padding(Vec4::splat(space::XS))
            .main(mode)
            .color(t.surface)
            .size(Size::new(Fill(1.0), Fixed(40)))
            .into()
        })
        .collect();

    Column::new(rows)
        .spacing(space::SM)
        .padding(Vec4::splat(space::SM))
        .color(t.surface_variant)
        .size(Size::new(Fill(1.0), Fit))
        .into()
}

/// 13) Cross-axis alignment. The first three rows use fixed-height children
///     of different sizes so Start/Center/End are visible. The last row uses
///     `Fit`-height children so `fill_cross` visibly grows them to fill the
///     row — resolved in the assign pass rather than in `place`.
fn view_cross_alignment(t: &Theme) -> Element {
    let varied = |cross: Align| -> Element {
        Row::new(el![
            Rectangle::new(Size::new(Fixed(44), Fixed(20)), swatch(0)),
            Rectangle::new(Size::new(Fixed(44), Fixed(48)), swatch(3)),
            Rectangle::new(Size::new(Fixed(44), Fixed(32)), swatch(5)),
        ])
        .spacing(space::SM)
        .padding(Vec4::splat(space::XS))
        .cross(cross)
        .color(t.surface)
        .size(Size::new(Fill(1.0), Fixed(64)))
        .into()
    };

    let stretch_row: Element = Row::new(el![
        Rectangle::new(Size::new(Fixed(44), Fit), swatch(1)),
        Rectangle::new(Size::new(Fixed(44), Fit), swatch(4)),
        Rectangle::new(Size::new(Fixed(44), Fit), swatch(6)),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::XS))
    .fill_cross(true)
    .color(t.surface)
    .size(Size::new(Fill(1.0), Fixed(64)))
    .into();

    Column::new(vec![
        varied(Align::START),
        varied(Align::CENTER),
        varied(Align::END),
        stretch_row,
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Fill(1.0), Fit))
    .into()
}

/// 14) `Center::new` one-liner: a Fill(1.0)/Fill(1.0) container that centers its child
///     on both axes within whatever space its parent gives it.
fn view_center_helper(t: &Theme) -> Element {
    Row::new(el![Center::new(Rectangle::new(
        Size::new(Fixed(80), Fixed(40)),
        swatch(4)
    ))])
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Fill(1.0), Fixed(120)))
    .into()
}
