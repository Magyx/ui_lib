//! Integration tests for `WrappingRows` construction.

#[cfg(test)]
mod common;

#[cfg(test)]
mod widget_grid {
    use std::num::NonZero;

    use ui::model::{Color, Size};
    use ui::widget::{Element, Length, Rectangle, Widget, WrappingRows};

    /// Build N Fixed-size rectangles for grid cells.
    fn rects(n: usize) -> Vec<Element> {
        (0..n)
            .map(|_| {
                Element::new(Rectangle::new(
                    Size::new(Length::Fixed(10), Length::Fixed(10)),
                    Color::BLACK,
                ))
            })
            .collect()
    }

    fn columns(n: usize) -> NonZero<usize> {
        NonZero::new(n).expect("columns must be > 0")
    }

    /// Total rows produced by WrappingRows::new (via Widget::child_count).
    fn row_count(g: &WrappingRows) -> usize {
        g.child_count()
    }

    /// Number of cells in the i-th row.
    fn row_cell_count(g: &mut WrappingRows, i: usize) -> usize {
        g.child_mut(i).child_count()
    }

    // Chunking correctness

    #[test]
    fn grid_with_zero_items_has_zero_rows() {
        let g = WrappingRows::new(columns(3), Vec::<Element>::new());
        assert_eq!(row_count(&g), 0);
    }

    #[test]
    fn grid_with_fewer_items_than_columns_has_one_row() {
        let mut g = WrappingRows::new(columns(5), rects(3));
        assert_eq!(row_count(&g), 1);
        assert_eq!(row_cell_count(&mut g, 0), 3);
    }

    #[test]
    fn grid_with_exact_multiple_has_full_rows() {
        let mut g = WrappingRows::new(columns(3), rects(9));
        assert_eq!(row_count(&g), 3);
        for i in 0..3 {
            assert_eq!(row_cell_count(&mut g, i), 3, "row {i} should have 3 cells");
        }
    }

    #[test]
    fn grid_with_remainder_places_leftover_in_final_row() {
        // 7 items in rows of 3 => [3, 3, 1].
        let mut g = WrappingRows::new(columns(3), rects(7));
        assert_eq!(row_count(&g), 3);
        assert_eq!(row_cell_count(&mut g, 0), 3);
        assert_eq!(row_cell_count(&mut g, 1), 3);
        assert_eq!(row_cell_count(&mut g, 2), 1);
    }

    #[test]
    fn grid_single_column_gives_row_per_item() {
        let mut g = WrappingRows::new(columns(1), rects(5));
        assert_eq!(row_count(&g), 5);
        for i in 0..5 {
            assert_eq!(row_cell_count(&mut g, i), 1);
        }
    }

    #[test]
    fn grid_single_item_any_columns_gives_one_row_one_cell() {
        for cols in [1, 2, 5, 100] {
            let mut g = WrappingRows::new(columns(cols), rects(1));
            assert_eq!(row_count(&g), 1);
            assert_eq!(row_cell_count(&mut g, 0), 1);
        }
    }

    #[test]
    fn grid_many_items_fill_ceil_div_rows() {
        // 17 items, 4 columns => ceil(17/4) = 5 rows, sizes [4, 4, 4, 4, 1].
        let mut g = WrappingRows::new(columns(4), rects(17));
        assert_eq!(row_count(&g), 5);
        for i in 0..4 {
            assert_eq!(row_cell_count(&mut g, i), 4);
        }
        assert_eq!(row_cell_count(&mut g, 4), 1);
    }
}
