use super::*;
use ui::widget::SimpleCanvas;

pub fn view(tid: &TargetId, state: &State) -> Element {
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
