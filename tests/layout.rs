//! Integration tests for the layout engine.

#[cfg(test)]
mod common;

#[cfg(test)]
mod layout {
    use super::common::*;

    use std::cell::Cell;
    use std::rc::Rc;

    use ui::context::LayoutCtx;
    use ui::el;
    use ui::layout::Node;
    use ui::model::{Color, Size, Vec4};
    use ui::widget::{
        Column, Element, IntoElement, Length, Overlay, Rectangle, Row, Spacer, Widget,
    };

    // Leaf sizing

    #[test]
    fn fixed_size_leaf_keeps_its_exact_size_regardless_of_viewport() {
        let mut h = Harness::default();
        let (mut rect, slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(50), Length::Fixed(30)),
            Color::BLACK,
        ));
        h.layout(&mut rect, 800, 600);
        assert_eq!(read(&slot), (0, 0, 50, 30));
    }

    #[test]
    fn fixed_leaf_smaller_than_parent_stays_fixed() {
        // Sanity check: Fixed(40) in a 500-wide parent stays at 40.
        let mut h = Harness::default();
        let (mut rect, slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(40), Length::Fixed(30)),
            Color::BLACK,
        ));
        h.layout(&mut rect, 500, 500);
        assert_eq!(read(&slot), (0, 0, 40, 30));
    }

    #[test]
    fn fixed_leaf_overflows_smaller_parent() {
        // A Fixed(500) leaf in a 50-wide viewport must stay 500, not shrink
        // to 50. Overflow is visible (and will be clipped by the GPU
        // scissor or a parent with clip_children, not by the engine).
        let mut h = Harness::default();
        let (mut rect, slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(500), Length::Fixed(300)),
            Color::BLACK,
        ));
        h.layout(&mut rect, 50, 50);
        let r = read(&slot);
        assert_eq!((r.2, r.3), (500, 300));
    }

    #[test]
    fn grow_leaf_fills_viewport() {
        let mut h = Harness::default();
        let (mut rect, slot) = Probe::new(Rectangle::new(
            Size::new(Length::Grow, Length::Grow),
            Color::BLACK,
        ));
        h.layout(&mut rect, 800, 600);
        assert_eq!(read(&slot), (0, 0, 800, 600));
    }

    #[test]
    fn grow_leaf_in_normal_parent_still_fills_parent() {
        // Regression guard: dropping the .min(parent_*) clamp must not
        // break the common case where parent >= min.
        let mut h = Harness::default();
        let (mut rect, slot) = Probe::new(Rectangle::new(
            Size::new(Length::Grow, Length::Grow),
            Color::BLACK,
        ));
        h.layout(&mut rect, 400, 300);
        assert_eq!(read(&slot), (0, 0, 400, 300));
    }

    #[test]
    fn grow_leaf_with_min_overflows_smaller_parent() {
        // Already covered by min_size_floor_is_respected_when_grow_and_viewport_smaller;
        // repeated here to make the flex-min story self-contained in one place.
        let mut h = Harness::default();
        let (mut rect, slot) = Probe::new(
            Rectangle::new(Size::splat(Length::Grow), Color::BLACK).min(Size::new(200, 100)),
        );
        h.layout(&mut rect, 50, 50);
        let r = read(&slot);
        assert_eq!(r.2, 200);
        assert_eq!(r.3, 100);
    }

    #[test]
    fn grow_leaf_respects_max_cap_even_when_less_than_parent() {
        // Regression guard for the max-cap path. Grow + max=100 in a
        // 800-wide parent should stay at 100, not grow to 800.
        let mut h = Harness::default();
        let (mut rect, slot) = Probe::new(
            Rectangle::new(Size::splat(Length::Grow), Color::BLACK).max(Size::new(100, 60)),
        );
        h.layout(&mut rect, 800, 600);
        let r = read(&slot);
        assert_eq!(r.2, 100);
        assert_eq!(r.3, 60);
    }

    #[test]
    fn fit_leaf_with_no_content_is_zero() {
        let mut h = Harness::default();
        let (mut rect, slot) = Probe::new(Rectangle::new(Size::splat(Length::Fit), Color::BLACK));
        h.layout(&mut rect, 800, 600);
        let r = read(&slot);
        assert_eq!((r.2, r.3), (0, 0));
    }

    #[test]
    fn min_size_floor_is_respected_when_grow_and_viewport_smaller() {
        let mut h = Harness::default();
        let (mut rect, slot) = Probe::new(
            Rectangle::new(Size::splat(Length::Grow), Color::BLACK).min(Size::new(200, 100)),
        );
        h.layout(&mut rect, 50, 50);
        let r = read(&slot);
        assert!(r.2 >= 200, "width={} must honour min.width=200", r.2);
        assert!(r.3 >= 100, "height={} must honour min.height=100", r.3);
    }

    #[test]
    fn max_size_cap_is_respected_when_grow_and_viewport_larger() {
        let mut h = Harness::default();
        let (mut rect, slot) = Probe::new(
            Rectangle::new(Size::splat(Length::Grow), Color::BLACK).max(Size::new(100, 60)),
        );
        h.layout(&mut rect, 800, 600);
        let r = read(&slot);
        assert!(r.2 <= 100);
        assert!(r.3 <= 60);
    }

    // Row

    #[test]
    fn empty_row_has_zero_fit_size() {
        let mut h = Harness::default();
        let (mut row, slot) = Probe::new(Row::<TopMsg>::new(Vec::<Element<TopMsg>>::new()));
        h.layout(&mut row, 800, 600);
        let r = read(&slot);
        assert_eq!((r.2, r.3), (0, 0));
    }

    #[test]
    fn row_fit_container_sizes_to_sum_of_fixed_children() {
        let mut h = Harness::default();

        let (a, _) = probed_rect(Length::Fixed(40), Length::Fixed(20), Color::RED);
        let (b, _) = probed_rect(Length::Fixed(60), Length::Fixed(30), Color::GREEN);
        let (mut row, slot) = Probe::new(Row::<TopMsg>::new([a, b]));

        h.layout(&mut row, 1000, 1000);
        let r = read(&slot);
        assert_eq!((r.0, r.1), (0, 0));
        assert_eq!(r.2, 100, "Fit row width should sum children: 40 + 60");
        assert_eq!(r.3, 30, "Fit row height should be max child height");
    }

    #[test]
    fn row_with_spacing_adds_gaps_between_children() {
        let mut h = Harness::default();

        let (a, _) = probed_rect(Length::Fixed(40), Length::Fixed(20), Color::RED);
        let (b, _) = probed_rect(Length::Fixed(60), Length::Fixed(20), Color::GREEN);
        let (c, _) = probed_rect(Length::Fixed(30), Length::Fixed(20), Color::BLUE);
        let (mut row, slot) = Probe::new(Row::<TopMsg>::new([a, b, c]).spacing(10));

        h.layout(&mut row, 1000, 1000);
        let r = read(&slot);
        assert_eq!(r.2, 40 + 60 + 30 + 2 * 10);
    }

    #[test]
    fn row_with_padding_expands_width_and_height() {
        let mut h = Harness::default();

        let (child, _) = probed_rect(Length::Fixed(40), Length::Fixed(20), Color::RED);
        let (mut row, slot) =
            Probe::new(Row::<TopMsg>::new([child]).padding(Vec4::new(5, 6, 7, 8)));

        h.layout(&mut row, 1000, 1000);
        let r = read(&slot);
        assert_eq!(r.2, 40 + 5 + 7, "width = child + left + right padding");
        assert_eq!(r.3, 20 + 6 + 8, "height = child + top + bottom padding");
    }

    #[test]
    fn row_grow_child_takes_remaining_space() {
        let mut h = Harness::default();

        let (a, a_slot) = probed_rect(Length::Fixed(100), Length::Fixed(20), Color::RED);
        let (b, b_slot) = probed_rect(Length::Grow, Length::Fixed(20), Color::GREEN);
        let mut row: Row<TopMsg> =
            Row::new([a, b]).size(Size::new(Length::Fixed(500), Length::Fit));

        h.layout(&mut row, 1000, 1000);

        let ra = read(&a_slot);
        let rb = read(&b_slot);

        assert_eq!(ra.2, 100, "fixed child stays at 100");
        assert_eq!(rb.2, 400, "grow child takes remaining 500 - 100");
        assert_eq!(ra.0, 0);
        assert_eq!(rb.0, 100, "second child is placed after first");
    }

    #[test]
    fn row_two_grow_children_split_space_evenly() {
        let mut h = Harness::default();

        let (a, a_slot) = probed_rect(Length::Grow, Length::Fixed(20), Color::RED);
        let (b, b_slot) = probed_rect(Length::Grow, Length::Fixed(20), Color::GREEN);
        let mut row: Row<TopMsg> =
            Row::new([a, b]).size(Size::new(Length::Fixed(400), Length::Fit));

        h.layout(&mut row, 1000, 1000);

        let aw = read(&a_slot).2;
        let bw = read(&b_slot).2;
        // Allow 1px rounding slack in integer distribution.
        assert!((aw - 200).abs() <= 1, "a width should be ~200, got {aw}");
        assert!((bw - 200).abs() <= 1, "b width should be ~200, got {bw}");
        assert_eq!(aw + bw, 400, "total width must equal container width");
    }

    #[test]
    fn row_with_fixed_children_exceeding_viewport_does_not_shrink() {
        // A Fit row containing two Fixed(300) children in a 200-wide viewport
        // should report width 600. Children should render at their fixed
        // size, not crushed down.
        let mut h = Harness::default();

        let (a, a_slot) = probed_rect(Length::Fixed(300), Length::Fixed(20), Color::RED);
        let (b, b_slot) = probed_rect(Length::Fixed(300), Length::Fixed(20), Color::GREEN);
        let (mut row, slot) = Probe::new(Row::<TopMsg>::new([a, b]));

        h.layout(&mut row, 200, 200);

        assert_eq!(
            read(&slot).2,
            600,
            "Fit row must equal sum of fixed children"
        );
        assert_eq!(read(&a_slot).2, 300, "child A keeps its fixed width");
        assert_eq!(read(&b_slot).2, 300, "child B keeps its fixed width");
    }

    #[test]
    fn row_with_grow_min_child_overflows_fixed_parent() {
        // A fixed-width Row(300) containing one Grow child with min=500
        // should have the child at 500 (its min), even though the row is
        // 300. This is the nested version of the leaf test and tests that
        // the shrink pass in the container doesn't override min.
        let mut h = Harness::default();

        let child = Rectangle::new(Size::splat(Length::Grow), Color::RED).min(Size::new(500, 20));
        let (child_probe, child_slot) = Probe::new(child);

        let mut row: Row<TopMsg> =
            Row::new([child_probe]).size(Size::new(Length::Fixed(300), Length::Fit));

        h.layout(&mut row, 1000, 1000);

        let r = read(&child_slot);
        assert_eq!(
            r.2, 500,
            "grow child's min must win over parent's inner width"
        );
    }

    #[test]
    fn fixed_row_containing_empty_column_keeps_its_fixed_size() {
        let mut h = Harness::default();

        let empty_col: Column<TopMsg> = Column::new(Vec::<Element<TopMsg>>::new());
        let (mut row, slot) = Probe::new(
            Row::<TopMsg>::new([empty_col]).size(Size::new(Length::Fixed(200), Length::Fixed(100))),
        );

        h.layout(&mut row, 800, 600);
        let r = read(&slot);
        assert_eq!((r.2, r.3), (200, 100));
    }

    // Column

    #[test]
    fn column_fit_sums_heights_of_fixed_children() {
        let mut h = Harness::default();

        let (a, _) = probed_rect(Length::Fixed(40), Length::Fixed(20), Color::RED);
        let (b, _) = probed_rect(Length::Fixed(60), Length::Fixed(30), Color::GREEN);
        let (mut col, slot) = Probe::new(Column::<TopMsg>::new([a, b]));

        h.layout(&mut col, 1000, 1000);
        let r = read(&slot);
        assert_eq!(r.2, 60, "Fit column width = max child width");
        assert_eq!(r.3, 50, "Fit column height = sum of child heights");
    }

    #[test]
    fn column_with_spacing_adds_vertical_gaps() {
        let mut h = Harness::default();

        let (a, _) = probed_rect(Length::Fixed(10), Length::Fixed(20), Color::RED);
        let (b, _) = probed_rect(Length::Fixed(10), Length::Fixed(30), Color::GREEN);
        let (c, _) = probed_rect(Length::Fixed(10), Length::Fixed(40), Color::BLUE);
        let (mut col, slot) = Probe::new(Column::<TopMsg>::new([a, b, c]).spacing(5));

        h.layout(&mut col, 1000, 1000);
        let r = read(&slot);
        assert_eq!(r.3, 20 + 30 + 40 + 2 * 5);
    }

    #[test]
    fn column_places_children_at_successive_y_offsets() {
        let mut h = Harness::default();

        let (a, a_slot) = probed_rect(Length::Fixed(10), Length::Fixed(20), Color::RED);
        let (b, b_slot) = probed_rect(Length::Fixed(10), Length::Fixed(30), Color::GREEN);
        let mut col: Column<TopMsg> = Column::new([a, b]);

        h.layout(&mut col, 1000, 1000);

        let ra = read(&a_slot);
        let rb = read(&b_slot);

        assert_eq!(ra.1, 0, "first child at y=0");
        assert_eq!(rb.1, 20, "second child at y = first.y + first.h");
        assert_eq!(ra.0, 0);
        assert_eq!(rb.0, 0, "both children share x=0 in a column");
    }

    #[test]
    fn column_grow_child_takes_remaining_height() {
        let mut h = Harness::default();

        let (a, a_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(20), Length::Fixed(100)),
            Color::RED,
        ));
        let (b, b_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(20), Length::Grow),
            Color::GREEN,
        ));
        let mut col: Column<TopMsg> =
            Column::new([a, b]).size(Size::new(Length::Fit, Length::Fixed(500)));

        h.layout(&mut col, 1000, 1000);

        let ra = read(&a_slot);
        let rb = read(&b_slot);

        assert_eq!(ra.3, 100, "fixed child keeps its 100 height");
        assert_eq!(rb.3, 400, "grow child takes remaining 500 - 100");
        assert_eq!(ra.1, 0);
        assert_eq!(rb.1, 100, "second child placed below first");
    }

    #[test]
    fn column_two_grow_children_split_height_evenly() {
        let mut h = Harness::default();

        let (a, a_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(20), Length::Grow),
            Color::RED,
        ));
        let (b, b_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(20), Length::Grow),
            Color::GREEN,
        ));
        let mut col: Column<TopMsg> =
            Column::new([a, b]).size(Size::new(Length::Fit, Length::Fixed(400)));

        h.layout(&mut col, 1000, 1000);

        let ah = read(&a_slot).3;
        let bh = read(&b_slot).3;

        assert!((ah - 200).abs() <= 1);
        assert!((bh - 200).abs() <= 1);
        assert_eq!(ah + bh, 400);
    }

    #[test]
    fn column_grow_with_spacing_distributes_minus_gaps() {
        // 3 Grow children in a 400-tall column with spacing=20:
        // inner_h = 400, gaps = 2 * 20 = 40, each child = (400 - 40) / 3 = 120.
        let mut h = Harness::default();

        let (a, a_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(20), Length::Grow),
            Color::RED,
        ));
        let (b, b_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(20), Length::Grow),
            Color::GREEN,
        ));
        let (c, c_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(20), Length::Grow),
            Color::BLUE,
        ));
        let mut col: Column<TopMsg> = Column::new([a, b, c])
            .spacing(20)
            .size(Size::new(Length::Fit, Length::Fixed(400)));

        h.layout(&mut col, 1000, 1000);

        let ah = read(&a_slot).3;
        let bh = read(&b_slot).3;
        let ch = read(&c_slot).3;

        // Available for children: 400 - 40 = 360. 360 / 3 = 120.
        for (i, &hv) in [ah, bh, ch].iter().enumerate() {
            assert!((hv - 120).abs() <= 1, "child {i}: expected ~120, got {hv}");
        }
        assert_eq!(
            ah + bh + ch,
            360,
            "total child heights fill the non-gap area"
        );
    }

    #[test]
    fn column_with_fixed_children_exceeding_viewport_does_not_shrink() {
        // Symmetric vertical case.
        let mut h = Harness::default();

        let (a, a_slot) = probed_rect(Length::Fixed(20), Length::Fixed(300), Color::RED);
        let (b, b_slot) = probed_rect(Length::Fixed(20), Length::Fixed(300), Color::GREEN);
        let (mut col, slot) = Probe::new(Column::<TopMsg>::new([a, b]));

        h.layout(&mut col, 200, 200);

        assert_eq!(
            read(&slot).3,
            600,
            "Fit column must equal sum of fixed children"
        );
        assert_eq!(read(&a_slot).3, 300);
        assert_eq!(read(&b_slot).3, 300);
    }

    // Mixed row/column composition

    #[test]
    fn column_of_rows_composes_sizes_correctly() {
        let mut h = Harness::default();

        // Column { Row { 40x20, 60x20 }, Row { 30x15 } }
        let (a1, _) = probed_rect(Length::Fixed(40), Length::Fixed(20), Color::RED);
        let (a2, _) = probed_rect(Length::Fixed(60), Length::Fixed(20), Color::GREEN);
        let r1: Row<TopMsg> = Row::new([a1, a2]);

        let (b1, _) = probed_rect(Length::Fixed(30), Length::Fixed(15), Color::BLUE);
        let r2: Row<TopMsg> = Row::new([b1]);

        let (mut col, slot) = Probe::new(Column::<TopMsg>::new([r1, r2]));
        h.layout(&mut col, 1000, 1000);

        let r = read(&slot);
        assert_eq!(r.2, 100, "column width = max row width (40+60 vs 30)");
        assert_eq!(r.3, 35, "column height = sum of row heights (20 + 15)");
    }

    // Spacer

    #[test]
    fn spacer_with_grow_in_row_pushes_last_child_to_far_side() {
        let mut h = Harness::default();

        let (left, left_slot) = probed_rect(Length::Fixed(50), Length::Fixed(20), Color::RED);
        let spacer: Element<TopMsg> =
            Element::new(Spacer::new(Size::new(Length::Grow, Length::Fixed(20))));
        let (right, right_slot) = probed_rect(Length::Fixed(50), Length::Fixed(20), Color::BLUE);

        let mut row: Row<TopMsg> =
            Row::new([left, spacer, right]).size(Size::new(Length::Fixed(300), Length::Fit));

        h.layout(&mut row, 1000, 1000);

        let l = read(&left_slot);
        let r = read(&right_slot);

        assert_eq!(l.0, 0, "left at x=0");
        assert_eq!(l.2, 50);
        assert_eq!(r.0, 250, "right pushed to far side: 300 - 50");
        assert_eq!(r.2, 50);
    }

    // Overlay / absolute positioning

    #[test]
    fn overlay_places_single_child_at_origin_when_not_pushed() {
        // Overlay::new wraps children with offset (0, 0) by default.
        // A single Fixed child should end up at (0, 0).
        let mut h = Harness::default();

        let (child, child_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(50), Length::Fixed(30)),
            Color::RED,
        ));

        let (mut overlay, overlay_slot) = Probe::new(Overlay::<TopMsg>::new([child]));

        h.layout(&mut overlay, 800, 600);

        let child_r = read(&child_slot);
        let overlay_r = read(&overlay_slot);

        // Overlay is Fit + has Absolute children, which are skipped from
        // the sum. So overlay's fit size comes from zero children = 0x0.
        assert_eq!(
            (overlay_r.2, overlay_r.3),
            (0, 0),
            "Fit overlay ignores absolute children for sizing"
        );
        // Child still gets laid out at its fixed size, at origin.
        assert_eq!(child_r, (0, 0, 50, 30));
    }

    #[test]
    fn overlay_with_fixed_size_gives_absolute_child_full_inner_area() {
        // Absolute children get inner_w / inner_h in assign_* phases.
        // A Fixed(300x200) Overlay with a Grow child should stretch the
        // child to (300, 200).
        let mut h = Harness::default();

        let (child, child_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Grow, Length::Grow),
            Color::BLUE,
        ));

        let mut overlay: Overlay<TopMsg> =
            Overlay::new([child]).size(Size::new(Length::Fixed(300), Length::Fixed(200)));

        h.layout(&mut overlay, 1000, 1000);

        let r = read(&child_slot);
        assert_eq!(
            (r.2, r.3),
            (300, 200),
            "absolute Grow child fills inner area"
        );
    }

    #[test]
    fn overlay_push_offsets_child_from_top_left() {
        // Overlay::push places the child at an explicit offset. With a
        // Fixed(400x300) overlay and a 50x30 child pushed to (100, 80),
        // the child should end up at (100, 80).
        let mut h = Harness::default();

        let (child, child_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(50), Length::Fixed(30)),
            Color::RED,
        ));

        let mut overlay: Overlay<TopMsg> = Overlay::new(Vec::<Element<TopMsg>>::new())
            .size(Size::new(Length::Fixed(400), Length::Fixed(300)));
        overlay.push(child, 100, 80);

        h.layout(&mut overlay, 1000, 1000);

        let r = read(&child_slot);
        assert_eq!(r.0, 100, "absolute child uses offset_pos.x");
        assert_eq!(r.1, 80, "absolute child uses offset_pos.y");
        assert_eq!((r.2, r.3), (50, 30), "fixed size preserved");
    }

    #[test]
    fn overlay_with_padding_offsets_absolute_child_by_padding() {
        // Absolute children are placed at `base_x + offset`, where base_x
        // already includes padding.left (see place() in layout.rs). So
        // pushing a child to (0, 0) inside a padded overlay should land
        // at (padding.left, padding.top), not (0, 0).
        let mut h = Harness::default();

        let (child, child_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(10), Length::Fixed(10)),
            Color::RED,
        ));

        let mut overlay: Overlay<TopMsg> = Overlay::new(Vec::<Element<TopMsg>>::new())
            .size(Size::new(Length::Fixed(200), Length::Fixed(200)))
            .padding(Vec4::new(7, 11, 0, 0));
        overlay.push(child, 0, 0);

        h.layout(&mut overlay, 1000, 1000);

        let r = read(&child_slot);
        assert_eq!(
            (r.0, r.1),
            (7, 11),
            "absolute child starts at (pad.left, pad.top) + offset"
        );
    }

    #[test]
    fn overlay_mixes_absolute_children_independently() {
        // Two pushed children with different offsets should each land at
        // their own spot without affecting each other.
        let mut h = Harness::default();

        let (a, a_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(20), Length::Fixed(20)),
            Color::RED,
        ));
        let (b, b_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(30), Length::Fixed(30)),
            Color::GREEN,
        ));

        let mut overlay: Overlay<TopMsg> = Overlay::new(Vec::<Element<TopMsg>>::new())
            .size(Size::new(Length::Fixed(500), Length::Fixed(400)));
        overlay.push(a, 10, 20);
        overlay.push(b, 200, 150);

        h.layout(&mut overlay, 1000, 1000);

        let ra = read(&a_slot);
        let rb = read(&b_slot);
        assert_eq!((ra.0, ra.1), (10, 20));
        assert_eq!((rb.0, rb.1), (200, 150));
    }

    #[test]
    fn overlay_nested_in_row_does_not_affect_sibling_positions() {
        // A Row containing [fixed 40, overlay{ absolute child 100x100 }, fixed 40].
        // Overlay reports its own size as 0x0 (absolute children are
        // skipped from fit-sum), so the Row should place the right
        // sibling immediately after the left sibling — not after the
        // overlay's content.
        let mut h = Harness::default();

        let (left, left_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(40), Length::Fixed(20)),
            Color::RED,
        ));
        let (right, right_slot) = Probe::new(Rectangle::new(
            Size::new(Length::Fixed(40), Length::Fixed(20)),
            Color::BLUE,
        ));

        let inner_rect = Rectangle::new(
            Size::new(Length::Fixed(100), Length::Fixed(100)),
            Color::GREEN,
        );
        let overlay: Overlay<TopMsg> = Overlay::new([inner_rect]);

        let mut row: Row<TopMsg> = Row::new(el![left, overlay, right]);

        h.layout(&mut row, 1000, 1000);

        let l = read(&left_slot);
        let r = read(&right_slot);

        assert_eq!(l.0, 0);
        assert_eq!(
            r.0, 40,
            "Overlay with only absolute children should not push siblings"
        );
    }

    // post_width_query — min_height_for_width inflation
    //
    // The engine runs post_width_query after width assignment and
    // before height measure. For LEAF widgets (no children), if
    // min_height_for_width returns Some(h), the engine sets the node's
    // min.height to max(h, existing min.height).min(max.height).
    //
    // We test this with a custom leaf widget that reports an intrinsic
    // height proportional to its allocated width.

    /// Leaf widget that reports `f(width)` as its intrinsic height.
    /// Stays generic over M so the same struct works for any harness.
    struct IntrinsicHeightLeaf {
        height_for_width: Box<dyn Fn(i32) -> i32>,
        slot: RectSlot,
    }

    impl IntrinsicHeightLeaf {
        fn new<F: Fn(i32) -> i32 + 'static>(f: F) -> (Self, RectSlot) {
            let slot: RectSlot = Rc::new(Cell::new(None));
            (
                Self {
                    height_for_width: Box::new(f),
                    slot: slot.clone(),
                },
                slot,
            )
        }
    }

    impl IntoElement for IntrinsicHeightLeaf {}

    impl<M: 'static> Widget<M> for IntrinsicHeightLeaf {
        fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
            Node {
                size: Size::new(Length::Grow, Length::Fit),
                ..Default::default()
            }
        }
        fn child_count(&self) -> usize {
            0
        }
        fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
            unreachable!("leaf has no children")
        }
        fn min_height_for_width<'a>(
            &mut self,
            _ctx: &mut LayoutCtx<'a, M>,
            width: i32,
        ) -> Option<i32> {
            Some((self.height_for_width)(width))
        }
        fn paint(
            &mut self,
            ctx: &mut ui::context::PaintCtx,
            _out: &mut Vec<ui::primitive::Instance>,
        ) {
            let r = ctx.rect();
            self.slot.set(Some(r.xywh()));
        }
        fn handle(&mut self, _ctx: &mut ui::context::EventCtx<M>) {}
    }

    #[test]
    fn post_width_query_inflates_leaf_min_height() {
        // Leaf reports "at any width W, I need height = W / 4".
        // Parent is a fixed-width Row at 400 wide. Leaf gets width=400,
        // should be inflated to height = 100.
        let mut h = Harness::default();

        let (leaf, leaf_slot) = IntrinsicHeightLeaf::new(|w| w / 4);

        // Wrap in a Row so the leaf gets a concrete width.
        let mut row: Row<TopMsg> =
            Row::new([leaf]).size(Size::new(Length::Fixed(400), Length::Fit));

        h.layout(&mut row, 1000, 1000);

        let r = read(&leaf_slot);
        assert_eq!(r.2, 400, "leaf got full row width");
        assert_eq!(
            r.3, 100,
            "post_width_query should have inflated height to width/4"
        );
    }

    #[test]
    fn post_width_query_narrower_width_gives_taller_result() {
        // Classic text-wrap intrinsic: narrower => taller.
        // Use a fresh harness for each run to keep engine state clean.
        let tall_h = {
            let mut h = Harness::default();
            let (leaf, leaf_slot) =
                IntrinsicHeightLeaf::new(|w| if w > 0 { 1000 / w } else { 1000 });
            let mut row: Row<TopMsg> =
                Row::new([leaf]).size(Size::new(Length::Fixed(50), Length::Fit));
            h.layout(&mut row, 1000, 1000);
            read(&leaf_slot).3
        };

        let short_h = {
            let mut h = Harness::default();
            let (leaf, leaf_slot) =
                IntrinsicHeightLeaf::new(|w| if w > 0 { 1000 / w } else { 1000 });
            let mut row: Row<TopMsg> =
                Row::new([leaf]).size(Size::new(Length::Fixed(500), Length::Fit));
            h.layout(&mut row, 1000, 1000);
            read(&leaf_slot).3
        };

        assert!(
            tall_h > short_h,
            "narrower width should produce taller leaf (tall={tall_h}, short={short_h})"
        );
    }

    #[test]
    fn post_width_query_does_not_reduce_existing_min_height() {
        // If min_height_for_width returns a value *less* than the node's
        // existing min.height (set via Node.min), the engine should keep
        // the larger value. See the clamp in post_width_query:
        //   h.max(node.min.height).min(node.max.height)
        struct LeafWithMin {
            slot: RectSlot,
        }
        impl IntoElement for LeafWithMin {}
        impl<M: 'static> Widget<M> for LeafWithMin {
            fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
                Node {
                    size: Size::new(Length::Grow, Length::Fit),
                    min: Size::new(0, 50), // force height floor
                    ..Default::default()
                }
            }
            fn child_count(&self) -> usize {
                0
            }
            fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
                unreachable!()
            }
            fn min_height_for_width<'a>(
                &mut self,
                _ctx: &mut LayoutCtx<'a, M>,
                _width: i32,
            ) -> Option<i32> {
                Some(0) // would shrink if post_width_query didn't clamp
            }
            fn paint(
                &mut self,
                ctx: &mut ui::context::PaintCtx,
                _out: &mut Vec<ui::primitive::Instance>,
            ) {
                let r = ctx.rect();
                self.slot.set(Some(r.xywh()));
            }
            fn handle(&mut self, _ctx: &mut ui::context::EventCtx<M>) {}
        }

        let mut h = Harness::default();
        let slot: RectSlot = Rc::new(Cell::new(None));
        let leaf = LeafWithMin { slot: slot.clone() };
        let mut row: Row<TopMsg> =
            Row::new([leaf]).size(Size::new(Length::Fixed(200), Length::Fit));

        h.layout(&mut row, 1000, 1000);
        let r = slot.get().unwrap();
        assert!(
            r.3 >= 50,
            "post_width_query must not reduce below existing min.height; got {}",
            r.3
        );
    }

    #[test]
    fn post_width_query_respects_max_height_cap() {
        // If min_height_for_width returns a huge value, the clamp should
        // cap it at max.height. Test: leaf with max.height=30 reporting 1000.
        struct LeafWithMax {
            slot: RectSlot,
        }
        impl IntoElement for LeafWithMax {}
        impl<M: 'static> Widget<M> for LeafWithMax {
            fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
                Node {
                    size: Size::new(Length::Grow, Length::Fit),
                    max: Size::new(i32::MAX, 30),
                    ..Default::default()
                }
            }
            fn child_count(&self) -> usize {
                0
            }
            fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
                unreachable!()
            }
            fn min_height_for_width<'a>(
                &mut self,
                _ctx: &mut LayoutCtx<'a, M>,
                _width: i32,
            ) -> Option<i32> {
                Some(1000)
            }
            fn paint(
                &mut self,
                ctx: &mut ui::context::PaintCtx,
                _out: &mut Vec<ui::primitive::Instance>,
            ) {
                let r = ctx.rect();
                self.slot.set(Some(r.xywh()));
            }
            fn handle(&mut self, _ctx: &mut ui::context::EventCtx<M>) {}
        }

        let mut h = Harness::default();
        let slot: RectSlot = Rc::new(Cell::new(None));
        let leaf = LeafWithMax { slot: slot.clone() };
        let mut row: Row<TopMsg> =
            Row::new([leaf]).size(Size::new(Length::Fixed(200), Length::Fit));

        h.layout(&mut row, 1000, 1000);
        let r = slot.get().unwrap();
        assert!(
            r.3 <= 30,
            "post_width_query must respect max.height; got {}",
            r.3
        );
    }

    #[test]
    fn post_width_query_ignores_non_leaf_nodes() {
        // post_width_query only queries leaf nodes (first_child.is_none()).
        // A container with children should NOT have min_height_for_width
        // called on it, even if it overrides the method. Test with a
        // wrapper that panics in min_height_for_width; layout should
        // succeed (no panic).
        struct WrapperThatWouldPanic<M: 'static> {
            child: Element<M>,
        }
        impl<M: 'static> IntoElement for WrapperThatWouldPanic<M> {}
        impl<M: 'static> Widget<M> for WrapperThatWouldPanic<M> {
            fn layout<'a>(&mut self, _ctx: &mut LayoutCtx<'a, M>) -> Node {
                Node {
                    size: Size::new(Length::Fit, Length::Fit),
                    ..Default::default()
                }
            }
            fn child_count(&self) -> usize {
                1
            }
            fn child_mut(&mut self, _i: usize) -> &mut dyn Widget<M> {
                self.child.as_mut()
            }
            fn min_height_for_width<'a>(
                &mut self,
                _ctx: &mut LayoutCtx<'a, M>,
                _width: i32,
            ) -> Option<i32> {
                panic!("post_width_query called min_height_for_width on a non-leaf");
            }
            fn paint(
                &mut self,
                _ctx: &mut ui::context::PaintCtx,
                _out: &mut Vec<ui::primitive::Instance>,
            ) {
            }
            fn handle(&mut self, _ctx: &mut ui::context::EventCtx<M>) {}
        }

        let mut h = Harness::default();
        let wrapper: WrapperThatWouldPanic<TopMsg> = WrapperThatWouldPanic {
            child: Element::new(Rectangle::new(
                Size::new(Length::Fixed(10), Length::Fixed(10)),
                Color::RED,
            )),
        };
        let mut root: Column<TopMsg> = Column::new([wrapper]);

        // Must not panic.
        h.layout(&mut root, 1000, 1000);
    }

    // Determinism

    #[test]
    fn layout_is_deterministic_across_runs_on_same_harness() {
        // Running the same tree twice on the same harness must produce
        // identical results. Catches state leaks between frames.
        let mut h = Harness::default();

        let build = || -> (Probe<TopMsg, Column<TopMsg>>, RectSlot) {
            let (a, _) = probed_rect(Length::Fixed(40), Length::Fixed(20), Color::RED);
            let (b, _) = probed_rect(Length::Fixed(60), Length::Fixed(30), Color::GREEN);
            Probe::new(Column::<TopMsg>::new([a, b]))
        };

        let (mut first, slot1) = build();
        h.layout(&mut first, 800, 600);
        let r1 = read(&slot1);

        let (mut second, slot2) = build();
        h.layout(&mut second, 800, 600);
        let r2 = read(&slot2);

        assert_eq!(r1, r2);
    }
}
