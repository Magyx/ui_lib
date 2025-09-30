use super::{Message, State};
use ui::{
    model::*,
    widget::{Button, Column, Container, Element, Length, Rectangle, Row, Widget},
};

fn small_block(r: u8, g: u8, b: u8) -> Element<Message> {
    Rectangle::new(
        Size::new(Length::Fixed(24), Length::Fixed(24)),
        Color::rgb(r, g, b),
    )
    .einto()
}

pub mod layout {
    use super::*;

    pub fn view(_state: &State) -> Element<Message> {
        use Length::{Fit, Fixed, Grow};

        Column::new(vec![
            /* 1) Fixed + Fixed, zero padding baseline */
            Row::new(vec![
                Rectangle::new(Size::new(Fixed(80), Fixed(40)), Color::RED).einto(),
                Rectangle::new(Size::new(Fixed(120), Fixed(40)), Color::GREEN).einto(),
            ])
            .spacing(8)
            .padding(Vec4::splat(0))
            .color(Color::rgb(240, 240, 240))
            .size(Size::new(Grow, Fixed(70)))
            .einto(),

            /* 2) Fixed + Grow + Fixed; height fixed, width grow (checks single-grow distribution) */
            Row::new(vec![
                Rectangle::new(Size::new(Fixed(60), Grow), Color::rgb(255, 200, 0)).einto(),
                Rectangle::new(Size::new(Grow, Grow), Color::rgb(0, 180, 180)).einto(),
                Rectangle::new(Size::new(Fixed(60), Grow), Color::rgb(255, 200, 0)).einto(),
            ])
            .spacing(10)
            .padding(Vec4::splat(10))
            .color(Color::rgb(220, 220, 240))
            .size(Size::new(Grow, Fixed(80)))
            .einto(),

            /* 3) Multiple Grow children in a Row (checks equalization logic) */
            Row::new(vec![
                Rectangle::new(Size::new(Grow, Fixed(50)), Color::rgb(200, 50, 50)).einto(),
                Rectangle::new(Size::new(Grow, Fixed(50)), Color::rgb(50, 200, 50)).einto(),
                Rectangle::new(Size::new(Grow, Fixed(50)), Color::rgb(50, 50, 200)).einto(),
            ])
            .spacing(6)
            .padding(Vec4::splat(10))
            .color(Color::rgb(240, 220, 220))
            .size(Size::new(Grow, Fixed(70)))
            .einto(),

            /* 4) Column with Grow height distribution and fixed caps at top/bottom */
            Column::new(vec![
                Rectangle::new(Size::new(Grow, Fixed(20)), Color::rgb(80, 80, 80)).einto(),
                Rectangle::new(Size::new(Grow, Grow), Color::rgb(100, 200, 100)).einto(),
                Rectangle::new(Size::new(Grow, Fixed(20)), Color::rgb(80, 80, 150)).einto(),
            ])
            .spacing(8)
            .padding(Vec4::splat(10))
            .color(Color::rgb(240, 240, 220))
            .size(Size::new(Grow, Fixed(100)))
            .einto(),

            /* 5) Fit sizing demo: Column(Fit,Fit) measured by fixed children, next to a Grow rectangle */
            Row::new(vec![
                Column::new(vec![
                    Rectangle::new(Size::new(Fixed(70), Fixed(20)), Color::rgb(100, 0, 100))
                        .einto(),
                    Rectangle::new(Size::new(Fixed(40), Fixed(30)), Color::rgb(140, 0, 140))
                        .einto(),
                ])
                .spacing(4)
                .padding(Vec4::splat(4))
                .size(Size::new(Fit, Fit))
                .color(Color::rgb(230, 200, 230))
                .einto(),
                Rectangle::new(Size::new(Grow, Fixed(60)), Color::rgb(180, 180, 180)).einto(),
            ])
            .spacing(10)
            .padding(Vec4::splat(10))
            .color(Color::rgb(220, 240, 240))
            .size(Size::new(Grow, Fixed(80)))
            .einto(),

            /* 6) Nested grow: Row of two Columns; left fixed width, right flexible */
            Row::new(vec![
                Column::new(vec![
                    Rectangle::new(Size::new(Grow, Fixed(18)), Color::rgb(160, 160, 0)).einto(),
                    Rectangle::new(Size::new(Grow, Grow), Color::rgb(160, 100, 0)).einto(),
                ])
                .spacing(6)
                .padding(Vec4::splat(6))
                .size(Size::new(Fixed(200), Grow))
                .color(Color::rgb(250, 240, 200))
                .einto(),
                Column::new(vec![
                    Rectangle::new(Size::new(Grow, Grow), Color::rgb(0, 120, 160)).einto(),
                    Rectangle::new(Size::new(Grow, Fixed(24)), Color::rgb(0, 80, 120)).einto(),
                ])
                .spacing(6)
                .padding(Vec4::splat(6))
                .size(Size::new(Grow, Grow))
                .color(Color::rgb(200, 240, 250))
                .einto(),
            ])
            .spacing(10)
            .padding(Vec4::splat(10))
            .color(Color::rgb(240, 230, 230))
            .size(Size::new(Grow, Fixed(100)))
            .einto(),

            /* 7) Spacing extremes: zero vs nonzero, plus a Grow filler */
            Row::new(vec![
                Row::new(vec![
                    Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::rgb(0, 0, 0)).einto(),
                    Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::rgb(80, 80, 80))
                        .einto(),
                ])
                .spacing(0)
                .padding(Vec4::splat(0))
                .size(Size::new(Fixed(100), Fixed(40)))
                .color(Color::rgb(220, 220, 220))
                .einto(),
                Row::new(vec![
                    Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::rgb(0, 0, 0)).einto(),
                    Rectangle::new(Size::new(Fixed(40), Fixed(40)), Color::rgb(80, 80, 80))
                        .einto(),
                ])
                .spacing(12)
                .padding(Vec4::splat(0))
                .size(Size::new(Fixed(120), Fixed(40)))
                .color(Color::rgb(220, 220, 220))
                .einto(),
                Rectangle::new(Size::new(Grow, Fixed(40)), Color::rgb(200, 200, 200)).einto(),
            ])
            .spacing(10)
            .padding(Vec4::splat(10))
            .color(Color::rgb(220, 220, 240))
            .size(Size::new(Grow, Fixed(60)))
            .einto(),

