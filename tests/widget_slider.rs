//! Slider `handle` state machine tests.

#[cfg(test)]
mod common;

#[cfg(test)]
mod widget_slider {
    use super::common::*;

    use ui::event::MouseButton;
    use ui::model::{Position, Size};
    use ui::widget::{Length, Slider};

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Changed(f32),
    }

    /// Build a Slider, run a layout pass so set_layout/set_id fire, return
    /// the slider plus a fresh harness.
    fn laid_out_slider(
        width: i32,
        range: (f32, f32),
        initial: f32,
        with_handler: bool,
    ) -> (Slider<TopMsg>, Harness) {
        let mut harness = Harness::default();

        let mut slider: Slider<TopMsg> = Slider::new(
            Size::new(Length::Fixed(width), Length::Fixed(30)),
            range,
            initial,
        );
        if with_handler {
            slider = slider.on_change(|v| TopMsg::from(Msg::Changed(v)));
        }

        harness.layout(&mut slider, 1000, 1000);

        (slider, harness)
    }

    /// Drain the message queue and assert exactly one Changed(v); return v.
    fn expect_single_change(harness: &mut Harness) -> f32 {
        let msgs = harness.ctx.take();
        assert_eq!(
            msgs.len(),
            1,
            "expected exactly one Changed message, got {msgs:?}"
        );
        // Pull the inner f32 out of TopMsg::Any -> Msg::Changed(f).
        match &msgs[0] {
            TopMsg::Any(arc) => {
                let m = arc
                    .as_any()
                    .downcast_ref::<Msg>()
                    .expect("message is not the local Msg type");
                match *m {
                    Msg::Changed(v) => v,
                }
            }
        }
    }

    // Hover / hot tracking

    #[test]
    fn marks_hot_when_cursor_inside() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);
        harness.ctx.mouse_pos = Position::new(50.0, 15.0);
        harness.handle(&mut s);
        assert!(harness.ctx.hot_item.is_some());
    }

    #[test]
    fn not_hot_when_cursor_outside() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);
        harness.ctx.mouse_pos = Position::new(500.0, 500.0);
        harness.handle(&mut s);
        assert!(harness.ctx.hot_item.is_none());
    }

    // Press captures active_item

    #[test]
    fn press_inside_captures_active() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);
        harness.ctx.mouse_pos = Position::new(25.0, 15.0);
        harness.ctx.mouse_buttons_pressed = 1 << MouseButton::Left.bit();
        harness.ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        harness.handle(&mut s);
        assert!(harness.ctx.active_item.is_some());
    }

    #[test]
    fn press_outside_does_not_capture() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);
        harness.ctx.mouse_pos = Position::new(500.0, 500.0);
        harness.ctx.mouse_buttons_pressed = 1 << MouseButton::Left.bit();
        harness.ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        harness.handle(&mut s);
        assert!(harness.ctx.active_item.is_none());
    }

    // Press at known cursor x emits proportional value

    #[test]
    fn press_at_quarter_emits_quartile_value() {
        // Slider width=100, range=(0,100): pressing at x=25 gives t=0.25,
        // new_v=25.0. Initial was 50.0, so value changes and emits.
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);

        harness.ctx.mouse_pos = Position::new(25.0, 15.0);
        harness.ctx.mouse_buttons_pressed = 1 << MouseButton::Left.bit();
        harness.ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        harness.handle(&mut s);

        let v = expect_single_change(&mut harness);
        assert!((v - 25.0).abs() < 1e-4, "expected ~25.0, got {v}");
    }

    #[test]
    fn drag_far_past_right_edge_clamps_to_hi() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 0.0, true);

        // Press inside.
        harness.ctx.mouse_pos = Position::new(10.0, 15.0);
        harness.ctx.mouse_buttons_pressed = 1 << MouseButton::Left.bit();
        harness.ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        harness.handle(&mut s);
        let _ = harness.ctx.take(); // drain Changed(10.0) from press-frame

        // Drag way past the right edge; t should clamp to 1.0, value=100.
        harness.ctx.mouse_pos = Position::new(999.0, 15.0);
        harness.ctx.mouse_buttons_pressed = 0;
        harness.handle(&mut s);

        let v = expect_single_change(&mut harness);
        assert!(
            (v - 100.0).abs() < 1e-4,
            "clamped at hi, expected 100.0, got {v}"
        );
    }

    #[test]
    fn drag_far_past_left_edge_clamps_to_lo() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);

        harness.ctx.mouse_pos = Position::new(50.0, 15.0);
        harness.ctx.mouse_buttons_pressed = 1 << MouseButton::Left.bit();
        harness.ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        harness.handle(&mut s);
        let _ = harness.ctx.take();

        harness.ctx.mouse_pos = Position::new(-500.0, 15.0);
        harness.ctx.mouse_buttons_pressed = 0;
        harness.handle(&mut s);

        let v = expect_single_change(&mut harness);
        assert!(v.abs() < 1e-4, "clamped at lo, expected 0.0, got {v}");
    }

    #[test]
    fn drag_off_vertically_still_tracks_x() {
        // Only x controls value. Dragging off vertically while holding
        // the button must keep updating value from x.
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 0.0, true);

        // Press inside at x=10.
        harness.ctx.mouse_pos = Position::new(10.0, 15.0);
        harness.ctx.mouse_buttons_pressed = 1 << MouseButton::Left.bit();
        harness.ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        harness.handle(&mut s);
        let _ = harness.ctx.take();

        // Drag to x=80 but y=500 (far below slider).
        harness.ctx.mouse_pos = Position::new(80.0, 500.0);
        harness.ctx.mouse_buttons_pressed = 0;
        harness.handle(&mut s);

        let v = expect_single_change(&mut harness);
        assert!((v - 80.0).abs() < 1e-4, "expected ~80.0, got {v}");
    }

    // Release

    #[test]
    fn release_clears_active_item() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);

        harness.ctx.mouse_pos = Position::new(30.0, 15.0);
        harness.ctx.mouse_buttons_pressed = 1 << MouseButton::Left.bit();
        harness.ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        harness.handle(&mut s);
        assert!(harness.ctx.active_item.is_some());
        let _ = harness.ctx.take();

        harness.ctx.mouse_buttons_pressed = 0;
        harness.ctx.mouse_buttons_down = 0;
        harness.ctx.mouse_buttons_released = 1 << MouseButton::Left.bit();
        harness.ctx.mouse_pos = Position::new(75.0, 15.0);
        harness.handle(&mut s);

        assert!(
            harness.ctx.active_item.is_none(),
            "active must clear on release"
        );
    }

    // No-press / no-handler paths

    #[test]
    fn no_press_no_emit() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);
        harness.ctx.mouse_pos = Position::new(50.0, 15.0);
        harness.handle(&mut s);
        assert!(harness.ctx.take().is_empty());
    }

    #[test]
    fn slider_without_on_change_is_silent() {
        // No on_change handler => dragging still updates internal value
        // (it's the source of truth) but no message is emitted. A redraw
        // is still requested when the value actually changed.
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, false);

        harness.ctx.mouse_pos = Position::new(25.0, 15.0);
        harness.ctx.mouse_buttons_pressed = 1 << MouseButton::Left.bit();
        harness.ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        harness.handle(&mut s);

        assert!(harness.ctx.take().is_empty(), "no handler => no emit");
        assert!(
            harness.ctx.take_redraw(),
            "value actually changed => redraw requested"
        );
    }

    // Constructor clamps initial value

    #[test]
    fn constructor_clamps_initial_value() {
        // value=999 with range (0, 10) should clamp to 10. We can't read
        // `value` directly (private), so observational: with cursor at
        // x=100 in a 100-wide slider with range (0, 10), new_v = 10.
        // If the stored value is already 10, no Changed fires. If
        // clamping silently failed and value stayed at 999, dragging to
        // 10 would emit Changed(10.0).
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 10.0), 999.0, true);

        harness.ctx.mouse_pos = Position::new(100.0, 15.0);
        harness.ctx.mouse_buttons_pressed = 1 << MouseButton::Left.bit();
        harness.ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        harness.handle(&mut s);

        assert!(
            harness.ctx.take().is_empty(),
            "initial should already be clamped to hi, so no Changed expected"
        );
    }
}
