use super::*;
use std::num::NonZero;

pub fn view(state: &State) -> Element {
    use Length::{Fill, Fit, Fixed};

    const ICON_PX: i32 = 48;
    const GRID_COLS: usize = 4;
    const MAX_DEMO_ICONS: usize = 16;

    let t = &state.theme;

    let png_cells: Vec<Element> = state
        .icons
        .iter()
        .take(MAX_DEMO_ICONS)
        .map(|&h| Image::new(Size::new(Fixed(ICON_PX), Fixed(ICON_PX)), h).into())
        .collect();

    let png_panel = Column::new(el![
        Text::h3("PNG"),
        WrappingRows::new(NonZero::new(GRID_COLS).unwrap(), png_cells)
            .col_spacing(space::SM)
            .row_spacing(space::SM)
            .size(Size::new(Fit, Fit))
            .color(Color::TRANSPARENT),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::MD))
    .color(t.surface_at(1))
    .size(Size::new(Fill(1.0), Fit));

    #[cfg(feature = "svg")]
    let svg_panel = {
        use ui::widget::Svg;

        let svg_cells: Vec<Element> = state
            .svg_icons
            .iter()
            .take(MAX_DEMO_ICONS)
            .cloned()
            .map(|p| Svg::new(Size::new(Fixed(ICON_PX), Fixed(ICON_PX)), p).into())
            .collect();

        Column::new(el![
            Text::h3("SVG"),
            WrappingRows::new(NonZero::new(GRID_COLS).unwrap(), svg_cells)
                .col_spacing(space::SM)
                .row_spacing(space::SM)
                .size(Size::new(Fit, Fit))
                .color(Color::TRANSPARENT),
        ])
        .spacing(space::SM)
        .padding(Vec4::splat(space::MD))
        .color(t.surface_at(1))
        .size(Size::new(Fill(1.0), Fit))
    };

    #[cfg(not(feature = "svg"))]
    let svg_panel = Column::new(el![
        Text::h3("SVG"),
        Text::caption("Enable with --features svg").color(t.on_surface_variant),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::MD))
    .color(t.surface_at(1))
    .size(Size::new(Fill(1.0), Fit));

    let two_col = Row::new(el![png_panel, svg_panel])
        .spacing(space::LG)
        .size(Size::new(Fill(1.0), Fit));

    Stack::new(el![
        Image::new(
            Size::new(Fill(1.0), Fill(1.0)),
            state.background.unwrap_or_default()
        )
        .fit(ContentFit::Cover),
        Column::new(el![
            Rectangle::new(Size::new(Fixed(70), Fixed(20)), swatch(7)),
            Rectangle::new(Size::new(Fixed(40), Fixed(30)), swatch(6)),
        ])
        .spacing(space::SM)
        .padding(Vec4::splat(space::SM))
        .color(with_alpha(t.surface, 235))
        .size(Size::splat(Fit))
        .pinned(Align2::TOP_RIGHT)
        .offset(-space::MD, space::MD),
        Row::new(el![two_col])
            .size(Size::new(Fill(1.0), Fill(1.0)))
            .edges(Edges::all(120)),
    ])
    .padding(Vec4::splat(0))
    .size(Size::new(Fill(1.0), Fill(1.0)))
    .into()
}
