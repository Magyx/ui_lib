use super::{Message, State};

use ui::{
    el,
    graphics::TargetId,
    model::*,
    text::Wrap,
    theme::Theme,
    widget::{
        Button, Column, Element, Length, Overlay, Rectangle, Row, Scrollable, Slider, Spacer, Text,
        TextArea, TextField, WrappingRows,
    },
};

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

pub mod layout;
pub mod scrollable;

pub mod interaction {

    use super::*;

    pub fn view(tid: &TargetId, state: &State) -> Element<Message> {
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
                    .hover_color(t.primary_container)
                    .pressed_color(t.primary_container)
                    .on_press(Message::ButtonPressed),
                blocks()
            ])
            .padding(Vec4::splat(space::SM))
            .spacing(space::SM)
            .color(t.surface)
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
            .color(t.surface)
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
        .color(t.surface)
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
        .color(t.surface)
        .size(Size::new(Grow, Fit));

        Column::new(el![buttons, slider_row, inputs,])
            .spacing(space::SM)
            .padding(Vec4::splat(space::LG))
            .color(t.bg)
            .size(Size::new(Grow, Grow))
            .into()
    }
}

pub mod pipeline {

    use super::*;
    use ui::widget::SimpleCanvas;

    pub fn view(tid: &TargetId, state: &State) -> Element<Message> {
        use Length::{Fit, Grow};

        let target = match state.per_target.get(tid) {
            Some(t) => t,
            None => return Rectangle::placeholder().into(),
        };
        let t = &state.theme;

        Overlay::new(el![
            SimpleCanvas::new(
                Size::new(Grow, Grow),
                "planet",
                Some(|cx| {
                    cx.ui.request_redraw();
                }),
            ),
            Row::new(el![
                Spacer::new(Size::new(Grow, Fit)),
                Text::h3(format!(
                    "{:.0}",
                    target.fps.iter().sum::<f32>() / target.fps.len().max(1) as f32
                ))
                .size(Size::new(Fit, Fit))
                .color(t.error),
            ])
            .padding(Vec4::splat(space::SM))
            .size(Size::new(Grow, Fit)),
        ])
        .color(t.bg)
        .padding(Vec4::splat(0))
        .size(Size::new(Grow, Grow))
        .into()
    }
}

pub mod texture {

    use super::*;
    use std::num::NonZero;
    use ui::widget::{ContentFit, Image};

    pub fn view(state: &State) -> Element<Message> {
        use Length::{Fit, Fixed, Grow};

        const ICON_PX: i32 = 48;
        const GRID_COLS: usize = 4;
        const MAX_DEMO_ICONS: usize = 16;

        let t = &state.theme;

        let png_cells: Vec<Element<Message>> = state
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
        .color(t.surface)
        .size(Size::new(Grow, Fit));

        #[cfg(feature = "svg")]
        let svg_panel = {
            use ui::widget::Svg;

            let svg_cells: Vec<Element<Message>> = state
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
            .color(t.surface)
            .size(Size::new(Grow, Fit))
        };

        #[cfg(not(feature = "svg"))]
        let svg_panel = Column::new(el![
            Text::h3("SVG"),
            Text::caption("Enable with --features svg").color(t.on_surface_variant),
        ])
        .spacing(space::SM)
        .padding(Vec4::splat(space::MD))
        .color(t.surface)
        .size(Size::new(Grow, Fit));

        let two_col = Row::new(el![png_panel, svg_panel])
            .spacing(space::LG)
            .size(Size::new(Grow, Fit));

        Overlay::new(el![
            Image::new(Size::new(Grow, Grow), state.background.unwrap_or_default())
                .fit(ContentFit::Cover),
            Column::new(el![
                Rectangle::new(Size::new(Fixed(70), Fixed(20)), swatch(7)),
                Rectangle::new(Size::new(Fixed(40), Fixed(30)), swatch(6)),
            ])
            .spacing(space::SM)
            .padding(Vec4::splat(space::SM))
            .color(with_alpha(t.surface, 235))
            .size(Size::splat(Fit)),
            Row::new(el![two_col])
                .padding(Vec4::splat(120))
                .size(Size::new(Grow, Grow)),
        ])
        .padding(Vec4::splat(0))
        .size(Size::new(Grow, Grow))
        .into()
    }
}

pub mod text {

    use super::*;

