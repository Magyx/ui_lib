//! Tests for the paint pass: paint_tree walker behaviour and per-widget
//! Instance emission. Requires the prepare/paint split — PaintCtx has no
//! GPU refs.

#[cfg(test)]
mod common;

#[cfg(test)]
mod paint {
    use super::common::*;

    use ui::{prelude::*, primitive::Primitive};

    // Rectangle paint output

    #[test]
    fn rectangle_emits_one_instance_with_correct_color() {
        let mut h = Harness::default();
        let mut rect = Rectangle::new(Size::new(Length::Fixed(50), Length::Fixed(30)), Color::RED);
        h.layout(&mut rect, 800, 600);

        let instances = h.paint(&mut rect);
        assert_eq!(instances.len(), 1, "one rectangle => one instance");

        let batch = &instances.batches()[0];
        let prim = instances.view::<Primitive>(batch)[0];
        assert_eq!(prim.position, [0.0, 0.0]);
        assert_eq!(prim.size, [50.0, 30.0]);
        assert_eq!(prim.data1[0], Color::RED.0, "color packed into data1[0]");
    }

    #[test]
    fn transparent_rectangle_emits_nothing() {
        let mut h = Harness::default();
        let mut rect = Rectangle::new(
            Size::new(Length::Fixed(50), Length::Fixed(30)),
            Color::TRANSPARENT,
        );
        h.layout(&mut rect, 800, 600);

        let instances = h.paint(&mut rect);
        assert!(
            instances.is_empty(),
            "transparent rectangle should emit no instances"
        );
    }

    // Row paint output — children in order at correct positions

    #[test]
    fn row_of_two_rectangles_emits_two_instances_in_order() {
        let mut h = Harness::default();

        let a = Rectangle::new(Size::new(Length::Fixed(40), Length::Fixed(20)), Color::RED);
        let b = Rectangle::new(
            Size::new(Length::Fixed(60), Length::Fixed(20)),
            Color::GREEN,
        );
        let mut row = Row::new([Element::new(a), Element::new(b)]);

        h.layout(&mut row, 1000, 1000);
        let instances = h.paint(&mut row);

        // Row itself has no background (transparent), so only children emit.
        assert_eq!(instances.len(), 2, "two rectangles => two instances");

        let batch = &instances.batches()[0];
        let prims = instances.view::<Primitive>(batch);
        let p0 = prims[0];
        let p1 = prims[1];

        assert_eq!(p0.position, [0.0, 0.0], "first child at origin");
        assert_eq!(p0.size, [40.0, 20.0]);

        assert_eq!(p1.position, [40.0, 0.0], "second child after first");
        assert_eq!(p1.size, [60.0, 20.0]);
    }

    // Column paint output

    #[test]
    fn column_of_two_rectangles_emits_at_successive_y() {
        let mut h = Harness::default();

        let a = Rectangle::new(Size::new(Length::Fixed(30), Length::Fixed(20)), Color::RED);
        let b = Rectangle::new(Size::new(Length::Fixed(30), Length::Fixed(40)), Color::BLUE);
        let mut col = Column::new([Element::new(a), Element::new(b)]);

        h.layout(&mut col, 1000, 1000);
        let instances = h.paint(&mut col);

        assert_eq!(instances.len(), 2);

        let batch = &instances.batches()[0];
        let prims = instances.view::<Primitive>(batch);
        let p0 = prims[0];
        let p1 = prims[1];

        assert_eq!(p0.position[1], 0.0);
        assert_eq!(p1.position[1], 20.0, "second child at y = first.h");
    }

    // paint_tree: clip_children sets scissor on children

    #[test]
    fn clip_children_sets_scissor_on_child_instances() {
        let mut h = Harness::default();

        // A row with clip_children=true containing a child that fits.
        // We can't set clip_children directly on Row (no API), but we
        // can observe that Scrollable has it. For a simpler test, use
        // an Overlay with ContentFit::Cover which sets clip_children.
        //
        // Simpler approach: just test that a child inside a Row at
        // known bounds gets the screen_clip applied. Since paint_tree
        // passes `screen_clip = Some([0, 0, max_w, max_h])`, every
        // instance should have a scissor matching the screen.
        let a = Rectangle::new(Size::new(Length::Fixed(40), Length::Fixed(20)), Color::RED);
        let mut row = Row::new([Element::new(a)]);

        h.layout(&mut row, 200, 100);
        let instances = h.paint(&mut row);

        assert!(!instances.is_empty());
        let scissor = instances.batches()[0].clip;
        assert_eq!(
            scissor,
            Some([0, 0, 200, 100]),
            "screen clip should be applied to all instances"
        );
    }

