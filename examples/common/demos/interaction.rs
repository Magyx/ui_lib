use super::*;

pub fn view(tid: &TargetId, state: &State) -> Element {
    use Length::{Fit, Fixed, Grow};

    let target = match state.per_target.get(tid) {
        Some(t) => t,
        None => return Rectangle::placeholder().into(),
    };
    let t = &state.theme;

    /* 1) button */
    let blocks = || {
        Row::new(
            (0..(target.counter % 6))
                .map(|i| small_block(swatch(i as usize)))
                .collect::<Vec<_>>(),
        )
        .color(Color::TRANSPARENT)
        .size(Size::new(Fit, Grow))
    };
    let buttons = Column::new(el![
        Row::new(el![
            Button::new(Size::new(Fixed(120), Fixed(size::CONTROL_H)), t.primary)
                .on_press(Message::ButtonPressed),
            blocks()
        ])
        .padding(Vec4::splat(space::SM))
        .spacing(space::SM)
        .color(t.surface_at(1))
        .size(Size::new(Grow, Fixed(60))),
        Row::new(el![
            Button::new_with(
                Column::new(el![
                    Spacer::new(Size::new(Grow, Grow)),
                    Text::body("Click Me!").wrap(Wrap::None).color(t.on_primary),
                    Spacer::new(Size::new(Grow, Grow)),
                ])
                .size(Size::new(Fit, Grow)),
            )
            .on_press(Message::ButtonPressed)
            .size(Size::new(Fit, Grow)),
            blocks()
        ])
        .padding(Vec4::splat(space::SM))
        .spacing(space::SM)
        .color(t.surface_at(1))
        .size(Size::new(Grow, Fixed(60))),
    ])
    .color(Color::TRANSPARENT)
    .spacing(space::MD)
    .size(Size::new(Grow, Fit));

    /* 2) slider */
    let slider_value = format!("Slider value: {:>5.1}", target.slider);
    let slider_row = Row::new(el![
        Text::label(slider_value)
            .wrap(Wrap::None)
            .size(Size::new(Fit, Fixed(size::CONTROL_H))),
        Spacer::new(Size::new(Fixed(space::MD), Fixed(1))),
        Slider::new(
            Size::new(Grow, Fixed(size::CONTROL_H)),
            (0.0, 100.0),
            target.slider,
        )
        .on_change(Message::SliderChanged), // emits f32 -> Message
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_at(1))
    .size(Size::new(Grow, Fixed(size::HEADER_H)));

    /* 3) text input */
    let greeting = if target.name.is_empty() {
        "Type your name to update the greeting…".to_string()
    } else {
        format!("Hello, {}!", target.name)
    };

    let inputs = Column::new(el![
        // Single-line TextField (themed automatically)
        TextField::new(target.name.clone(), Size::new(Grow, Fixed(size::CONTROL_H)))
            .placeholder("Your name")
            .on_change(|s| Message::NameChanged(s.to_string())),
        // Live feedback
        Text::body(greeting).size(Size::new(Grow, Fit)),
        // Multi-line TextArea
        TextArea::new(
            target.text_area_content.clone(),
            Size::new(Grow, Fixed(120))
        )
        .placeholder("Notes (multi-line)")
        .on_change(|s| Message::TextAreaContentChanged(s.to_string()))
    ])
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_at(1))
    .size(Size::new(Grow, Fit));

    Column::new(el![buttons, slider_row, inputs,])
        .spacing(space::SM)
        .padding(Vec4::splat(space::LG))
        .color(t.surface)
        .size(Size::new(Grow, Grow))
        .into()
}
