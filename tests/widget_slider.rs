//! Slider `handle` state machine tests.

#[cfg(test)]
mod common;

#[cfg(test)]
mod widget_slider {
    use super::common::*;

    use ui::event::{KeyState, MouseButton, UiEventRef};
    use ui::model::{Position, Size};
    use ui::widget::{Length, Slider};

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Changed(f32),
    }

    /// Build a Slider, run a layout pass to assign engine geometry/identity, return
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

    /// Dispatch a Pressed/Released `MouseButton` event for `b`. The bit
    /// fields on `harness.ctx` must already be set up by the caller --
    /// this mirrors what `handle_platform_event` does in the real engine:
    /// the bitfield + the event are both presented to the widget.
    fn press(harness: &mut Harness, root: &mut Slider<TopMsg>, b: MouseButton) {
        harness.ctx.mouse_buttons_pressed = 1 << b.bit();
        harness.ctx.mouse_buttons_down = 1 << b.bit();
        harness.handle_event(
            root,
            UiEventRef::MouseButton {
                button: b,
                state: KeyState::Pressed,
            },
        );
        harness.ctx.mouse_buttons_pressed = 0;
    }

    fn release(harness: &mut Harness, root: &mut Slider<TopMsg>, b: MouseButton) {
        harness.ctx.mouse_buttons_pressed = 0;
        harness.ctx.mouse_buttons_down = 0;
        harness.ctx.mouse_buttons_released = 1 << b.bit();
        harness.handle_event(
            root,
            UiEventRef::MouseButton {
                button: b,
                state: KeyState::Released,
            },
        );
        harness.ctx.mouse_buttons_released = 0;
    }

    /// Cursor-moved frame while the button is held. Mirrors the real
    /// engine's CursorMoved dispatch.
    fn cursor_moved(harness: &mut Harness, root: &mut Slider<TopMsg>, pos: Position<f32>) {
        harness.ctx.mouse_pos = pos;
        harness.handle_event(root, UiEventRef::CursorMoved { position: pos });
    }

    /// Drain the message queue and assert exactly one Changed(v); return v.
    fn expect_single_change(harness: &mut Harness) -> f32 {
        let msgs = harness.drain_messages();
        assert_eq!(
            msgs.len(),
            1,
            "expected exactly one Changed message, got {msgs:?}"
        );
        let msg = msgs[0].get().expect("message is not the local Msg type");
        match *msg {
            Msg::Changed(v) => v,
        }
    }

    // Hover / hot tracking

    #[test]
    fn marks_hot_when_cursor_inside() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);
        harness.ctx.mouse_pos = Position::new(50.0, 15.0);
        harness.handle(&mut s);
        assert!(harness.ctx.focus.hovered().is_some());
    }

    #[test]
    fn not_hot_when_cursor_outside() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);
        harness.ctx.mouse_pos = Position::new(500.0, 500.0);
        harness.handle(&mut s);
        assert!(harness.ctx.focus.hovered().is_none());
    }

    // Press captures active_item

    #[test]
    fn press_inside_captures_active() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);
        harness.ctx.mouse_pos = Position::new(25.0, 15.0);
        press(&mut harness, &mut s, MouseButton::Left);
        assert!(harness.ctx.focus.pressed().is_some());
    }

    #[test]
    fn press_outside_does_not_capture() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);
        harness.ctx.mouse_pos = Position::new(500.0, 500.0);
        press(&mut harness, &mut s, MouseButton::Left);
        assert!(harness.ctx.focus.pressed().is_none());
    }

    // Pressing on the track (not the knob) jumps value to cursor.
    //
    // Slider w=100, h=30, track_h=6. knob_size = (6*2).clamp(10,30) = 12.
    // Value track spans x in [6, 94], width 88. Cursor x=25 puts the
    // *click* clearly off the knob (knob at value=50 sits centred on
    // x=50, hit region [44, 56]). On press, we jump-to-cursor:
    //   t = (25 - 6) / 88
    //   v = 0 + t * 100 ≈ 21.59
    #[test]
    fn press_on_track_jumps_value_to_cursor() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);

        harness.ctx.mouse_pos = Position::new(25.0, 15.0);
        press(&mut harness, &mut s, MouseButton::Left);

        let v = expect_single_change(&mut harness);
        let expected = (25.0 - 6.0) / 88.0 * 100.0;
        assert!((v - expected).abs() < 1e-3, "expected ~{expected}, got {v}",);
    }

    // Pressing *on the knob* must NOT teleport the value -- it stores a
    // grab offset and waits for the cursor to actually move. This is
    // the central anti-regression for the "you can't drag the knob"
    // bug.
    #[test]
    fn press_on_knob_does_not_jump_value() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);

        // Knob is centred at value=50 -> x = 6 + 0.5 * 88 = 50. Hit
        // region is [44, 56]. We grab off-centre at x=53.
        harness.ctx.mouse_pos = Position::new(53.0, 15.0);
        press(&mut harness, &mut s, MouseButton::Left);

        assert!(
            harness.ctx.focus.pressed().is_some(),
            "knob press must still capture active_item"
        );
        assert!(
            harness.message_sink.drain().is_empty(),
            "pressing the knob must not emit a value change",
        );
    }

    // Grabbing the knob off-centre and dragging keeps the grab offset:
    // after pressing at x=53 (3px right of knob centre), moving the
    // cursor to x=70 should put the knob centre at 70-3=67, i.e.
    //   t = (67 - 6) / 88
    //   v = t * 100 ≈ 69.32
    // Not 70.
    #[test]
    fn drag_preserves_grab_offset() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);

        harness.ctx.mouse_pos = Position::new(53.0, 15.0);
        press(&mut harness, &mut s, MouseButton::Left);
        assert!(harness.message_sink.drain().is_empty());

        cursor_moved(&mut harness, &mut s, Position::new(70.0, 15.0));

        let v = expect_single_change(&mut harness);
        let expected = (70.0 - 3.0 - 6.0) / 88.0 * 100.0;
        assert!((v - expected).abs() < 1e-3, "expected ~{expected}, got {v}",);
    }

    // Clamping at the right edge: with grab offset preserved, the knob
    // can drag fully to `hi` (its centre reaches x = w - kw/2 = 94).
    #[test]
    fn drag_far_past_right_edge_clamps_to_hi() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 0.0, true);

        // Press at x=10. Knob is at value=0 -> centre x=6, hit region
        // [0, 12]. So x=10 is on the knob; grab offset = 4.
        harness.ctx.mouse_pos = Position::new(10.0, 15.0);
        press(&mut harness, &mut s, MouseButton::Left);
        let _ = harness.message_sink.drain();

        // Drag way past the right edge; t should clamp to 1.0, value=100.
        cursor_moved(&mut harness, &mut s, Position::new(999.0, 15.0));

        let v = expect_single_change(&mut harness);
        assert!(
            (v - 100.0).abs() < 1e-3,
            "clamped at hi, expected 100.0, got {v}"
        );
    }

    #[test]
    fn drag_far_past_left_edge_clamps_to_lo() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);

        // Press at x=50 (on the knob).
        harness.ctx.mouse_pos = Position::new(50.0, 15.0);
        press(&mut harness, &mut s, MouseButton::Left);
        let _ = harness.message_sink.drain();

        cursor_moved(&mut harness, &mut s, Position::new(-500.0, 15.0));

        let v = expect_single_change(&mut harness);
        assert!(v.abs() < 1e-3, "clamped at lo, expected 0.0, got {v}");
    }

    #[test]
    fn drag_off_vertically_still_tracks_x() {
        // Only x controls value. Dragging off vertically while holding
        // the button must keep updating value from x.
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 0.0, true);

        // Press at x=10 (on the knob centred at x=6, hit region [0,12]).
        // Grab offset = 10 - 6 = 4.
        harness.ctx.mouse_pos = Position::new(10.0, 15.0);
        press(&mut harness, &mut s, MouseButton::Left);
        let _ = harness.message_sink.drain();

        // Drag to x=80 but y=500 (far below slider).
        cursor_moved(&mut harness, &mut s, Position::new(80.0, 500.0));

        let v = expect_single_change(&mut harness);
        // Knob centre lands at 80 - 4 = 76; t = (76-6)/88; v = t * 100.
        let expected = (80.0 - 4.0 - 6.0) / 88.0 * 100.0;
        assert!((v - expected).abs() < 1e-3, "expected ~{expected}, got {v}",);
    }

    // Release

    #[test]
    fn release_clears_active_item() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);

        harness.ctx.mouse_pos = Position::new(30.0, 15.0);
        press(&mut harness, &mut s, MouseButton::Left);
        assert!(harness.ctx.focus.pressed().is_some());
        let _ = harness.message_sink.drain();

        harness.ctx.mouse_pos = Position::new(75.0, 15.0);
        release(&mut harness, &mut s, MouseButton::Left);

        assert!(
            harness.ctx.focus.pressed().is_none(),
            "active must clear on release"
        );
    }

    // No-press / no-handler paths

    #[test]
    fn no_press_no_emit() {
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, true);
        harness.ctx.mouse_pos = Position::new(50.0, 15.0);
        harness.handle(&mut s);
        assert!(harness.message_sink.drain().is_empty());
    }

    #[test]
    fn slider_without_on_change_is_silent() {
        // No on_change handler => press on the track still updates the
        // internal value (it's the source of truth) but no message is
        // emitted. A redraw is still requested.
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 100.0), 50.0, false);

        harness.ctx.mouse_pos = Position::new(25.0, 15.0);
        press(&mut harness, &mut s, MouseButton::Left);

        assert!(
            harness.message_sink.drain().is_empty(),
            "no handler => no emit"
        );
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
        // x=94 (the right end of the value track for a 100-wide slider),
        // new_v = 10. If the stored value is already 10, no Changed fires.
        let (mut s, mut harness) = laid_out_slider(100, (0.0, 10.0), 999.0, true);

        // x=94 is exactly the right edge of the value track. Knob is at
        // value=10 -> centre x=94. So this is a knob press, not a track
        // jump, and even on drag the value wouldn't move (already at hi).
        harness.ctx.mouse_pos = Position::new(94.0, 15.0);
        press(&mut harness, &mut s, MouseButton::Left);

        assert!(
            harness.message_sink.drain().is_empty(),
            "initial should already be clamped to hi, so no Changed expected"
        );
    }
}
