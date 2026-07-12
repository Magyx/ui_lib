use super::*;

use Length::{Fit, Fixed, Grow};

pub fn view(tid: &TargetId, state: &State) -> Element<Message> {
    let target = match state.per_target.get(tid) {
        Some(t) => t,
        None => return Rectangle::placeholder().into(),
    };
    let t = &state.theme;

    let header = Row::new(el![
        Text::h2("Scrollable demo")
            .wrap(Wrap::None)
            .size(Size::new(Fit, Grow)),
        Spacer::new(Size::new(Grow, Grow)),
        Text::body(format!("button presses: {}", target.counter))
            .wrap(Wrap::None)
            .color(t.primary)
            .size(Size::new(Fit, Grow)),
    ])
    .padding(Vec4::new(space::LG, space::MD, space::LG, space::MD))
    .spacing(space::MD)
    .color(t.surface_variant)
    .size(Size::new(Grow, Fixed(size::HEADER_H)));

    Column::new(el![
        header,
        Row::new(el![flat_panel(t), nested_panel(t),])
            .spacing(space::MD)
            .padding(Vec4::splat(space::MD))
            .color(t.bg)
            .size(Size::new(Grow, Grow)),
    ])
    .color(t.bg)
    .size(Size::new(Grow, Grow))
    .into()
}

/// Left panel: a single Scrollable with many buttons. Tests that wheel
/// scrolling works on first frame and that hit-testing inside the scrolled
/// region tracks the visible button.
fn flat_panel(t: &Theme) -> Element<Message> {
    let buttons: Vec<Element<Message>> = (0..40)
        .map(|i| labeled_button(t, format!("Row {i}")).into())
        .collect();

    Column::new(el![
        panel_label(t, "Flat scrollable (40 rows)"),
        // The scrollable is a recessed "well" (surface_variant); the rows are
        // raised (surface) with a border, so they read clearly against it.
        Scrollable::new(
            Column::new(buttons)
                .spacing(space::XS + 2)
                .padding(Vec4::splat(space::SM))
                .color(Color::TRANSPARENT)
                .size(Size::new(Grow, Fit)),
        )
        .size(Size::new(Grow, Grow))
        .bg(t.surface_variant),
    ])
    .spacing(space::SM)
    .color(t.surface)
    .padding(Vec4::splat(space::SM))
    .size(Size::new(Grow, Grow))
    .into()
}

/// Right panel: a Scrollable that contains another Scrollable. Tests that
/// nested children_offset accumulates correctly through write_back.
fn nested_panel(t: &Theme) -> Element<Message> {
    let inner_buttons: Vec<Element<Message>> = (100..130)
        .map(|i| labeled_button(t, format!("Inner {i}")).into())
        .collect();

    // The inner scrollable lives inside the outer one's content, sandwiched
    // between filler rows so the user has to scroll the outer to align it.
    let outer_content = Column::new(el![
        section_text(
            t,
            "Top of outer scrollable. Scroll down to reach the inner panel."
        ),
        filler_block(t, "Filler 1", 120),
        filler_block(t, "Filler 2", 120),
        section_text(
            t,
            "Inner scrollable below — scroll inside it independently."
        ),
        // Deeper recess (bg) so the inner well is distinct from the outer one.
        Scrollable::new(
            Column::new(inner_buttons)
                .spacing(space::XS)
                .padding(Vec4::splat(space::SM))
                .color(Color::TRANSPARENT)
                .size(Size::new(Grow, Fit)),
        )
        .size(Size::new(Grow, Fixed(220)))
        .bg(t.bg),
        filler_block(t, "Filler 3", 120),
        filler_block(t, "Filler 4", 120),
        section_text(t, "Bottom of outer scrollable."),
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(Color::TRANSPARENT)
    .size(Size::new(Grow, Fit));

    Column::new(el![
        panel_label(t, "Nested scrollable"),
        Scrollable::new(outer_content)
            .size(Size::new(Grow, Grow))
            .bg(t.surface_variant),
    ])
    .spacing(space::SM)
    .color(t.surface)
    .padding(Vec4::splat(space::SM))
    .size(Size::new(Grow, Grow))
    .into()
}

fn labeled_button(t: &Theme, label: String) -> Button<Message> {
    Button::new_with(
        Row::new(el![Text::label(label).wrap(Wrap::None).color(t.on_surface),])
            .padding(Vec4::new(space::MD, 0, space::MD, 0))
            .color(Color::TRANSPARENT)
            .size(Size::new(Grow, Grow)),
    )
    .color(t.surface)
    .border()
    .on_press(Message::ButtonPressed)
    .size(Size::new(Grow, Fixed(size::ROW_H)))
}

fn panel_label(t: &Theme, s: &'static str) -> Element<Message> {
    Text::label(s)
        .color(t.on_surface_variant)
        .size(Size::new(Grow, Fixed(20)))
        .into()
}

fn section_text(t: &Theme, s: &'static str) -> Element<Message> {
    Text::caption(s)
        .color(t.on_surface_variant)
        .size(Size::new(Grow, Fit))
        .into()
}

fn filler_block(t: &Theme, label: &'static str, h: i32) -> Element<Message> {
    Row::new(el![Text::body(label)
        .color(t.on_surface_variant)
        .size(Size::new(Fit, Grow)),])
    .padding(Vec4::splat(space::MD))
    .color(t.surface)
    .size(Size::new(Grow, Fixed(h)))
    .into()
}
