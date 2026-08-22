use super::*;

use Length::{Fill, Fit, Fixed};

pub fn view(state: &State) -> Element {
    let t = &state.theme;

    let main = Column::new(vec![
        view_nine_anchors(t),
        view_origin_matters(t),
        view_centre_point(t),
        view_offset_pixels(t),
        view_single_edge(t),
        view_both_edges(t),
        view_stack_align(t),
        view_absolute_anywhere(t),
        view_cross_self(t),
    ])
    .padding(Vec4::splat(space::LG))
    .spacing(space::MD)
    .size(Size::new(Fill(1.0), Fit));

    Scrollable::new(main)
        .size(Size::new(Fill(1.0), Fill(1.0)))
        .into()
}

//  helpers

/// A labelled block. The label says what placed it, so a screenshot of this
/// demo doubles as documentation.
fn tag(label: &'static str, c: Color, t: &Theme) -> Element {
    Column::new(el![Text::new(label)
        .wrap(Wrap::None)
        .color(t.on_primary)
        .size(Size::new(Fit, Fit)),])
    .padding(Vec4::new(space::SM, space::XS, space::SM, space::XS))
    .color(c)
    .size(Size::new(Fit, Fit))
    .into()
}

/// The canvas every example draws inside, so positions are comparable.
fn canvas(children: Vec<Element>, t: &Theme) -> Element {
    Stack::new(children)
        .size(Size::new(Fill(1.0), Fixed(140)))
        .color(t.surface_variant)
        .into()
}

fn section(title: &'static str, body: Element, t: &Theme) -> Element {
    Column::new(el![
        Text::new(title)
            .color(t.on_surface_variant)
            .size(Size::new(Fill(1.0), Fit)),
        body,
    ])
    .spacing(space::XS)
    .size(Size::new(Fill(1.0), Fit))
    .into()
}

// examples

/// 1) All nine anchors, each with a matching origin so the block tucks into
///    the corner rather than straddling it.
fn view_nine_anchors(t: &Theme) -> Element {
    let a = [
        ("top-left", Align2::TOP_LEFT),
        ("top-centre", Align2::TOP_CENTER),
        ("top-right", Align2::TOP_RIGHT),
        ("centre-left", Align2::CENTER_LEFT),
        ("centre", Align2::CENTER),
        ("centre-right", Align2::CENTER_RIGHT),
        ("bottom-left", Align2::BOTTOM_LEFT),
        ("bottom-centre", Align2::BOTTOM_CENTER),
        ("bottom-right", Align2::BOTTOM_RIGHT),
    ];
    let kids: Vec<Element> = a
        .iter()
        .enumerate()
        .map(|(i, (name, al))| tag(name, swatch(i), t).pinned(*al).into())
        .collect();

    section("pinned(..): anchor and origin together", canvas(kids, t), t)
}

/// 2) Same anchor, three origins. Shows that anchor picks the point on the
///    parent and origin picks the point on the child.
fn view_origin_matters(t: &Theme) -> Element {
    let kids = el![
        tag("origin TL", swatch(0), t).at(Align2::CENTER, Align2::TOP_LEFT),
        tag("origin C", swatch(3), t).at(Align2::CENTER, Align2::CENTER),
        tag("origin BR", swatch(5), t).at(Align2::CENTER, Align2::BOTTOM_RIGHT),
    ];
    section(
        "at(anchor, origin): one anchor, three origins",
        canvas(kids, t),
        t,
    )
}

/// 3) A badge whose own centre sits on the parent's corner, so it hangs half outside.
fn view_centre_point(t: &Theme) -> Element {
    let kids = el![
        Rectangle::new(Size::new(Fixed(120), Fixed(80)), t.surface).pinned(Align2::CENTER),
        tag("centre on corner", swatch(6), t).at(Align2::TOP_RIGHT, Align2::CENTER),
    ];
    section(
        "at(TOP_RIGHT, CENTER): the child hangs outside the anchor",
        canvas(kids, t),
        t,
    )
}

/// 4) Plain pixel placement: the direct replacement for the old
///    `Overlay::push(el, x, y)`.
fn view_offset_pixels(t: &Theme) -> Element {
    let kids = el![
        tag("offset(10, 20)", swatch(1), t).offset(10, 20),
        tag("offset(200, 60)", swatch(4), t).offset(200, 60),
        tag("BR + offset(-12, -12)", swatch(2), t)
            .pinned(Align2::BOTTOM_RIGHT)
            .offset(-12, -12),
    ];
    section(
        "offset(x, y): pixels, optionally measured from an anchor",
        canvas(kids, t),
        t,
    )
}