            /* 8) Many children + padding stress */
            Row::new((0..8).map(|i| {
                let c = (i * 30 + 40) as u8;
                small_block(c, 30, 200u8.saturating_sub(c))
            }).collect())
            .spacing(6)
            .padding(Vec4::splat(16))
            .color(Color::rgb(245, 245, 220))
            .size(Size::new(Grow, Fixed(56)))
            .einto(),

            /* 9) Test clamping */
            Row::new(vec![
                Rectangle::new(Size::new(Length::Grow, Length::Fixed(24)), Color::GREEN)
                    .min(Size::new(120, 24))       // >= 120px wide, one line tall
                    .max(Size::new(300, 24))       // <= 300px wide
                    .einto(),
                Rectangle::new(Size::new(Length::Fixed(100), Length::Grow), Color::BLUE)
                    .min(Size::new(100, 60))       // at least 60px tall
                    .max(Size::new(100, 120))      // at most 120px tall
                    .einto(),
            ])
            .spacing(6)
            .padding(Vec4::splat(16))
            .color(Color::rgb(245, 245, 220))
            .size(Size::new(Length::Grow, Length::Grow))
            .einto(),

            /* 10) Transparent container background over content below */
            Column::new(vec![
                Rectangle::new(Size::new(Grow, Fixed(20)), Color::rgb(30, 200, 30)).einto(),
                Rectangle::new(Size::new(Grow, Fixed(20)), Color::rgb(30, 30, 200)).einto(),
            ])
            .spacing(6)
            .padding(Vec4::splat(10))
            .color(Color::TRANSPARENT)
            .size(Size::new(Grow, Fixed(60)))
            .einto(),

            /* 11) Container with background, padding, and a single child */
            Container::new(vec![
                Rectangle::new(Size::new(Grow, Grow), Color::rgb(220, 240, 255)).einto(),
                Rectangle::new(Size::new(Fixed(60), Fixed(60)), Color::rgb(255, 0, 0)).einto(),
            ])
            .padding(Vec4::splat(10))
            .color(Color::rgb(210, 210, 210))
            .size(Size::new(Grow, Fixed(60)))
            .einto(),

        ])
        .color(Color::rgb(100, 80, 100))
        .padding(Vec4::splat(16))
        .spacing(14)
        .size(Size::new(Grow, Grow))
        .einto()
    }
}

pub mod interaction {

    use super::*;

