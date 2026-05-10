//! Integration tests for `Grid` construction.

#[cfg(test)]
mod common;

#[cfg(test)]
mod view_state_sweep {
    use super::common::*;

    use ui::event::{ScrollDelta, ScrollUnits, UiEventRef};
    use ui::model::{Color, Position, Size};
    use ui::widget::{Length, Rectangle, Scrollable};

    /// Run a frame on `root`: layout, handle, paint, sweep.
    /// SweepCx needs a Gpu + TextureRegistry which the harness doesn't
    /// have, so this test only exercises the path where no entry has
    /// an OnSweep impl. That's fine — Scrollable's state has no
    /// resources, only data, so its sweep callback is None.
    fn frame<W: ui::widget::Widget<TopMsg>>(h: &mut Harness, root: &mut W) {
        h.layout(root, 800, 600);
        h.handle(root);
        let _ = h.paint(root);
        h.ctx.sweep_focus();
        // No sweep_cx available in unit tests — call a test-only
        // overload, see note below.
        h.ctx.view_state.sweep_for_test();
    }

    #[test]
    fn removed_scrollable_is_swept_and_re_added_one_starts_fresh() {
        let mut h = Harness::default();

        // Frame 1: scrollable present.
        let mut s1: Scrollable<TopMsg> = Scrollable::new(Rectangle::new(
            Size::new(Length::Fixed(400), Length::Fixed(2000)),
            Color::WHITE,
        ));
        frame(&mut h, &mut s1);

        // Scroll it.
        h.ctx.mouse_pos = Position::new(10.0, 10.0);
        let wheel = UiEventRef::MouseWheel(ScrollDelta {
            dx: 0.0,
            dy: -200.0,
            units: ScrollUnits::Pixels,
        });
        h.handle_event(&mut s1, wheel);
        assert!(
            s1.__scroll_y_for_test(&h.ctx.view_state) > 0,
            "scrollable should have advanced y after wheel event"
        );

        // Frame 2: render a different root. Scrollable's state must
        // be swept.
        let mut empty: Rectangle = Rectangle::new(
            Size::new(Length::Fixed(10), Length::Fixed(10)),
            Color::BLACK,
        );
        frame(&mut h, &mut empty);
        assert!(
            h.ctx.view_state.is_empty(),
            "removed scrollable's state must be swept"
        );

        // Frame 3: scrollable re-added at the same tree position.
        // Must start at y=0, not inherit s1's offset.
        let mut s2: Scrollable<TopMsg> = Scrollable::new(Rectangle::new(
            Size::new(Length::Fixed(400), Length::Fixed(2000)),
            Color::WHITE,
        ));
        frame(&mut h, &mut s2);
        assert_eq!(
            s2.__scroll_y_for_test(&h.ctx.view_state),
            0,
            "re-added scrollable must not inherit predecessor's scroll offset"
        );
    }
}