/// 5) One pinned edge fixes that axis; the other axis still anchors.
fn view_single_edge(t: &Theme) -> Element {
    let kids = el![
        tag("left(24), y centred", swatch(0), t)
            .pinned(Align2::CENTER)
            .edges(Edges::NONE.with_left(24)),
        tag("right(24), y centred", swatch(5), t)
            .pinned(Align2::CENTER)
            .edges(Edges::NONE.with_right(24)),
        tag("left(0) is flush", swatch(7), t)
            .pinned(Align2::BOTTOM_CENTER)
            .edges(Edges::NONE.with_left(0)),
    ];
    section(
        "edges: one side pinned, the other axis still anchored",
        canvas(kids, t),
        t,
    )
}

/// 6) Both edges of an axis derive the size, so even a `Fit` child stretches.
fn view_both_edges(t: &Theme) -> Element {
    let kids = el![
        Rectangle::new(Size::new(Fit, Fixed(28)), swatch(3))
            .edges(Edges::horizontal(16).with_top(12)),
        Rectangle::new(Size::new(Fit, Fit), swatch(6)).edges(Edges::all(56)),
        tag("Edges::all(56) stretches Fit", swatch(6), t).pinned(Align2::CENTER),
    ];
    section(
        "edges: both sides pinned derives the size",
        canvas(kids, t),
        t,
    )
}

/// 7) `Stack::align` sets the default for children that didn't ask, and does
///    not disturb ones that did.
fn view_stack_align(t: &Theme) -> Element {
    let body = Row::new(el![
        Stack::new(el![
            tag("default", swatch(0), t),
            tag("explicit TR", swatch(5), t).pinned(Align2::TOP_RIGHT),
        ])
        .align(Align2::CENTER)
        .size(Size::new(Fill(1.0), Fixed(120)))
        .color(t.surface_variant),
        Stack::new(el![
            tag("default", swatch(0), t),
            tag("offset(8, 8)", swatch(2), t).offset(8, 8),
        ])
        .align(Align2::BOTTOM_CENTER)
        .size(Size::new(Fill(1.0), Fixed(120)))
        .color(t.surface_variant),
    ])
    .spacing(space::MD)
    .size(Size::new(Fill(1.0), Fit));

    section(
        "Stack::align: a default that explicit placement overrides",
        body.into(),
        t,
    )
}

/// 8) Placement lives on `Node`, so any container hosts absolute children
fn view_absolute_anywhere(t: &Theme) -> Element {
    let body = Row::new(el![
        Card::new(el![
            Text::new("a Card with a badge"),
            tag("badge", swatch(6), t).at(Align2::TOP_RIGHT, Align2::CENTER),
        ])
        .padding(Vec4::splat(space::MD))
        .size(Size::splat(Fill(1.0))),
        Column::new(el![
            Text::new("a Column, likewise"),
            Rectangle::new(Size::new(Fixed(90), Fixed(24)), swatch(3)),
            tag("pinned BR", swatch(1), t).pinned(Align2::BOTTOM_RIGHT),
        ])
        .spacing(space::SM)
        .padding(Vec4::splat(space::MD))
        .color(t.surface_variant)
        .size(Size::splat(Fill(1.0))),
    ])
    .spacing(space::MD)
    .size(Size::new(Fill(1.0), Fixed(120)));

    section(
        "any container can host absolute children now",
        body.into(),
        t,
    )
}

/// 9) `cross_self`: one child opting out of the row's cross alignment.
fn view_cross_self(t: &Theme) -> Element {
    let body = Row::new(el![
        Rectangle::new(Size::new(Fixed(60), Fixed(30)), swatch(0)),
        Rectangle::new(Size::new(Fixed(60), Fixed(30)), swatch(3)).cross_self(Align::START),
        Rectangle::new(Size::new(Fixed(60), Fixed(30)), swatch(5)).cross_self(Align::END),
        Rectangle::new(Size::new(Fixed(60), Fixed(30)), swatch(6)),
    ])
    .cross(Align::CENTER)
    .spacing(space::SM)
    .padding(Vec4::splat(space::SM))
    .color(t.surface_variant)
    .size(Size::new(Fill(1.0), Fixed(100)));

    section(
        "cross_self: children 2 and 3 opt out of cross(CENTER)",
        body.into(),
        t,
    )
}