    pub fn view(state: &State) -> Element<Message> {
        use Length::{Fit, Fixed, Grow};

        Column::new(vec![
            /* 1) interactive button */
            Row::new(vec![
                Button::new(Size::new(Fixed(120), Fixed(36)), Color::rgb(200, 50, 50))
                    .hover_color(Color::rgb(50, 200, 50))
                    .pressed_color(Color::rgb(50, 50, 200))
                    .on_press(Message::ButtonPressed)
                    .einto(),
                Row::new(
                    (0..(state.counter % 6))
                        .map(|i| {
                            let c = (i * 30 + 40) as u8;
                            small_block(c, 30, 200u8.saturating_sub(c))
                        })
                        .collect(),
                )
                .color(Color::TRANSPARENT)
                .size(Size::new(Fit, Grow))
                .einto(),
            ])
            .padding(Vec4::splat(10))
            .spacing(10)
            .color(Color::rgb(220, 220, 240))
            .size(Size::new(Grow, Fixed(60)))
            .einto(),
        ])
        .color(Color::rgb(100, 80, 100))
        .padding(Vec4::splat(16))
        .spacing(14)
        .size(Size::new(Grow, Grow))
        .einto()
    }
}

pub mod pipeline {

    use super::*;
    use ui::widget::SimpleCanvas;

    pub fn view(_state: &State) -> Element<Message> {
        use Length::{Fixed, Grow};

        Container::new(vec![
            SimpleCanvas::new(
                Size::new(Grow, Grow),
                "planet",
                Some(|cx| {
                    cx.ui.request_redraw();
                }),
            )
            .einto(),
            Column::new(vec![
                Rectangle::new(Size::new(Fixed(70), Fixed(20)), Color::rgb(100, 0, 100)).einto(),
                Rectangle::new(Size::new(Fixed(40), Fixed(30)), Color::rgb(140, 0, 140)).einto(),
            ])
            .spacing(10)
            .padding(Vec4::splat(10))
            .color(Color::rgba(220, 240, 240, 1))
            .size(Size::new(Fixed(70), Fixed(80)))
            .einto(),
        ])
        .color(Color::rgb(20, 20, 40))
        .padding(Vec4::splat(0))
        .size(Size::new(Grow, Grow))
        .einto()
    }
}

pub mod texture {

    use super::*;
    use ui::widget::Image;

    pub fn view(state: &State) -> Element<Message> {
        use Length::{Fixed, Grow};

        let mut rows: Vec<Element<Message>> = Vec::new();
        for chunk in state.icons.chunks(25) {
            let mut cells = Vec::new();
            for &h in chunk {
                cells.push(
                    Image::new(Size::new(Fixed(48), Fixed(48)), h)
                        .tint(Color::WHITE)
                        .einto(),
                );
            }
            rows.push(
                Row::new(cells)
                    .spacing(8)
                    .padding(Vec4::splat(8))
                    .size(Size::new(Grow, Fixed(64)))
                    .einto(),
            );
        }

        Container::new(vec![
            Image::new(Size::new(Grow, Grow), state.background.unwrap_or_default()).einto(),
            Column::new(vec![
                Rectangle::new(Size::new(Fixed(70), Fixed(20)), Color::rgb(100, 0, 100)).einto(),
                Rectangle::new(Size::new(Fixed(40), Fixed(30)), Color::rgb(140, 0, 140)).einto(),
            ])
            .spacing(10)
            .padding(Vec4::splat(10))
            .color(Color::rgba(220, 240, 240, 1))
            .size(Size::new(Fixed(70), Fixed(80)))
            .einto(),
            Container::new(vec![
                Column::new(rows)
                    .spacing(8)
                    .padding(Vec4::splat(10))
                    .color(Color::splat(204))
                    .size(Size::new(Grow, Grow))
                    .einto(),
            ])
            .padding(Vec4::splat(120))
            .size(Size::new(Grow, Grow))
            .einto(),
        ])
        .padding(Vec4::splat(0))
        .size(Size::new(Grow, Grow))
        .einto()
    }
}

pub mod text {
    use super::*;
    use cosmic_text::Weight;
    use ui::widget::{Spacer, Text};

