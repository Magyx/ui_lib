//! TextInput handle state machine tests.
//!
//! These test the keyboard interaction for TextField (SingleLine) and
//! TextArea (MultiLine). No GPU needed — handle only uses EventCtx.
//! We use `harness.handle_event` to pass UiEventRef::Key and
//! UiEventRef::Text events.

#[cfg(test)]
mod common;

#[cfg(test)]
mod text_input {
    use super::common::*;

    use ui::event::{
        KeyEvent, KeyLocation, KeyState, LogicalKey, Modifiers, MouseButton, PhysicalKey,
        TextInput as TextInputEvent, UiEventRef,
    };
    use ui::model::{Position, Size};
    use ui::widget::{Length, TextArea, TextField};

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Changed(String),
        Submitted(String),
    }

    // ----- helpers -----

    fn key_event(key: LogicalKey, state: KeyState) -> KeyEvent {
        KeyEvent {
            state,
            repeat: false,
            logical_key: key,
            physical_key: PhysicalKey::Unidentified,
            location: KeyLocation::Standard,
            modifiers: Modifiers::default(),
        }
    }

    fn key_press(key: LogicalKey) -> KeyEvent {
        key_event(key, KeyState::Pressed)
    }

    /// Build a TextField, layout it, focus it, return (widget, harness).
    fn focused_field(width: i32) -> (TextField<TopMsg>, Harness) {
        let mut h = Harness::default();
        let mut field: TextField<TopMsg> =
            TextField::new(Size::new(Length::Fixed(width), Length::Fixed(30)))
                .on_change(|s| TopMsg::from(Msg::Changed(s.to_string())))
                .on_submit(|s| TopMsg::from(Msg::Submitted(s.to_string())));

        h.layout(&mut field, 1000, 1000);

        // Focus: click inside.
        h.ctx.mouse_pos = Position::new(5.0, 15.0);
        h.ctx.mouse_buttons_released = 1 << MouseButton::Left.bit();
        h.handle(&mut field);
        h.ctx.mouse_buttons_released = 0;
        assert!(
            h.ctx.kbd_focus_item.is_some(),
            "field should be focused after click inside"
        );
        let _ = h.ctx.take(); // drain any messages from focus event
        let _ = h.ctx.take_redraw();

        (field, h)
    }

    /// Build a TextArea, layout it, focus it, return (widget, harness).
    fn focused_area(width: i32, height: i32) -> (TextArea<TopMsg>, Harness) {
        let mut h = Harness::default();
        let mut area: TextArea<TopMsg> =
            TextArea::new(Size::new(Length::Fixed(width), Length::Fixed(height)))
                .on_change(|s| TopMsg::from(Msg::Changed(s.to_string())));

        h.layout(&mut area, 1000, 1000);

        h.ctx.mouse_pos = Position::new(5.0, 15.0);
        h.ctx.mouse_buttons_released = 1 << MouseButton::Left.bit();
        h.handle(&mut area);
        h.ctx.mouse_buttons_released = 0;
        let _ = h.ctx.take();
        let _ = h.ctx.take_redraw();

        (area, h)
    }

    /// Drain messages and return the last Changed value (if any).
    fn last_value(h: &mut Harness) -> Option<String> {
        let msgs = h.ctx.take();
        msgs.iter()
            .filter_map(|m| m.get::<Msg>())
            .filter_map(|m| match m {
                Msg::Changed(s) => Some(s.clone()),
                _ => None,
            })
            .next_back()
    }

    // Text insertion via UiEventRef::Key(Character)

    #[test]
    fn typing_characters_via_key_events() {
        let (mut field, mut h) = focused_field(200);

        // Type "hi"
        let k1 = key_press(LogicalKey::Character("h".into()));
        h.handle_event(&mut field, UiEventRef::Key(&k1));
        let k2 = key_press(LogicalKey::Character("i".into()));
        h.handle_event(&mut field, UiEventRef::Key(&k2));

        let v = last_value(&mut h).expect("should have changed");
        assert_eq!(v, "hi");
    }

    #[test]
    fn typing_via_text_input_event() {
        let (mut field, mut h) = focused_field(200);

        let ev = TextInputEvent {
            text: "hello".to_string(),
        };
        h.handle_event(&mut field, UiEventRef::Text(&ev));

        let v = last_value(&mut h).expect("should have changed");
        assert_eq!(v, "hello");
    }

    #[test]
    fn concatenation_across_multiple_inputs() {
        let (mut field, mut h) = focused_field(200);

        let ev1 = TextInputEvent {
            text: "abc".to_string(),
        };
        h.handle_event(&mut field, UiEventRef::Text(&ev1));
        let _ = h.ctx.take();

        let ev2 = TextInputEvent {
            text: "def".to_string(),
        };
        h.handle_event(&mut field, UiEventRef::Text(&ev2));

        let v = last_value(&mut h).expect("should have changed");
        assert_eq!(v, "abcdef");
    }

    // Backspace

    #[test]
    fn backspace_removes_last_character() {
        let (mut field, mut h) = focused_field(200);

        // Type "abc"
        let ev = TextInputEvent {
            text: "abc".to_string(),
        };
        h.handle_event(&mut field, UiEventRef::Text(&ev));
        let _ = h.ctx.take();

        // Backspace
        let k = key_press(LogicalKey::Backspace);
        h.handle_event(&mut field, UiEventRef::Key(&k));

        let v = last_value(&mut h).expect("should have changed");
        assert_eq!(v, "ab");
    }

    #[test]
    fn backspace_at_empty_is_noop() {
        let (mut field, mut h) = focused_field(200);

        let k = key_press(LogicalKey::Backspace);
        h.handle_event(&mut field, UiEventRef::Key(&k));

        // No Changed message should be emitted.
        assert!(
            h.ctx.take().is_empty(),
            "backspace on empty should not emit"
        );
    }

    #[test]
    fn backspace_then_retype_produces_correct_value() {
        let (mut field, mut h) = focused_field(200);

        let ev = TextInputEvent {
            text: "abc".to_string(),
        };
        h.handle_event(&mut field, UiEventRef::Text(&ev));
        let _ = h.ctx.take();

        // Backspace removes 'c'.
        let bs = key_press(LogicalKey::Backspace);
        h.handle_event(&mut field, UiEventRef::Key(&bs));
        let v = last_value(&mut h).expect("should have changed");
        assert_eq!(v, "ab");

        // Type 'X' — should append.
        let kx = key_press(LogicalKey::Character("X".into()));
        h.handle_event(&mut field, UiEventRef::Key(&kx));
        let v = last_value(&mut h).expect("should have changed");
        assert_eq!(v, "abX");
    }

    // Delete

    #[test]
    fn delete_at_end_is_noop() {
        // Delete at end of string should do nothing — the cursor is past
        // the last character.
        let (mut field, mut h) = focused_field(200);

        let ev = TextInputEvent {
            text: "abc".to_string(),
        };
        h.handle_event(&mut field, UiEventRef::Text(&ev));
        let _ = h.ctx.take();

        let del = key_press(LogicalKey::Delete);
        h.handle_event(&mut field, UiEventRef::Key(&del));

        // No Changed should fire — nothing to delete forward.
        assert!(
            h.ctx.take().is_empty(),
            "Delete at end of string should be a no-op"
        );
    }

    // Arrow keys

    #[test]
    fn arrow_left_at_start_is_noop() {
        let (mut field, mut h) = focused_field(200);

        let ev = TextInputEvent {
            text: "x".to_string(),
        };
        h.handle_event(&mut field, UiEventRef::Text(&ev));
        let _ = h.ctx.take();
        let _ = h.ctx.take_redraw();

        // Move left twice (past the start).
        let left = key_press(LogicalKey::ArrowLeft);
        h.handle_event(&mut field, UiEventRef::Key(&left));
        h.handle_event(&mut field, UiEventRef::Key(&left)); // second is noop

        // Type 'Y' — should insert at position 0.
        let ky = key_press(LogicalKey::Character("Y".into()));
        h.handle_event(&mut field, UiEventRef::Key(&ky));

        let v = last_value(&mut h).expect("should have changed");
        assert_eq!(v, "Yx");
    }

    // Enter: SingleLine → submit, MultiLine → newline

    #[test]
    fn enter_on_single_line_emits_submit() {
        let (mut field, mut h) = focused_field(200);

        let ev = TextInputEvent {
            text: "hello".to_string(),
        };
        h.handle_event(&mut field, UiEventRef::Text(&ev));
        let _ = h.ctx.take();

        let enter = key_press(LogicalKey::Enter);
        h.handle_event(&mut field, UiEventRef::Key(&enter));

        let msgs = h.ctx.take();
        assert!(
            msgs.iter()
                .any(|m| m.get::<Msg>() == Some(&Msg::Submitted("hello".to_string()))),
            "Enter on SingleLine should emit Submitted, got {msgs:?}"
        );
    }

    #[test]
    fn enter_on_single_line_does_not_insert_newline() {
        let (mut field, mut h) = focused_field(200);

        let ev = TextInputEvent {
            text: "hi".to_string(),
        };
        h.handle_event(&mut field, UiEventRef::Text(&ev));
        let _ = h.ctx.take();

        let enter = key_press(LogicalKey::Enter);
        h.handle_event(&mut field, UiEventRef::Key(&enter));

        let msgs = h.ctx.take();
        // No Changed message — value stays "hi".
        assert!(
            !msgs
                .iter()
                .any(|m| matches!(m.get::<Msg>(), Some(Msg::Changed(_)))),
            "Enter on SingleLine should NOT emit Changed"
        );
    }

    #[test]
    fn enter_on_multi_line_inserts_newline() {
        let (mut area, mut h) = focused_area(200, 100);

        let ev = TextInputEvent {
            text: "ab".to_string(),
        };
        h.handle_event(&mut area, UiEventRef::Text(&ev));
        let _ = h.ctx.take();

        let enter = key_press(LogicalKey::Enter);
        h.handle_event(&mut area, UiEventRef::Key(&enter));

        let v = last_value(&mut h).expect("should have changed");
        assert_eq!(v, "ab\n");
    }

    // Focus / unfocus

    #[test]
    fn click_outside_unfocuses() {
        let (mut field, mut h) = focused_field(200);
        assert!(h.ctx.kbd_focus_item.is_some());

        // Click outside.
        h.ctx.mouse_pos = Position::new(500.0, 500.0);
        h.ctx.mouse_buttons_pressed = 1 << MouseButton::Left.bit();
        h.ctx.mouse_buttons_down = 1 << MouseButton::Left.bit();
        h.handle(&mut field);

        assert!(
            h.ctx.kbd_focus_item.is_none(),
            "click outside should clear focus"
        );
    }

    #[test]
    fn key_events_ignored_when_not_focused() {
        let mut h = Harness::default();
        let mut field: TextField<TopMsg> =
            TextField::new(Size::new(Length::Fixed(200), Length::Fixed(30)))
                .on_change(|s| TopMsg::from(Msg::Changed(s.to_string())));

        h.layout(&mut field, 1000, 1000);
        // Do NOT focus.

        let k = key_press(LogicalKey::Character("a".into()));
        h.handle_event(&mut field, UiEventRef::Key(&k));

        assert!(
            h.ctx.take().is_empty(),
            "unfocused field should ignore key events"
        );
    }

    // on_change fires on every modification

    #[test]
    fn on_change_fires_for_each_keystroke() {
        let (mut field, mut h) = focused_field(200);

        let ka = key_press(LogicalKey::Character("a".into()));
        h.handle_event(&mut field, UiEventRef::Key(&ka));
        assert_eq!(
            h.ctx.take(),
            vec![TopMsg::from(Msg::Changed("a".to_string()))]
        );

        let kb = key_press(LogicalKey::Character("b".into()));
        h.handle_event(&mut field, UiEventRef::Key(&kb));
        assert_eq!(
            h.ctx.take(),
            vec![TopMsg::from(Msg::Changed("ab".to_string()))]
        );

        let bs = key_press(LogicalKey::Backspace);
        h.handle_event(&mut field, UiEventRef::Key(&bs));
        assert_eq!(
            h.ctx.take(),
            vec![TopMsg::from(Msg::Changed("a".to_string()))]
        );
    }

    #[test]
    fn no_on_change_handler_means_silent() {
        let mut h = Harness::default();
        let mut field: TextField<TopMsg> =
            TextField::new(Size::new(Length::Fixed(200), Length::Fixed(30)));
        // No .on_change()

        h.layout(&mut field, 1000, 1000);

        // Focus.
        h.ctx.mouse_pos = Position::new(5.0, 15.0);
        h.ctx.mouse_buttons_released = 1 << MouseButton::Left.bit();
        h.handle(&mut field);
        h.ctx.mouse_buttons_released = 0;
        let _ = h.ctx.take();

        let ka = key_press(LogicalKey::Character("x".into()));
        h.handle_event(&mut field, UiEventRef::Key(&ka));

        assert!(h.ctx.take().is_empty(), "no handler => no message emitted");
        // But redraw still requested.
        assert!(h.ctx.take_redraw());
    }

    // Tab unfocuses

    #[test]
    fn tab_unfocuses_field() {
        let (mut field, mut h) = focused_field(200);
        assert!(h.ctx.kbd_focus_item.is_some());

        let tab = key_press(LogicalKey::Tab);
        h.handle_event(&mut field, UiEventRef::Key(&tab));

        assert!(h.ctx.kbd_focus_item.is_none(), "Tab should clear kbd focus");
    }

    // Space

    #[test]
    fn space_inserts_space_character() {
        let (mut field, mut h) = focused_field(200);

        let sp = key_press(LogicalKey::Space);
        h.handle_event(&mut field, UiEventRef::Key(&sp));

        let v = last_value(&mut h).expect("should have changed");
        assert_eq!(v, " ");
    }
}