    pub fn view(state: &State) -> Element<Message> {
        use Length::{Fit, Fixed, Grow};

        let t = &state.theme;

        // Sidebar (fixed width)
        let sidebar = Column::new(el![
            // Sidebar header — h3 + no-wrap so it stays on one line inside the
            // narrow sidebar (h2 wrapped to two lines and overflowed its row).
            Row::new(el![Text::h3("Project Nimbus").wrap(Wrap::None)])
                .padding(Vec4::new(space::LG, space::LG, space::LG, space::SM))
                .color(Color::TRANSPARENT)
                .size(Size::new(Grow, Fit)),
            // Sidebar items
            Column::new(el![
                Text::body("Overview"),
                Text::body("Assets"),
                Text::body("Settings"),
            ])
            .spacing(space::SM)
            .padding(Vec4::new(space::LG, space::SM, space::LG, space::LG))
            .color(Color::TRANSPARENT)
            .size(Size::new(Grow, Fit)),
        ])
        .spacing(space::XS)
        .padding(Vec4::splat(space::SM))
        .color(t.surface)
        .size(Size::new(Fixed(220), Grow));

        // Top bar (fixed height)
        let topbar = Row::new(el![
            Text::h2("Dashboard"),
            Spacer::new(Size::new(Grow, Grow)),
            // a little "pill" on the right
            Row::new(el![Text::label("LIVE").color(t.on_primary)])
                .padding(Vec4::new(
                    space::MD,
                    space::XS + 2,
                    space::MD,
                    space::XS + 2
                ))
                .color(t.primary)
                .size(Size::new(Fit, Grow)),
        ])
        .padding(Vec4::new(space::LG, space::MD, space::LG, space::MD))
        .color(t.surface_variant)
        .size(Size::new(Grow, Fixed(52)));

        // Main content
        let hero_text = "This area demonstrates styled, multiline text using cosmic-text. \n\
            The grey rectangle below acts as an image/preview placeholder. \n\
            Resize the window to see wrapping and layout negotiation.";
        let long = "This is a very very very long line of text that should wrap \
                when the container is narrower than the preferred single-line width.";
        let list = "\
            Lorem ipsum dolor sit amet, consectetur adipiscing elit.\n\
            Etiam ullamcorper arcu a dolor eleifend luctus.\n\
            Vestibulum sit amet mi quis lacus cursus accumsan eu non ante.\n\
            Etiam a magna hendrerit massa mattis fermentum ac eu nisl.\n\
            Quisque vulputate eros id quam pulvinar, vel aliquam tellus placerat.\n\
            Pellentesque sollicitudin odio eu neque fringilla varius.\n\n\
            In dignissim odio et nunc posuere laoreet.\n\
            Phasellus facilisis sapien sit amet lectus vestibulum elementum.\n\
            Proin in turpis convallis, mollis ligula et, tincidunt ante.\n\n\
            Ut vestibulum risus at turpis tincidunt, ut eleifend erat euismod.\n\
            Nullam sed turpis convallis, laoreet lacus id, rutrum dolor.\n\
            In euismod diam at elit blandit lobortis.\n\n\
            Nulla interdum neque non neque aliquet sodales.\n\
            Aenean non purus et nulla dignissim gravida.\n\
            Ut placerat lorem non lorem ultricies tincidunt.\n\
            Nullam eu tortor at dui tincidunt pulvinar vitae vel quam.\n\n\
            Maecenas aliquam sem fringilla tellus ornare placerat.\n\
            Nam viverra nibh a metus ornare vulputate.\n\
            Donec quis neque et nisl fermentum ultrices.\n\
        ";

        let content = Column::new(el![
            // Title
            Text::h1("Welcome to the Showcase").size(Size::new(Grow, Fit)),
            // Body (multiline)
            Text::body(hero_text).size(Size::new(Grow, Fit)),
            // Body (fit checks)
            Column::new(el![
                Row::new(el![
                    Text::body(long).size(Size::new(Grow, Fit)),
                    Text::body(long).size(Size::new(Grow, Fit)),
                ])
                .size(Size::new(Grow, Fit))
                .spacing(space::MD),
                Text::body(long).size(Size::new(Grow, Fit)),
            ])
            .size(Size::new(Grow, Fit))
            .spacing(space::MD),
            // List of text with scrolling
            Scrollable::new(Text::body(list))
                .size(Size::new(Grow, Fixed(140)))
                .bg(t.surface_variant),
            // A couple of stat tiles
            Row::new(el![
                Column::new(el![Text::label("Builds"), Text::h1("128").color(t.primary),])
                    .padding(Vec4::splat(space::MD))
                    .color(t.surface)
                    .size(Size::new(Grow, Fixed(88))),
                Column::new(el![
                    Text::label("Warnings"),
                    Text::h1("3").color(t.secondary),
                ])
                .padding(Vec4::splat(space::MD))
                .color(t.surface)
                .size(Size::new(Grow, Fixed(88))),
                Column::new(el![Text::label("Errors"), Text::h1("0").color(t.error),])
                    .padding(Vec4::splat(space::MD))
                    .color(t.surface)
                    .size(Size::new(Grow, Fixed(88))),
            ])
            .spacing(space::MD)
            .padding(Vec4::splat(0))
            .color(Color::TRANSPARENT)
            .size(Size::new(Grow, Fit)),
        ])
        .spacing(space::MD)
        .padding(Vec4::splat(space::LG))
        .color(Color::TRANSPARENT)
        .size(Size::new(Grow, Fit));

        // Page layout: sidebar | (topbar + content)
        Row::new(el![
            sidebar,
            Scrollable::new(
                Column::new(el![topbar, content,])
                    .spacing(space::MD)
                    .color(Color::TRANSPARENT)
                    .size(Size::new(Grow, Fit)),
            )
            .size(Size::new(Grow, Fit)),
        ])
        .spacing(space::MD)
        .padding(Vec4::splat(space::MD))
        .color(t.bg)
        .size(Size::new(Grow, Grow))
        .into()
    }
}

pub mod theme_editor {

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
            TextField::new("", Size::new(Grow, Fixed(size::CONTROL_H)))
                .placeholder("Preview input"),
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
}