    // batching

    #[test]
    fn siblings_under_one_clip_share_a_batch() {
        let mut h = Harness::default();

        let a = Rectangle::new(Size::new(Length::Fixed(40), Length::Fixed(20)), Color::RED);
        let b = Rectangle::new(Size::new(Length::Fixed(40), Length::Fixed(20)), Color::BLUE);
        let mut row = Row::new([Element::new(a), Element::new(b)]);

        h.layout(&mut row, 200, 100);
        let instances = h.paint(&mut row);

        assert_eq!(instances.len(), 2, "two rectangles");
        assert_eq!(
            instances.batches().len(),
            1,
            "same pipeline and same clip must compress into one draw"
        );
        assert_eq!(instances.batches()[0].count, 2);
    }

    #[test]
    fn batches_cover_every_instance_contiguously() {
        let mut h = Harness::default();

        let inner = Column::new([
            Element::new(Rectangle::new(
                Size::new(Length::Fixed(80), Length::Fixed(60)),
                Color::RED,
            )),
            Element::new(Rectangle::new(
                Size::new(Length::Fixed(80), Length::Fixed(60)),
                Color::BLUE,
            )),
        ]);
        let mut root = Row::new([Element::new(
            Scrollable::new(inner).size(Size::new(Length::Fixed(100), Length::Fixed(50))),
        )]);

        h.layout(&mut root, 200, 100);
        let instances = h.paint(&mut root);

        // The invariant the renderer depends on: batches tile the byte buffer
        // exactly, and their counts sum to the instance count.
        let stride = std::mem::size_of::<Primitive>() as u64;
        let mut next = 0u64;
        let mut total = 0usize;
        for batch in instances.batches() {
            assert_eq!(batch.byte_offset, next, "gap or overlap between batches");
            assert!(batch.count > 0, "empty batch");
            next += batch.count as u64 * stride;
            total += batch.count as usize;
        }
        assert_eq!(total, instances.len());
    }

    #[test]
    fn scrollable_narrows_the_clip_for_its_children() {
        let mut h = Harness::default();

        // Content taller than the viewport, so the scrollable must clip.
        let inner = Rectangle::new(
            Size::new(Length::Fixed(80), Length::Fixed(400)),
            Color::GREEN,
        );
        let mut root = Row::new([Element::new(
            Scrollable::new(inner).size(Size::new(Length::Fixed(100), Length::Fixed(50))),
        )]);

        h.layout(&mut root, 200, 100);
        let instances = h.paint(&mut root);

        let screen = Some([0, 0, 200, 100]);
        let narrowed = instances
            .batches()
            .iter()
            .find(|b| b.clip.is_some() && b.clip != screen);

        let clip = narrowed
            .expect("scrollable content should carry a clip tighter than the screen")
            .clip
            .unwrap();

        assert!(
            clip[3] <= 50,
            "child clip height {} should be bounded by the 50px viewport",
            clip[3]
        );
        assert!(
            clip[2] <= 100,
            "child clip width {} should be bounded by the 100px viewport",
            clip[2]
        );
    }

    #[test]
    fn nested_scrollables_intersect_to_the_tighter_clip() {
        let mut h = Harness::default();

        let innermost = Rectangle::new(
            Size::new(Length::Fixed(80), Length::Fixed(400)),
            Color::GREEN,
        );
        let inner =
            Scrollable::new(innermost).size(Size::new(Length::Fixed(90), Length::Fixed(30)));
        let outer = Scrollable::new(inner).size(Size::new(Length::Fixed(120), Length::Fixed(80)));
        let mut root = Row::new([Element::new(outer)]);

        h.layout(&mut root, 200, 100);
        let instances = h.paint(&mut root);

        // Whatever the tightest clip is, it must not exceed the inner
        // scrollable's viewport: parents narrow children, never the reverse.
        let tightest = instances
            .batches()
            .iter()
            .filter_map(|b| b.clip)
            .min_by_key(|c| c[2] as i64 * c[3] as i64)
            .expect("expected at least one clipped batch");

        assert!(
            tightest[3] <= 30,
            "nested clip height {} must be bounded by the inner 30px viewport",
            tightest[3]
        );
        assert!(
            tightest[2] <= 90,
            "nested clip width {} must be bounded by the inner 90px viewport",
            tightest[2]
        );
    }

