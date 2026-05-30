use super::*;

use Length::{Fit, Fixed, Grow};

const HEADER_HEIGHT: i32 = 56;
const ROW_HEIGHT: i32 = 36;

const PANEL_BG: Color = Color::rgb(40, 44, 54);
const PANEL_BG_ALT: Color = Color::rgb(34, 38, 46);
const APP_BG: Color = Color::rgb(24, 26, 32);
const TEXT: Color = Color::rgb(220, 225, 235);
const ACCENT: Color = Color::rgb(88, 146, 255);

pub fn view(tid: &TargetId, state: &State) -> Element<Message> {
    let target = match state.per_target.get(tid) {
        Some(t) => t,
        None => return Rectangle::placeholder().into(),
    };

    let header = Row::new(el![
        Text::new("Scrollable demo")
            .font_size(20.0)
            .wrap(Wrap::None)
            .color(TEXT)
            .size(Size::new(Fit, Grow)),
        Spacer::new(Size::new(Grow, Grow)),
        Text::new(format!("button presses: {}", target.counter))
            .font_size(16.0)
            .wrap(Wrap::None)
            .color(ACCENT)
            .size(Size::new(Fit, Grow)),
    ])
    .padding(Vec4::new(16, 12, 16, 12))
    .spacing(12)
    .color(PANEL_BG_ALT)
    .size(Size::new(Grow, Fixed(HEADER_HEIGHT)));

    Column::new(el![
        header,
        Row::new(el![flat_panel(), nested_panel(),])
            .spacing(12)
            .padding(Vec4::splat(12))
            .color(APP_BG)
            .size(Size::new(Grow, Grow)),
    ])
    .color(APP_BG)
    .size(Size::new(Grow, Grow))
    .into()
}

/// Left panel: a single Scrollable with many buttons. Tests that wheel
/// scrolling works on first frame and that hit-testing inside the scrolled
/// region tracks the visible button.
fn flat_panel() -> Element<Message> {
    let buttons: Vec<Element<Message>> = (0..40)
        .map(|i| labeled_button(format!("Row {i}")).into())
        .collect();

    Column::new(el![
        panel_label("Flat scrollable (40 rows)"),
        Scrollable::new(
            Column::new(buttons)
                .spacing(6)
                .padding(Vec4::splat(8))
                .color(Color::TRANSPARENT)
                .size(Size::new(Grow, Fit)),
        )
        .size(Size::new(Grow, Grow))
        .bg(PANEL_BG_ALT),
    ])
    .spacing(8)
    .color(PANEL_BG)
    .padding(Vec4::splat(8))
    .size(Size::new(Grow, Grow))
    .into()
}

/// Right panel: a Scrollable that contains another Scrollable. Tests that
/// nested children_offset accumulates correctly through write_back.
fn nested_panel() -> Element<Message> {
    let inner_buttons: Vec<Element<Message>> = (100..130)
        .map(|i| labeled_button(format!("Inner {i}")).into())
        .collect();

    // The inner scrollable lives inside the outer one's content, sandwiched
    // between filler rows so the user has to scroll the outer to align it.
    let outer_content = Column::new(el![
        section_text("Top of outer scrollable. Scroll down to reach the inner panel."),
        filler_block("Filler 1", 120),
        filler_block("Filler 2", 120),
        section_text("Inner scrollable below — scroll inside it independently."),
        Scrollable::new(
            Column::new(inner_buttons)
                .spacing(4)
                .padding(Vec4::splat(8))
                .color(Color::TRANSPARENT)
                .size(Size::new(Grow, Fit)),
        )
        .size(Size::new(Grow, Fixed(220)))
        .bg(Color::rgb(28, 30, 38)),
        filler_block("Filler 3", 120),
        filler_block("Filler 4", 120),
        section_text("Bottom of outer scrollable."),
    ])
    .spacing(8)
    .padding(Vec4::splat(8))
    .color(Color::TRANSPARENT)
    .size(Size::new(Grow, Fit));

    Column::new(el![
        panel_label("Nested scrollable"),
        Scrollable::new(outer_content)
            .size(Size::new(Grow, Grow))
            .bg(PANEL_BG_ALT),
    ])
    .spacing(8)
    .color(PANEL_BG)
    .padding(Vec4::splat(8))
    .size(Size::new(Grow, Grow))
    .into()
}

fn labeled_button(label: String) -> Button<Message> {
    Button::new_with(
        Row::new(el![Text::new(label)
            .font_size(14.0)
            .wrap(cosmic_text::Wrap::None)
            .color(Color::WHITE),])
        .padding(Vec4::new(12, 0, 12, 0))
        .color(Color::TRANSPARENT)
        .size(Size::new(Grow, Grow)),
    )
    .color(Color::rgb(60, 66, 80))
    .hover_color(Color::rgb(80, 110, 160))
    .pressed_color(ACCENT)
    .on_press(Message::ButtonPressed)
    .size(Size::new(Grow, Fixed(ROW_HEIGHT)))
}

fn panel_label(s: &'static str) -> Element<Message> {
    Text::new(s)
        .font_size(14.0)
        .color(TEXT)
        .size(Size::new(Grow, Fixed(20)))
        .into()
}

fn section_text(s: &'static str) -> Element<Message> {
    Text::new(s)
        .font_size(13.0)
        .color(Color::rgb(160, 170, 190))
        .size(Size::new(Grow, Fit))
        .into()
}

fn filler_block(label: &'static str, h: i32) -> Element<Message> {
    Row::new(el![Text::new(label)
        .font_size(14.0)
        .color(Color::rgb(200, 205, 215))
        .size(Size::new(Fit, Grow)),])
    .padding(Vec4::splat(12))
    .color(Color::rgb(50, 54, 64))
    .size(Size::new(Grow, Fixed(h)))
    .into()
}
