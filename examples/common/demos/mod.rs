use super::{Message, State};

use ui::{
    el,
    graphics::TargetId,
    model::*,
    widget::{
        Button, Column, Element, Length, Overlay, Rectangle, Row, Scrollable, Slider, Spacer, Text,
        TextArea, TextField, WrappingRows,
    },
};

fn small_block(r: u8, g: u8, b: u8) -> Element<Message> {
    Rectangle::new(
        Size::new(Length::Fixed(24), Length::Fixed(24)),
        Color::rgb(r, g, b),
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

        /* 1) button */
        let blocks = || {
            Row::new(
                (0..(target.counter % 6))
                    .map(|i| {
                        let c = (i * 30 + 40) as u8;
                        small_block(c, 30, 200u8.saturating_sub(c))
                    })
                    .collect::<Vec<_>>(),
            )
            .color(Color::TRANSPARENT)
            .size(Size::new(Fit, Grow))
        };
        let buttons = Column::new(el![
            Row::new(el![
                Button::new(Size::new(Fixed(120), Fixed(36)), Color::rgb(200, 50, 50))
                    .hover_color(Color::rgb(50, 200, 50))
                    .pressed_color(Color::rgb(50, 50, 200))
                    .on_press(Message::ButtonPressed),
                blocks()
            ])
            .padding(Vec4::splat(10))
            .spacing(10)
            .color(Color::rgb(220, 220, 240))
            .size(Size::new(Grow, Fixed(60))),
            Row::new(el![
                Button::new_with(
                    Column::new(el![
                        Spacer::new(Size::new(Grow, Grow)),
                        Text::new("Click Me!", 18.0).wrap(Wrap::None),
                        Spacer::new(Size::new(Grow, Grow)),
                    ])
                    .size(Size::new(Fit, Grow)),
                )
                .color(Color::rgb(200, 50, 50))
                .hover_color(Color::rgb(50, 200, 50))
                .pressed_color(Color::rgb(50, 50, 200))
                .on_press(Message::ButtonPressed)
                .size(Size::new(Fit, Grow)),
                blocks()
            ])
            .padding(Vec4::splat(10))
            .spacing(10)
            .color(Color::rgb(220, 220, 240))
            .size(Size::new(Grow, Fixed(60))),
        ])
        .color(Color::rgb(100, 80, 100))
        .spacing(14)
        .size(Size::new(Grow, Fit));

        /* 2) slider */
        let slider_value = format!("Slider value: {:>5.1}", target.slider);
        let slider_row = Row::new(el![
            Text::new(slider_value, 16.0)
                .wrap(Wrap::None)
                .size(Size::new(Fit, Fixed(36)))
                .color(Color::BLACK),
            Spacer::new(Size::new(Fixed(12), Fixed(1))),
            Slider::new(Size::new(Grow, Fixed(36)), (0.0, 100.0), target.slider)
                .on_change(Message::SliderChanged), // emits f32 -> Message
        ])
        .spacing(10)
        .padding(Vec4::splat(10))
        .color(Color::rgb(235, 235, 245))
        .size(Size::new(Grow, Fixed(56)));

        /* 3) text input */
        let greeting = if target.name.is_empty() {
            "Type your name to update the greeting…".to_string()
        } else {
            format!("Hello, {}!", target.name)
        };

        let inputs = Column::new(el![
            // Single-line TextField
            TextField::new(Size::new(Grow, Fixed(36)))
                .placeholder("Your name")
                .on_change(|s| Message::NameChanged(s.to_string())),
            // Live feedback
            Text::new(greeting, 16.0)
                .size(Size::new(Grow, Fit))
                .color(Color::BLACK),
            // Multi-line TextArea
            TextArea::new(Size::new(Grow, Fixed(120))).placeholder("Notes (multi-line)")
        ])
        .spacing(8)
        .padding(Vec4::splat(10))
        .color(Color::rgb(245, 245, 245))
        .size(Size::new(Grow, Fit));

        Column::new(el![buttons, slider_row, inputs,])
            .spacing(10)
            .padding(Vec4::splat(16))
            .color(Color::rgb(100, 80, 100))
            .size(Size::new(Grow, Grow))
            .into()
    }
}

pub mod pipeline {

    use super::*;
    use cosmic_text::Weight;
    use ui::widget::SimpleCanvas;

