//! Integration tests for `Button` construction.

#[cfg(test)]
mod common;

#[cfg(test)]
mod widget_interaction {
    use super::common::*;

    use ui::event::{KeyState, MouseButton, UiEventRef};
    use ui::model::{Color, Position, Size};
    use ui::widget::{Button, Length};

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Clicked(u32),
    }

    /// Build a Button and run it through a full layout pass so its set_layout
    /// is called with a known rect. Returns the Button ready for handle().
    fn laid_out_button(size_w: i32, size_h: i32, with_handler: bool) -> (Button<TopMsg>, Harness) {
        let mut harness = Harness::default();

        let mut btn: Button<TopMsg> = Button::new(
            Size::new(Length::Fixed(size_w), Length::Fixed(size_h)),
            Color::BLACK,
        );
        if with_handler {
            btn = btn.on_press(TopMsg::from(Msg::Clicked(1)));
        }

        harness.layout(&mut btn, 1000, 1000);

        (btn, harness)
    }

    // Hover tracking

    #[test]
    fn button_marks_itself_as_hot_when_cursor_is_inside() {
        let (mut btn, mut harness) = laid_out_button(100, 50, true);

        harness.ctx.mouse_pos = Position::new(10.0, 10.0);
        harness.handle(&mut btn);

        assert!(
            harness.ctx.focus.hovered().is_some(),
            "hot_item should be set when cursor is inside"
        );
    }

    #[test]
    fn button_does_not_mark_hot_when_cursor_is_outside() {
        let (mut btn, mut harness) = laid_out_button(100, 50, true);

        harness.ctx.mouse_pos = Position::new(500.0, 500.0);
        harness.handle(&mut btn);

        // Button explicitly sets hot_item only when inside; since nothing else
        // runs, it stays None.
        assert!(harness.ctx.focus.hovered().is_none());
    }

    #[test]
    fn button_hover_state_change_requests_redraw() {
        let (mut btn, mut harness) = laid_out_button(100, 50, true);

        // Start outside, first dispatch establishes baseline (not hovered).
        harness.ctx.mouse_pos = Position::new(500.0, 500.0);
        harness.handle(&mut btn);
        let _ = harness.ctx.take_redraw(); // drain any prior state

        // Move inside; hovered flips false -> true, so redraw requested.
        harness.ctx.mouse_pos = Position::new(10.0, 10.0);
        harness.handle(&mut btn);
        assert!(
            harness.ctx.take_redraw(),
            "hover transition must trigger request_redraw"
        );
    }

    #[test]
    fn button_no_hover_change_does_not_request_redraw() {
        let (mut btn, mut harness) = laid_out_button(100, 50, true);

        harness.ctx.mouse_pos = Position::new(10.0, 10.0);
        harness.handle(&mut btn);
        let _ = harness.ctx.take_redraw();

        // Same position, same pressed state: nothing changed.
        harness.handle(&mut btn);
        assert!(
            !harness.ctx.take_redraw(),
            "no state change => no redraw request"
        );
    }

    // Click sequence: press inside, release inside -> message

    #[test]
    fn click_inside_emits_on_press_message() {
        let (mut btn, mut harness) = laid_out_button(100, 50, true);

        // Frame 1: cursor in, mouse button pressed.
        harness.ctx.mouse_pos = Position::new(20.0, 20.0);
        harness.ctx.mouse_buttons_pressed = 1 << MouseButton::Left.bit();
        harness.ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        harness.handle_event(
            &mut btn,
            UiEventRef::MouseButton {
                button: MouseButton::Left,
                state: KeyState::Pressed,
            },
        );

        assert!(
            harness.ctx.focus.pressed().is_some(),
            "pressing inside should capture active_item"
        );
        assert!(
            harness.ctx.take().is_empty(),
            "press alone should not emit the message"
        );

        // Frame 2: still inside, mouse button released.
        harness.ctx.mouse_buttons_pressed = 0;
        harness.ctx.mouse_buttons_down = 0;
        harness.ctx.mouse_buttons_released = 1 << MouseButton::Left.bit();
        harness.handle_event(
            &mut btn,
            UiEventRef::MouseButton {
                button: MouseButton::Left,
                state: KeyState::Released,
            },
        );

        let msgs = harness.ctx.take();
        assert_eq!(msgs, vec![TopMsg::from(Msg::Clicked(1))]);
        assert!(
            harness.ctx.focus.pressed().is_none(),
            "active_item should be released on click completion"
        );
    }

    // Press inside, release outside -> no message

    #[test]
    fn press_inside_release_outside_does_not_emit() {
        let (mut btn, mut harness) = laid_out_button(100, 50, true);

        // Press inside.
        harness.ctx.mouse_pos = Position::new(20.0, 20.0);
        harness.ctx.mouse_buttons_pressed = 1 << MouseButton::Left.bit();
        harness.ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        harness.handle_event(
            &mut btn,
            UiEventRef::MouseButton {
                button: MouseButton::Left,
                state: KeyState::Pressed,
            },
        );
        assert!(harness.ctx.focus.pressed().is_some());

        // Drag outside still holding the button.
        harness.ctx.mouse_pos = Position::new(500.0, 500.0);
        harness.ctx.mouse_buttons_pressed = 0;
        harness.handle(&mut btn);

        // Release outside.
        harness.ctx.mouse_buttons_down = 0;
        harness.ctx.mouse_buttons_released = 1 << MouseButton::Left.bit();
        harness.handle_event(
            &mut btn,
            UiEventRef::MouseButton {
                button: MouseButton::Left,
                state: KeyState::Released,
            },
        );

        let msgs = harness.ctx.take();
        assert!(
            msgs.is_empty(),
            "release outside must not emit the message, got {:?}",
            msgs
        );
        assert!(
            harness.ctx.focus.pressed().is_none(),
            "active_item should be cleared on release regardless of location"
        );
    }

    // Press outside -> no capture, no message

    #[test]
    fn press_outside_does_not_capture_or_emit() {
        let (mut btn, mut harness) = laid_out_button(100, 50, true);

        harness.ctx.mouse_pos = Position::new(500.0, 500.0);
        harness.ctx.mouse_buttons_pressed = 1 << MouseButton::Left.bit();
        harness.ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        harness.handle(&mut btn);

        assert!(harness.ctx.focus.pressed().is_none());

        harness.ctx.mouse_buttons_pressed = 0;
        harness.ctx.mouse_buttons_down = 0;
        harness.ctx.mouse_buttons_released = 1 << MouseButton::Left.bit();
        harness.handle(&mut btn);

        assert!(harness.ctx.take().is_empty());
    }

    // Button without on_press: click is safe no-op

    #[test]
    fn button_with_no_on_press_is_silent_on_click() {
        let (mut btn, mut harness) = laid_out_button(100, 50, false);
        harness.layout(&mut btn, 1000, 1000);

        harness.ctx.mouse_pos = Position::new(20.0, 20.0);
        harness.ctx.mouse_buttons_pressed = 1 << MouseButton::Left.bit();
        harness.ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        harness.handle(&mut btn);

        harness.ctx.mouse_buttons_pressed = 0;
        harness.ctx.mouse_buttons_down = 0;
        harness.ctx.mouse_buttons_released = 1 << MouseButton::Left.bit();
        harness.handle(&mut btn);

        assert!(harness.ctx.take().is_empty());
    }

    // contains() boundary behaviour (inclusive-left/top, exclusive-right/bottom)
    //
    // Button::contains uses `p.x >= l && p.x < r`. Since button is laid out at
    // (0, 0) with size 100x50, the inclusive bounds are [0, 100) x [0, 50).

    #[test]
    fn button_hover_is_inclusive_at_top_left_corner() {
        let (mut btn, mut harness) = laid_out_button(100, 50, true);
        harness.ctx.mouse_pos = Position::new(0.0, 0.0);
        harness.handle(&mut btn);
        assert!(
            harness.ctx.focus.hovered().is_some(),
            "point (0, 0) is inside an inclusive top-left bound"
        );
    }

    #[test]
    fn button_hover_is_exclusive_at_bottom_right_corner() {
        let (mut btn, mut harness) = laid_out_button(100, 50, true);
        // The point (100.0, 50.0) is exactly on the right/bottom edge, which
        // is outside under the `p.x < r` rule.
        harness.ctx.mouse_pos = Position::new(100.0, 50.0);
        harness.handle(&mut btn);
        assert!(
            harness.ctx.focus.hovered().is_none(),
            "point (w, h) is outside an exclusive bottom-right bound"
        );
    }

    #[test]
    fn button_hover_just_inside_bottom_right_corner() {
        let (mut btn, mut harness) = laid_out_button(100, 50, true);
        harness.ctx.mouse_pos = Position::new(99.999, 49.999);
        harness.handle(&mut btn);
        assert!(harness.ctx.focus.hovered().is_some());
    }
}
