use super::*;

pub fn view(state: &State) -> Element<Message> {
    use Length::{Fit, Fixed, Grow};

    let t = state.theme;

    // -- Corner radius slider --
    let radius_row = Row::new(el![
        Text::label(format!("Corner Radius: {:.0}", t.corner_radius))
            .wrap(Wrap::None)
            .size(Size::new(Fixed(160), Grow)),
        Slider::new(
            Size::new(Grow, Fixed(size::SLIDER_H)),
            (0.0, 24.0),
            t.corner_radius,
        )
        .on_change(Message::ThemeCornerRadius),
    ])
    .spacing(space::MD)
    .padding(Vec4::splat(space::SM))
    .size(Size::new(Grow, Fixed(44)));

    // -- Border width slider --
    let border_row = Row::new(el![
        Text::label(format!("Border Width: {}", t.border_width))
            .wrap(Wrap::None)
            .size(Size::new(Fixed(160), Grow)),
        Slider::new(
            Size::new(Grow, Fixed(size::SLIDER_H)),
            (0.0, 6.0),
            t.border_width as f32,
        )
        .on_change(Message::ThemeBorderWidth),
    ])
    .spacing(space::MD)
    .padding(Vec4::splat(space::SM))
    .size(Size::new(Grow, Fixed(44)));

    // -- Dark / Light toggle --
    let toggle_row = Row::new(el![
        Button::new_with(
            Text::label("Dark")
                .wrap(Wrap::None)
                .size(Size::new(Fit, Grow)),
        )
        .on_press(Message::ThemeSetDark)
        .size(Size::new(Fixed(80), Fixed(32))),
        Button::new_with(
            Text::label("Light")
                .wrap(Wrap::None)
                .size(Size::new(Fit, Grow)),
        )
        .on_press(Message::ThemeSetLight)
        .size(Size::new(Fixed(80), Fixed(32))),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .size(Size::new(Grow, Fixed(48)));

    // -- Preview widgets --
    let preview = Column::new(el![
        Text::h3("Preview").size(Size::new(Grow, Fit)),
        // Button using theme defaults
        Button::new_with(
            Text::label("Theme Button")
                .wrap(Wrap::None)
                .size(Size::new(Fit, Grow)),
        )
        .on_press(Message::ButtonPressed)
        .size(Size::new(Fixed(160), Fixed(size::CONTROL_H))),
        // Input using theme defaults
        TextField::new("", Size::new(Grow, Fixed(size::CONTROL_H))).placeholder("Preview input"),
        // Slider using theme defaults
        Slider::new(Size::new(Grow, Fixed(size::SLIDER_H)), (0.0, 100.0), 65.0),
        // Nested container — distinct marker blocks
        Column::new(el![
            Text::caption("Nested container"),
            Row::new(el![
                Rectangle::new(Size::new(Fixed(40), Fixed(40)), swatch(0)),
                Rectangle::new(Size::new(Fixed(40), Fixed(40)), swatch(3)),
                Rectangle::new(Size::new(Fixed(40), Fixed(40)), swatch(5)),
            ])
            .spacing(space::SM)
            .size(Size::new(Grow, Fit)),
        ])
        .spacing(space::SM)
        .padding(Vec4::splat(space::MD))
        .color(t.surface)
        .size(Size::new(Grow, Fit)),
    ])
    .spacing(space::MD)
    .padding(Vec4::splat(space::MD))
    .color(t.surface)
    .size(Size::new(Grow, Fit));

    // -- Layout --
    Column::new(el![
        Text::h1("Theme Editor").size(Size::new(Grow, Fit)),
        toggle_row,
        radius_row,
        border_row,
        preview,
    ])
    .spacing(space::MD)
    .padding(Vec4::splat(space::LG))
    .color(t.bg)
    .size(Size::new(Grow, Grow))
    .into()
}
