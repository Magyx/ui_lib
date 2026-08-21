use crate::model::{Inset, Position, Rect, Size};

#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub size: Size<Length>,
    pub min: Size<i32>,
    pub max: Size<i32>,
    pub layout_dir: Axis,
    pub padding: Inset,
    pub spacing: i32,
    pub clip_children: bool,
    pub children_offset: Position<i32>,
    pub is_absolute: bool,
    pub offset_pos: Position<i32>,
    pub main: Main,
    pub cross: Align,
    /// Per-child override of the parent's `cross`. `None` inherits.
    pub cross_self: Option<Align>,
    /// Cross-axis only: `Fit` children fill the container's cross size.
    /// Resolved in the assign pass, so a filled child's subtree reflows.
    pub fill_cross: bool,
}
impl Default for Node {
    fn default() -> Self {
        Self {
            size: Default::default(),
            min: Default::default(),
            max: Size::splat(i32::MAX),
            layout_dir: Default::default(),
            padding: Default::default(),
            spacing: Default::default(),
            children_offset: Default::default(),
            clip_children: Default::default(),
            is_absolute: Default::default(),
            offset_pos: Default::default(),
            main: Main::default(),
            cross: Align::START,
            cross_self: None,
            fill_cross: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Length {
    #[default]
    Fit,
    Fixed(i32),
    Fill(f32),
}
impl Length {
    pub(crate) fn weight(self) -> Option<f32> {
        match self {
            Length::Fill(w) => Some(w),
            _ => None,
        }
    }
}
impl<T> From<T> for Length
where
    T: Into<i32>,
{
    fn from(value: T) -> Self {
        Length::Fixed(value.into())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Axis {
    #[default]
    Horizontal,
    Vertical,
}

/// A position along one axis, as a fraction of the free space.
/// `0.0` is the leading edge, `0.5` the centre, `1.0` the trailing edge.
///
/// Values outside `0.0..=1.0` are allowed and place the point outside the box,
/// which is what lets an anchor sit just beyond an edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Align(pub f32);

impl Align {
    pub const START: Self = Self(0.0);
    pub const CENTER: Self = Self(0.5);
    pub const END: Self = Self(1.0);

    #[inline]
    pub const fn new(fraction: f32) -> Self {
        Self(fraction)
    }

    #[inline]
    pub const fn get(self) -> f32 {
        self.0
    }

    /// Mirror about the centre -> `START` becomes `END`.
    #[inline]
    pub fn flip(self) -> Self {
        Self(1.0 - self.0)
    }
}

/// How a container distributes its children along the layout axis.
///
/// Split from [`Align`] because distribution is only meaningful on the main
/// axis; the cross axis takes a bare `Align`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Main {
    /// Pack the children together and put the block at this position.
    At(Align),
    /// Free space between each pair of children; none at the ends.
    Between,
    /// A half unit at each end, a full unit between children.
    Around,
    /// Equal units at the ends and between children.
    Evenly,
}

impl Default for Main {
    #[inline]
    fn default() -> Self {
        Self::At(Align::START)
    }
}

impl From<Align> for Main {
    #[inline]
    fn from(a: Align) -> Self {
        Self::At(a)
    }
}

/// A point expressed as a fraction of a box, on both axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Align2 {
    pub x: Align,
    pub y: Align,
}

impl Align2 {
    pub const TOP_LEFT: Self = Self::frac(0.0, 0.0);
    pub const TOP_CENTER: Self = Self::frac(0.5, 0.0);
    pub const TOP_RIGHT: Self = Self::frac(1.0, 0.0);
    pub const CENTER_LEFT: Self = Self::frac(0.0, 0.5);
    pub const CENTER: Self = Self::frac(0.5, 0.5);
    pub const CENTER_RIGHT: Self = Self::frac(1.0, 0.5);
    pub const BOTTOM_LEFT: Self = Self::frac(0.0, 1.0);
    pub const BOTTOM_CENTER: Self = Self::frac(0.5, 1.0);
    pub const BOTTOM_RIGHT: Self = Self::frac(1.0, 1.0);

    #[inline]
    pub const fn new(x: Align, y: Align) -> Self {
        Self { x, y }
    }

    /// Build from raw fractions -> `Align2::frac(0.3, 1.0)`.
    #[inline]
    pub const fn frac(x: f32, y: f32) -> Self {
        Self::new(Align::new(x), Align::new(y))
    }

    #[inline]
    pub const fn splat(a: Align) -> Self {
        Self::new(a, a)
    }

    /// Mirror horizontally -> `TOP_LEFT` becomes `TOP_RIGHT`.
    #[inline]
    pub fn flip_x(self) -> Self {
        Self::new(self.x.flip(), self.y)
    }

    /// Mirror vertically —> `TOP_LEFT` becomes `BOTTOM_LEFT`.
    #[inline]
    pub fn flip_y(self) -> Self {
        Self::new(self.x, self.y.flip())
    }
}
impl Default for Align2 {
    #[inline]
    fn default() -> Self {
        Self::TOP_LEFT
    }
}
impl From<(f32, f32)> for Align2 {
    #[inline]
    fn from((x, y): (f32, f32)) -> Self {
        Self::frac(x, y)
    }
}
impl From<Align> for Align2 {
    #[inline]
    fn from(a: Align) -> Self {
        Self::splat(a)
    }
}

impl Rect {
    /// Resolve a fractional point within this rect.
    ///
    /// `rect.point(Align2::CENTER)` is the centre; `Align2::BOTTOM_RIGHT` is
    /// the far corner. Rounds to the nearest pixel.
    #[inline]
    pub fn point(&self, a: Align2) -> Position<i32> {
        Position::new(
            self.x + (self.w as f32 * a.x.get()).round() as i32,
            self.y + (self.h as f32 * a.y.get()).round() as i32,
        )
    }

    /// Position a box of `size` so that its `origin` point lands on this
    /// rect's `anchor` point.
    #[inline]
    pub fn place(&self, size: Size<i32>, anchor: Align2, origin: Align2) -> Position<i32> {
        let dx = self.w as f32 * anchor.x.get() - size.width as f32 * origin.x.get();
        let dy = self.h as f32 * anchor.y.get() - size.height as f32 * origin.y.get();
        Position::new(self.x + dx.floor() as i32, self.y + dy.floor() as i32)
    }
}

#[cfg(test)]
mod align2_tests {
    use super::*;

    #[test]
    fn corners_and_centre() {
        assert_eq!(Align2::TOP_LEFT, Align2::frac(0.0, 0.0));
        assert_eq!(Align2::CENTER, Align2::splat(Align::CENTER));
        assert_eq!(Align2::BOTTOM_RIGHT, Align2::frac(1.0, 1.0));
        assert_eq!(Align2::default(), Align2::TOP_LEFT);
    }

    #[test]
    fn flips_mirror() {
        assert_eq!(Align2::TOP_LEFT.flip_x(), Align2::TOP_RIGHT);
        assert_eq!(Align2::TOP_LEFT.flip_y(), Align2::BOTTOM_LEFT);
        assert_eq!(Align2::CENTER.flip_x(), Align2::CENTER);
        assert_eq!(Align2::BOTTOM_RIGHT.flip_x().flip_y(), Align2::TOP_LEFT);
    }
}

#[cfg(test)]
mod anchor_tests {
    use super::*;
    use crate::model::Size;

    #[test]
    fn point_resolves_fractions() {
        let r = Rect::new(0, 0, 100, 50);
        assert_eq!(r.point(Align2::TOP_LEFT), Position::new(0, 0));
        assert_eq!(r.point(Align2::CENTER), Position::new(50, 25));
        assert_eq!(r.point(Align2::BOTTOM_RIGHT), Position::new(100, 50));
    }

    #[test]
    fn point_honours_origin_offset() {
        let r = Rect::new(10, 20, 100, 50);
        assert_eq!(r.point(Align2::CENTER), Position::new(60, 45));
    }

    #[test]
    fn point_allows_out_of_range_fractions() {
        let r = Rect::new(0, 0, 100, 100);
        assert_eq!(r.point(Align2::frac(1.5, -0.5)), Position::new(150, -50));
    }

    #[test]
    fn place_top_left_is_plain_corner_placement() {
        let parent = Rect::new(10, 10, 100, 100);
        let pos = parent.place(Size::new(20, 20), Align2::TOP_LEFT, Align2::TOP_LEFT);
        assert_eq!(pos, Position::new(10, 10));
    }

    /// The headline case: a box centred on the parent regardless of size.
    #[test]
    fn place_centre_on_centre() {
        let parent = Rect::new(0, 0, 100, 100);
        let pos = parent.place(Size::new(40, 20), Align2::CENTER, Align2::CENTER);
        assert_eq!(pos, Position::new(30, 40));
    }

    /// A badge whose own centre sits on the parent's top-right corner.
    #[test]
    fn place_centre_origin_on_corner_anchor() {
        let parent = Rect::new(0, 0, 100, 100);
        let pos = parent.place(Size::new(20, 20), Align2::TOP_RIGHT, Align2::CENTER);
        assert_eq!(pos, Position::new(90, -10));
    }

    /// Anchoring a popup below a target: anchor at the target's bottom edge,
    /// origin at the popup's top edge.
    #[test]
    fn place_below_target() {
        let target = Rect::new(50, 200, 80, 30);
        let pos = target.place(Size::new(120, 90), Align2::BOTTOM_LEFT, Align2::TOP_LEFT);
        assert_eq!(pos, Position::new(50, 230));
    }

    #[test]
    fn place_is_independent_of_parent_origin_shift() {
        let a = Rect::new(0, 0, 100, 100);
        let b = Rect::new(7, 13, 100, 100);
        let sz = Size::new(30, 30);
        let pa = a.place(sz, Align2::CENTER, Align2::CENTER);
        let pb = b.place(sz, Align2::CENTER, Align2::CENTER);
        assert_eq!(pb - pa, Position::new(7, 13));
    }

    /// `place` then `clamp_inside` is the whole popup positioner, minus flip.
    #[test]
    fn place_then_clamp_composes() {
        let viewport = Rect::new(0, 0, 400, 300);
        let target = Rect::new(360, 260, 30, 30);
        let popup = Rect::from_parts(
            target.place(Size::new(120, 90), Align2::BOTTOM_LEFT, Align2::TOP_LEFT),
            Size::new(120, 90),
        );
        assert_eq!(popup, Rect::new(360, 290, 120, 90));
        assert_eq!(popup.clamp_inside(viewport), Rect::new(280, 210, 120, 90));
    }

    /// `Rect::place` must reproduce the integer halving in
    /// `LayoutEngine::place`'s `cross_offset` for *every* input, so that
    /// swapping one for the other cannot shift a centred layout by a pixel.
    /// Only checked where the child fits, since `cross_offset` additionally
    /// clamps overflow to zero — a clamp `place` deliberately does not copy.
    #[test]
    fn place_matches_legacy_cross_offset() {
        for inner in 0..64 {
            for child in 0..=inner {
                let legacy = (inner - child) / 2;
                let via_rect = Rect::new(0, 0, inner, 1)
                    .place(Size::new(child, 1), Align2::TOP_CENTER, Align2::TOP_CENTER)
                    .x;
                assert_eq!(legacy, via_rect, "inner={inner} child={child}");
            }
        }
    }

    /// `Align::End` in `cross_offset` is `inner - child`; the `Align2`
    /// spelling is anchor and origin both at the trailing edge.
    #[test]
    fn place_matches_legacy_end_alignment() {
        for inner in 0..64 {
            for child in 0..=inner {
                let legacy = inner - child;
                let via_rect = Rect::new(0, 0, inner, 1)
                    .place(Size::new(child, 1), Align2::TOP_RIGHT, Align2::TOP_RIGHT)
                    .x;
                assert_eq!(legacy, via_rect, "inner={inner} child={child}");
            }
        }
    }
}

#[cfg(test)]
mod align_tests {
    use super::*;

    #[test]
    fn named_positions_are_the_expected_fractions() {
        assert_eq!(Align::START.get(), 0.0);
        assert_eq!(Align::CENTER.get(), 0.5);
        assert_eq!(Align::END.get(), 1.0);
    }

    /// `Align::default()` must stay `START`, since `Node::default()` relies on
    /// it to preserve the old `Align::Start` default.
    #[test]
    fn default_is_start() {
        assert_eq!(Align::default(), Align::START);
    }

    #[test]
    fn flip_mirrors_about_the_centre() {
        assert_eq!(Align::START.flip(), Align::END);
        assert_eq!(Align::END.flip(), Align::START);
        assert_eq!(Align::CENTER.flip(), Align::CENTER);
    }

    #[test]
    fn out_of_range_values_are_preserved() {
        assert_eq!(Align::new(1.5).get(), 1.5);
        assert_eq!(Align::new(-0.5).get(), -0.5);
    }
}

#[cfg(test)]
mod main_tests {
    use super::*;

    /// The old `main_align: Align::Start` default must survive the split.
    #[test]
    fn default_is_leading_edge() {
        assert_eq!(Main::default(), Main::At(Align::START));
    }

    #[test]
    fn align_converts_to_a_position() {
        assert_eq!(Main::from(Align::CENTER), Main::At(Align::CENTER));
        let m: Main = Align::END.into();
        assert_eq!(m, Main::At(Align::END));
    }

    /// `Main::At(a)` computes its lead as `floor(free * a)`. Check it against
    /// the integer arithmetic the old `Align` arms used, for every split of a
    /// realistic amount of free space.
    #[test]
    fn at_matches_legacy_lead() {
        for free in 0..128 {
            let lead = |a: Align| (free as f32 * a.get()).floor() as i32;
            assert_eq!(lead(Align::START), 0, "free={free}");
            assert_eq!(lead(Align::CENTER), free / 2, "free={free}");
            assert_eq!(lead(Align::END), free, "free={free}");
        }
    }
}

#[cfg(test)]
mod length_tests {
    use super::*;

    #[test]
    fn only_fill_carries_weight() {
        assert_eq!(Length::Fill(2.5).weight(), Some(2.5));
        assert_eq!(Length::Fit.weight(), None);
        assert_eq!(Length::Fixed(10).weight(), None);
    }

    #[test]
    fn integers_still_convert_to_fixed() {
        assert_eq!(Length::from(12i32), Length::Fixed(12));
    }

    #[test]
    fn default_is_fit() {
        assert_eq!(Length::default(), Length::Fit);
    }
}