    pub fn view(tid: &TargetId, state: &State) -> Element<Message> {
        use Length::{Fit, Grow};

        let target = match state.per_target.get(tid) {
            Some(t) => t,
            None => return Rectangle::placeholder().into(),
        };
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
                Text::new(
                    format!(
                        "{:.0}",
                        target.fps.iter().sum::<f32>() / target.fps.len().max(1) as f32
                    ),
                    16.0,
                )
                .size(Size::new(Fit, Fit))
                .color(Color::BLUE)
                .weight(Weight::SEMIBOLD),
            ])
            .padding(Vec4::splat(10))
            .size(Size::new(Grow, Fit)),
        ])
        .color(Color::rgb(20, 20, 40))
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

        let png_cells: Vec<Element<Message>> = state
            .icons
            .iter()
            .take(MAX_DEMO_ICONS)
            .map(|&h| Image::new(Size::new(Fixed(ICON_PX), Fixed(ICON_PX)), h).into())
            .collect();

        let png_panel = Column::new(el![
            Text::new("PNG", 16.0).color(Color::BLACK),
            WrappingRows::new(NonZero::new(GRID_COLS).unwrap(), png_cells)
                .col_spacing(8)
                .row_spacing(8)
                .size(Size::new(Fit, Fit))
                .color(Color::TRANSPARENT),
        ])
        .spacing(10)
        .padding(Vec4::splat(12))
        .color(Color::rgb(235, 235, 235))
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
                Text::new("SVG", 16.0).color(Color::BLACK),
                WrappingRows::new(NonZero::new(GRID_COLS).unwrap(), svg_cells)
                    .col_spacing(8)
                    .row_spacing(8)
                    .size(Size::new(Fit, Fit))
                    .color(Color::TRANSPARENT),
            ])
            .spacing(10)
            .padding(Vec4::splat(12))
            .color(Color::rgb(235, 235, 235))
            .size(Size::new(Grow, Fit))
        };

        #[cfg(not(feature = "svg"))]
        let svg_panel = Column::new(el![
            Text::new("SVG", 16.0).color(Color::BLACK),
            Text::new("Enable with --features svg", 14.0).color(Color::rgb(50, 50, 50)),
        ])
        .spacing(10)
        .padding(Vec4::splat(12))
        .color(Color::rgb(235, 235, 235))
        .size(Size::new(Grow, Fit));

        let two_col = Row::new(el![png_panel, svg_panel])
            .spacing(16)
            .size(Size::new(Grow, Fit));

        Overlay::new(el![
            Image::new(Size::new(Grow, Grow), state.background.unwrap_or_default())
                .fit(ContentFit::Cover),
            Column::new(el![
                Rectangle::new(Size::new(Fixed(70), Fixed(20)), Color::rgb(100, 0, 100)),
                Rectangle::new(Size::new(Fixed(40), Fixed(30)), Color::rgb(140, 0, 140)),
            ])
            .spacing(10)
            .padding(Vec4::splat(10))
            .color(Color::rgba(220, 240, 240, 1))
            .size(Size::new(Fixed(70), Fixed(80))),
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
    use cosmic_text::Weight;

    pub fn view(_state: &State) -> Element<Message> {
        use Length::{Fit, Fixed, Grow};

        // Colors
        let bg_app = Color::rgb(24, 26, 32);
        let bg_panel = Color::rgb(34, 38, 46);
        let bg_panel_alt = Color::rgb(40, 44, 54);
        let fg_title = Color::rgb(235, 240, 255);
        let fg_text = Color::rgb(210, 215, 230);
        let accent = Color::rgb(88, 146, 255);

        // Sidebar (fixed width)
        let sidebar = Column::new(el![
            // Sidebar header
            Row::new(el![Text::new("Project Nimbus", 20.0).color(fg_title)])
                .padding(Vec4::new(16, 16, 16, 8))
                .color(Color::TRANSPARENT)
                .size(Size::new(Grow, Fixed(40))),
            // Sidebar items
            Column::new(el![
                Text::new("Overview", 16.0).color(fg_text),
                Text::new("Assets", 16.0).color(fg_text),
                Text::new("Settings", 16.0).color(fg_text),
            ])
            .spacing(8)
            .padding(Vec4::new(16, 8, 16, 16))
            .color(Color::TRANSPARENT)
            .size(Size::new(Grow, Fit)),
        ])
        .spacing(6)
        .padding(Vec4::splat(8))
        .color(bg_panel)
        .size(Size::new(Fixed(220), Grow));

        // Top bar (fixed height)
        let topbar = Row::new(el![
            Text::new("Dashboard", 22.0).color(fg_title),
            Spacer::new(Size::new(Grow, Grow)),
            // a little “pill” on the right
            Row::new(el![Text::new("LIVE", 14.0).weight(Weight::BLACK)])
                .padding(Vec4::new(10, 6, 10, 6))
                .color(accent)
                .size(Size::new(Fit, Grow)),
        ])
        .padding(Vec4::new(16, 10, 16, 10))
        .color(bg_panel_alt)
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
            Text::new("Welcome to the Showcase", 20.0)
                .size(Size::new(Grow, Fit))
                .color(fg_title),
            // Body (multiline)
            Text::new(hero_text, 16.0)
                .size(Size::new(Grow, Fit))
                .color(fg_text),
            // Body (fit checks)
            Column::new(el![
                Row::new(el![
                    Text::new(long, 16.0).size(Size::new(Grow, Fit)),
                    Text::new(long, 16.0).size(Size::new(Grow, Fit)),
                ])
                .size(Size::new(Grow, Fit))
                .spacing(12),
                Text::new(long, 16.0).size(Size::new(Grow, Fit)),
            ])
            .size(Size::new(Grow, Fit))
            .spacing(12),
            // List of text with scrolling
            Scrollable::new(Text::new(list, 16.0))
                .size(Size::new(Grow, Fixed(140)))
                .bg(Color::rgb(72, 78, 90)),
            // A couple of stat tiles
            Row::new(el![
                Column::new(el![
                    Text::new("Builds", 16.0).color(fg_text),
                    Text::new("128", 28.0).color(fg_title),
                ])
                .padding(Vec4::splat(12))
                .color(bg_panel)
                .size(Size::new(Grow, Fixed(88))),
                Column::new(el![
                    Text::new("Warnings", 16.0).color(fg_text),
                    Text::new("3", 28.0).color(Color::rgb(255, 206, 86)),
                ])
                .padding(Vec4::splat(12))
                .color(bg_panel)
                .size(Size::new(Grow, Fixed(88))),
                Column::new(el![
                    Text::new("Errors", 16.0).color(fg_text),
                    Text::new("0", 28.0).color(Color::rgb(76, 217, 100)),
                ])
                .padding(Vec4::splat(12))
                .color(bg_panel)
                .size(Size::new(Grow, Fixed(88))),
            ])
            .spacing(12)
            .padding(Vec4::splat(0))
            .color(Color::TRANSPARENT)
            .size(Size::new(Grow, Fit)),
        ])
        .spacing(12)
        .padding(Vec4::splat(16))
        .color(Color::TRANSPARENT)
        .size(Size::new(Grow, Fit));

        // Page layout: sidebar | (topbar + content)
        Row::new(el![
            sidebar,
            Scrollable::new(
                Column::new(el![topbar, content,])
                    .spacing(12)
                    .color(Color::TRANSPARENT)
                    .size(Size::new(Grow, Fit)),
            )
            .size(Size::new(Grow, Fit)),
        ])
        .spacing(12)
        .padding(Vec4::splat(12))
        .color(bg_app)
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
            Text::new(format!("Corner Radius: {:.0}", t.corner_radius), 14.0)
                .wrap(Wrap::None)
                .size(Size::new(Fixed(160), Grow)),
            Slider::new(Size::new(Grow, Fixed(28)), (0.0, 24.0), t.corner_radius)
                .on_change(Message::ThemeCornerRadius),
        ])
        .spacing(12)
        .padding(Vec4::splat(8))
        .size(Size::new(Grow, Fixed(44)));

        // -- Border width slider --
        let border_row = Row::new(el![
            Text::new(format!("Border Width: {}", t.border_width), 14.0)
                .wrap(Wrap::None)
                .size(Size::new(Fixed(160), Grow)),
            Slider::new(
                Size::new(Grow, Fixed(28)),
                (0.0, 6.0),
                t.border_width as f32,
            )
            .on_change(Message::ThemeBorderWidth),
        ])
        .spacing(12)
        .padding(Vec4::splat(8))
        .size(Size::new(Grow, Fixed(44)));

        // -- Dark / Light toggle --
        let toggle_row = Row::new(el![
            Button::new_with(
                Text::new("Dark", 14.0)
                    .wrap(Wrap::None)
                    .size(Size::new(Fit, Grow)),
            )
            .on_press(Message::ThemeSetDark)
            .size(Size::new(Fixed(80), Fixed(32))),
            Button::new_with(
                Text::new("Light", 14.0)
                    .wrap(Wrap::None)
                    .size(Size::new(Fit, Grow)),
            )
            .on_press(Message::ThemeSetLight)
            .size(Size::new(Fixed(80), Fixed(32))),
        ])
        .spacing(8)
        .padding(Vec4::splat(8))
        .size(Size::new(Grow, Fixed(48)));

        // -- Preview widgets --
        let preview = Column::new(el![
            Text::new("Preview", 16.0).size(Size::new(Grow, Fit)),
            // Button using theme defaults
            Button::new_with(
                Text::new("Theme Button", 14.0)
                    .wrap(Wrap::None)
                    .size(Size::new(Fit, Grow)),
            )
            .on_press(Message::ButtonPressed)
            .size(Size::new(Fixed(160), Fixed(36))),
            // Input using theme defaults
            TextField::new(Size::new(Grow, Fixed(36))).placeholder("Preview input"),
            // Slider using theme defaults
            Slider::new(Size::new(Grow, Fixed(28)), (0.0, 100.0), 65.0),
            // Nested container
            Column::new(el![
                Text::new("Nested container", 13.0),
                Row::new(el![
                    Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::rgb(200, 60, 60)),
                    Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::rgb(60, 200, 60)),
                    Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::rgb(60, 60, 200)),
                ])
                .spacing(8)
                .size(Size::new(Grow, Fit)),
            ])
            .spacing(8)
            .padding(Vec4::splat(12))
            .size(Size::new(Grow, Fit)),
        ])
        .spacing(12)
        .padding(Vec4::splat(12))
        .size(Size::new(Grow, Fit));

        // -- Layout --
        Column::new(el![
            Text::new("Theme Editor", 20.0).size(Size::new(Grow, Fit)),
            toggle_row,
            radius_row,
            border_row,
            preview,
        ])
        .spacing(12)
        .padding(Vec4::splat(16))
        .size(Size::new(Grow, Grow))
        .into()
    }
}
