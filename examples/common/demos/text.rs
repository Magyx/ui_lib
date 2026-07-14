use super::*;

pub fn view(state: &State) -> Element<Message> {
    use Length::{Fit, Fixed, Grow, Weighted};

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
    .size(Size::new(Weighted(0.2), Grow));

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