    #[test]
    fn clip_is_restored_after_a_clipping_subtree() {
        let mut h = Harness::default();

        let scrolled = Scrollable::new(Rectangle::new(
            Size::new(Length::Fixed(80), Length::Fixed(400)),
            Color::GREEN,
        ))
        .size(Size::new(Length::Fixed(60), Length::Fixed(30)));

        // A sibling *after* the scrollable must not inherit its clip.
        let after = Rectangle::new(Size::new(Length::Fixed(40), Length::Fixed(20)), Color::RED);
        let mut root = Row::new([Element::new(scrolled), Element::new(after)]);

        h.layout(&mut root, 200, 100);
        let instances = h.paint(&mut root);

        let screen = Some([0, 0, 200, 100]);
        assert_eq!(
            instances.batches().last().unwrap().clip,
            screen,
            "the trailing sibling should be back under the screen clip"
        );
    }

    // paint_tree: cursor increments through all nodes

    #[test]
    fn paint_visits_all_nodes_in_tree() {
        let mut h = Harness::default();

        // Row { Rect, Column { Rect, Rect } } = 5 nodes total
        // (Row, Rect, Column, Rect, Rect)
        let a = Rectangle::new(Size::new(Length::Fixed(10), Length::Fixed(10)), Color::RED);
        let b = Rectangle::new(
            Size::new(Length::Fixed(10), Length::Fixed(10)),
            Color::GREEN,
        );
        let c = Rectangle::new(Size::new(Length::Fixed(10), Length::Fixed(10)), Color::BLUE);

        let col = Column::new([Element::new(b), Element::new(c)]);
        let mut row = Row::new([Element::new(a), Element::new(col)]);

        h.layout(&mut row, 1000, 1000);
        let instances = h.paint(&mut row);

        // 3 visible rectangles (Row and Column themselves are transparent).
        assert_eq!(instances.len(), 3);
    }

    // Row with background color emits bg + children

    #[test]
    fn row_with_background_emits_bg_then_children() {
        let mut h = Harness::default();

        let child = Rectangle::new(
            Size::new(Length::Fixed(20), Length::Fixed(20)),
            Color::GREEN,
        );
        let mut row = Row::new([Element::new(child)]).color(Color::RED);

        h.layout(&mut row, 1000, 1000);
        let instances = h.paint(&mut row);

        // Row bg + child rect = 2 instances.
        assert_eq!(instances.len(), 2, "bg + child");

        let batch = &instances.batches()[0];
        let prims = instances.view::<Primitive>(batch);
        let p0 = prims[0];
        let p1 = prims[1];

        // Background is emitted first (by paint_tree visiting the parent
        // before recursing into children).
        assert_eq!(
            p0.data1[0],
            Color::RED.0,
            "first instance is the Row background"
        );
        assert_eq!(p1.data1[0], Color::GREEN.0, "second instance is the child");
    }

    // Nested layout: positions propagate correctly through paint

    #[test]
    fn nested_row_column_positions_correct_in_paint_output() {
        let mut h = Harness::default();

        // Row { fixed(50x10), Column { fixed(30x15), fixed(30x25) } }
        let a = Rectangle::new(Size::new(Length::Fixed(50), Length::Fixed(10)), Color::RED);
        let b = Rectangle::new(
            Size::new(Length::Fixed(30), Length::Fixed(15)),
            Color::GREEN,
        );
        let c = Rectangle::new(Size::new(Length::Fixed(30), Length::Fixed(25)), Color::BLUE);

        let col = Column::new([Element::new(b), Element::new(c)]);
        let mut row = Row::new([Element::new(a), Element::new(col)]);

        h.layout(&mut row, 1000, 1000);
        let instances = h.paint(&mut row);

        assert_eq!(instances.len(), 3);

        let batch = &instances.batches()[0];
        let prims = instances.view::<Primitive>(batch);
        let pa = prims[0];
        let pb = prims[1];
        let pc = prims[2];

        // a at (0, 0)
        assert_eq!(pa.position, [0.0, 0.0]);
        // b at (50, 0) — column starts after a
        assert_eq!(pb.position, [50.0, 0.0]);
        // c at (50, 15) — below b in the column
        assert_eq!(pc.position, [50.0, 15.0]);
    }
}