    pub fn view(_state: &State) -> Element<Message> {
        use Length::{Fit, Fixed, Grow};

        // Colors
        let bg_app = Color::rgb(24, 26, 32);
        let bg_panel = Color::rgb(34, 38, 46);
        let bg_panel_alt = Color::rgb(40, 44, 54);
        let fg_title = Color::rgb(235, 240, 255);
        let fg_text = Color::rgb(210, 215, 230);
        let accent = Color::rgb(88, 146, 255);

        // --- Sidebar (fixed width) ---
        let sidebar = Column::new(vec![
            // Sidebar header
            Container::new(vec![
                Text::new("Project Nimbus", 20.0).color(fg_title).einto(),
            ])
            .padding(Vec4::new(16, 16, 16, 8))
            .color(Color::TRANSPARENT)
            .size(Size::new(Grow, Fixed(40)))
            .einto(),
            // Sidebar items
            Column::new(vec![
                Text::new("Overview", 16.0).color(fg_text).einto(),
                Text::new("Assets", 16.0).color(fg_text).einto(),
                Text::new("Settings", 16.0).color(fg_text).einto(),
            ])
            .spacing(8)
            .padding(Vec4::new(16, 8, 16, 16))
            .color(Color::TRANSPARENT)
            .size(Size::new(Grow, Fit))
            .einto(),
        ])
        .spacing(6)
        .padding(Vec4::splat(8))
        .color(bg_panel)
        .size(Size::new(Fixed(220), Grow))
        .einto();

        // --- Top bar (fixed height) ---
        let topbar = Row::new(vec![
            Text::new("Dashboard", 22.0).color(fg_title).einto(),
            Spacer::new(Size::new(Grow, Fit)).einto(),
            // a little “pill” on the right
            Container::new(vec![Text::new("LIVE", 14.0).weight(Weight::BLACK).einto()])
                .padding(Vec4::new(10, 6, 10, 6))
                .color(accent)
                .size(Size::new(Fit, Fit))
                .einto(),
        ])
        .spacing(12)
        .padding(Vec4::new(16, 10, 16, 10))
        .color(bg_panel_alt)
        .size(Size::new(Grow, Fixed(52)))
        .einto();

        // --- Main content ---
        let hero_text = "This area demonstrates styled, multiline text using cosmic-text. \n\
            The grey rectangle below acts as an image/preview placeholder. \n\
            Resize the window to see wrapping and layout negotiation.";

        let content = Column::new(vec![
            // Title
            Text::new("Welcome to the Showcase", 20.0)
                .size(Size::new(Grow, Fit))
                .color(fg_title)
                .einto(),
            // Body (multiline)
            Text::new(hero_text, 16.0)
                .size(Size::new(Grow, Fit))
                .color(fg_text)
                .einto(),
            // Image/preview placeholder
            Rectangle::new(Size::new(Grow, Fixed(240)), Color::rgb(72, 78, 90)).einto(),
            // A couple of stat tiles
            Row::new(vec![
                Column::new(vec![
                    Text::new("Builds", 16.0).color(fg_text).einto(),
                    Text::new("128", 28.0).color(fg_title).einto(),
                ])
                .padding(Vec4::splat(12))
                .color(bg_panel)
                .size(Size::new(Grow, Fixed(88)))
                .einto(),
                Column::new(vec![
                    Text::new("Warnings", 16.0).color(fg_text).einto(),
                    Text::new("3", 28.0).color(Color::rgb(255, 206, 86)).einto(),
                ])
                .padding(Vec4::splat(12))
                .color(bg_panel)
                .size(Size::new(Grow, Fixed(88)))
                .einto(),
                Column::new(vec![
                    Text::new("Errors", 16.0).color(fg_text).einto(),
                    Text::new("0", 28.0).color(Color::rgb(76, 217, 100)).einto(),
                ])
                .padding(Vec4::splat(12))
                .color(bg_panel)
                .size(Size::new(Grow, Fixed(88)))
                .einto(),
            ])
            .spacing(12)
            .padding(Vec4::splat(0))
            .color(Color::TRANSPARENT)
            .size(Size::new(Grow, Fit))
            .einto(),
        ])
        .spacing(12)
        .padding(Vec4::new(16, 16, 16, 16))
        .color(Color::TRANSPARENT)
        .size(Size::new(Grow, Grow))
        .einto();

        // --- Page layout: sidebar | (topbar + content) ---
        Row::new(vec![
            sidebar,
            Column::new(vec![topbar, content])
                .spacing(12)
                .padding(Vec4::splat(0))
                .color(Color::TRANSPARENT)
                .size(Size::new(Grow, Grow))
                .einto(),
        ])
        .spacing(12)
        .padding(Vec4::splat(12))
        .color(bg_app)
        .size(Size::new(Grow, Grow))
        .einto()
    }
}
