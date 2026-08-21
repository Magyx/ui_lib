macro_rules! define_vector {
    (
        $name:ident, $dim:expr,
        $( $field:ident ),+
    ) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
        #[repr(C)]
        pub struct $name<T> {
            $( pub $field: T ),+
        }

        impl<T> $name<T> {
            pub const fn new($( $field: T ),+) -> Self {
                Self { $( $field ),+ }
            }

            pub const fn splat(value: T) -> Self where T: Copy {
                Self { $( $field: value ),+ }
            }

            pub fn as_slice(&self) -> &[T] {
                unsafe {
                    std::slice::from_raw_parts(
                        self as *const Self as *const T,
                        $dim
                    )
                }
            }

            pub fn as_slice_mut(&mut self) -> &mut [T] {
                unsafe {
                    std::slice::from_raw_parts_mut(
                        self as *mut Self as *mut T,
                        $dim
                    )
                }
            }
        }

        impl<T> From<[T; $dim]> for $name<T> {
            fn from(arr: [T; $dim]) -> Self {
                let [$( $field ),+] = arr;
                Self { $( $field ),+ }
            }
        }

        impl<T> From<$name<T>> for [T; $dim] {
            fn from(v: $name<T>) -> Self {
                [ $( v.$field ),+ ]
            }
        }

        impl<T> AsRef<[T]> for $name<T> {
            fn as_ref(&self) -> &[T] {
                self.as_slice()
            }
        }

        impl<T> AsMut<[T]> for $name<T> {
            fn as_mut(&mut self) -> &mut [T] {
                self.as_slice_mut()
            }
        }

        impl<T> core::ops::Add for $name<T>
        where
            T: core::ops::Add<Output = T>,
        {
            type Output = Self;
            fn add(self, rhs: Self) -> Self::Output {
                Self { $( $field: self.$field + rhs.$field ),+ }
            }
        }

        impl<T> core::ops::Sub for $name<T>
        where
            T: core::ops::Sub<Output = T>,
        {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self::Output {
                Self { $( $field: self.$field - rhs.$field ),+ }
            }
        }

        impl<T> core::ops::AddAssign for $name<T>
        where
            T: core::ops::AddAssign,
        {
            fn add_assign(&mut self, rhs: Self) {
                $( self.$field += rhs.$field; )+
            }
        }

        impl<T> core::ops::SubAssign for $name<T>
        where
            T: core::ops::SubAssign,
        {
            fn sub_assign(&mut self, rhs: Self) {
                $( self.$field -= rhs.$field; )+
            }
        }

        impl<T> core::ops::Neg for $name<T>
        where
            T: core::ops::Neg<Output = T>,
        {
            type Output = Self;
            fn neg(self) -> Self::Output {
                Self { $( $field: -self.$field ),+ }
            }
        }

        impl<T> core::ops::Add<T> for $name<T>
        where
            T: core::ops::Add<Output = T> + Copy,
        {
            type Output = Self;
            fn add(self, rhs: T) -> Self::Output {
                Self { $( $field: self.$field + rhs ),+ }
            }
        }

        impl<T> core::ops::Sub<T> for $name<T>
        where
            T: core::ops::Sub<Output = T> + Copy,
        {
            type Output = Self;
            fn sub(self, rhs: T) -> Self::Output {
                Self { $( $field: self.$field - rhs ),+ }
            }
        }

        impl<T> core::ops::AddAssign<T> for $name<T>
        where
            T: core::ops::AddAssign + Copy,
        {
            fn add_assign(&mut self, rhs: T) {
                $( self.$field += rhs; )+
            }
        }

        impl<T> core::ops::SubAssign<T> for $name<T>
        where
            T: core::ops::SubAssign + Copy,
        {
            fn sub_assign(&mut self, rhs: T) {
                $( self.$field -= rhs; )+
            }
        }
    };
}

define_vector!(Vec4, 4, x, y, z, w);
define_vector!(Size, 2, width, height);
define_vector!(Position, 2, x, y);

impl<T> From<(T, T)> for Size<T> {
    fn from((width, height): (T, T)) -> Self {
        Self { width, height }
    }
}

impl<T> Size<T> {
    pub fn max(self, other: Size<T>) -> Size<T>
    where
        T: Ord,
    {
        Size {
            width: self.width.max(other.width),
            height: self.height.max(other.height),
        }
    }

    pub fn min(self, other: Size<T>) -> Size<T>
    where
        T: Ord,
    {
        Size {
            width: self.width.min(other.width),
            height: self.height.min(other.height),
        }
    }
}

impl<T> std::ops::Add<Size<T>> for Position<T>
where
    T: core::ops::Add<T, Output = T> + Copy,
{
    type Output = Position<T>;
    fn add(self, rhs: Size<T>) -> Position<T> {
        Self {
            x: self.x + rhs.width,
            y: self.y + rhs.height,
        }
    }
}

impl<T> std::ops::Sub<Size<T>> for Position<T>
where
    T: core::ops::Sub<T, Output = T> + Copy,
{
    type Output = Position<T>;
    fn sub(self, rhs: Size<T>) -> Position<T> {
        Self {
            x: self.x - rhs.width,
            y: self.y - rhs.height,
        }
    }
}

impl<T> std::ops::AddAssign<Size<T>> for Position<T>
where
    T: core::ops::AddAssign<T> + Copy,
{
    fn add_assign(&mut self, rhs: Size<T>) {
        self.x += rhs.width;
        self.y += rhs.height;
    }
}

impl<T> std::ops::SubAssign<Size<T>> for Position<T>
where
    T: core::ops::SubAssign<T> + Copy,
{
    fn sub_assign(&mut self, rhs: Size<T>) {
        self.x -= rhs.width;
        self.y -= rhs.height;
    }
}

/// A per-side inset, in logical pixels.
///
/// One type for every "space around a box" concept: container padding, child
/// margins, and edge-pinning distances for absolutely placed nodes. Field
/// names are physical (`left`/`right`), not logical (`start`/`end`); see the
/// note on RTL in the docs for [`Rect::inset`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Inset {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Inset {
    pub const ZERO: Self = Self::all(0);

    #[inline]
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// The same inset on all four sides.
    #[inline]
    pub const fn all(v: i32) -> Self {
        Self::new(v, v, v, v)
    }

    /// `h` on the left and right, `v` on the top and bottom.
    #[inline]
    pub const fn symmetric(h: i32, v: i32) -> Self {
        Self::new(h, v, h, v)
    }

    /// Left and right only; top and bottom stay zero.
    #[inline]
    pub const fn horizontal(v: i32) -> Self {
        Self::new(v, 0, v, 0)
    }

    /// Top and bottom only; left and right stay zero.
    #[inline]
    pub const fn vertical(v: i32) -> Self {
        Self::new(0, v, 0, v)
    }

    /// Total horizontal inset — the amount a box loses in width.
    #[inline]
    pub const fn width(&self) -> i32 {
        self.left + self.right
    }

    /// Total vertical inset — the amount a box loses in height.
    #[inline]
    pub const fn height(&self) -> i32 {
        self.top + self.bottom
    }

    /// The top-left corner offset, i.e. where a content box begins.
    #[inline]
    pub const fn origin(&self) -> Position<i32> {
        Position::new(self.left, self.top)
    }
}

impl From<i32> for Inset {
    #[inline]
    fn from(v: i32) -> Self {
        Self::all(v)
    }
}

/// `x` → left, `y` → top, `z` → right, `w` → bottom.
impl From<Vec4<i32>> for Inset {
    #[inline]
    fn from(v: Vec4<i32>) -> Self {
        Self::new(v.x, v.y, v.z, v.w)
    }
}

impl From<Inset> for Vec4<i32> {
    #[inline]
    fn from(i: Inset) -> Self {
        Vec4::new(i.left, i.top, i.right, i.bottom)
    }
}

impl From<[i32; 4]> for Inset {
    #[inline]
    fn from(v: [i32; 4]) -> Self {
        Self::new(v[0], v[1], v[2], v[3])
    }
}

impl core::ops::Add for Inset {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(
            self.left + rhs.left,
            self.top + rhs.top,
            self.right + rhs.right,
            self.bottom + rhs.bottom,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    #[inline]
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    #[inline]
    pub fn contains(&self, p: Position<f32>) -> bool {
        let l = self.x as f32;
        let t = self.y as f32;
        let r = l + self.w as f32;
        let b = t + self.h as f32;
        p.x >= l && p.x < r && p.y >= t && p.y < b
    }

    #[inline]
    pub fn intersect(&self, other: &Self) -> Self {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.w).min(other.x + other.w);
        let y1 = (self.y + self.h).min(other.y + other.h);

        Self {
            x: x0,
            y: y0,
            w: (x1 - x0).max(0),
            h: (y1 - y0).max(0),
        }
    }

    #[inline]
    pub fn xywh(&self) -> (i32, i32, i32, i32) {
        (self.x, self.y, self.w, self.h)
    }

    #[inline]
    pub fn from_parts(pos: Position<i32>, size: Size<i32>) -> Self {
        Self::new(pos.x, pos.y, size.width, size.height)
    }

    #[inline]
    pub fn origin(&self) -> Position<i32> {
        Position::new(self.x, self.y)
    }

    #[inline]
    pub fn size(&self) -> Size<i32> {
        Size::new(self.w, self.h)
    }

    /// x coordinate of the right edge (exclusive).
    #[inline]
    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    /// y coordinate of the bottom edge (exclusive).
    #[inline]
    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    /// Shrink by `i` on each side — the content box inside padding.
    ///
    /// Width and height clamp at zero, but the origin still moves by
    /// `left`/`top`, matching CSS behaviour for an over-padded box.
    ///
    /// Note that `Inset` is physical, so this does not mirror under RTL. When
    /// logical insets land, resolve them to physical before calling this.
    #[inline]
    pub fn inset(&self, i: Inset) -> Self {
        Self::new(
            self.x + i.left,
            self.y + i.top,
            (self.w - i.width()).max(0),
            (self.h - i.height()).max(0),
        )
    }

    /// Grow by `i` on each side — the inverse of [`inset`](Self::inset), for
    /// margin boxes and hit-test slop.
    #[inline]
    pub fn outset(&self, i: Inset) -> Self {
        Self::new(
            self.x - i.left,
            self.y - i.top,
            (self.w + i.width()).max(0),
            (self.h + i.height()).max(0),
        )
    }

    #[inline]
    pub fn translate(&self, by: Position<i32>) -> Self {
        Self::new(self.x + by.x, self.y + by.y, self.w, self.h)
    }

    /// Shift (never resize) this rect so it lies within `bounds`.
    ///
    /// When this rect is larger than `bounds` on an axis, the leading edge
    /// wins — the overflow spills off the trailing edge, which is the
    /// behaviour a popup wants when it cannot fit on screen.
    #[inline]
    pub fn clamp_inside(&self, bounds: Self) -> Self {
        let x = self.x.min(bounds.right() - self.w).max(bounds.x);
        let y = self.y.min(bounds.bottom() - self.h).max(bounds.y);
        Self::new(x, y, self.w, self.h)
    }
}

#[cfg(test)]
mod inset_tests {
    use super::*;

    #[test]
    fn constructors() {
        assert_eq!(Inset::all(4), Inset::new(4, 4, 4, 4));
        assert_eq!(Inset::symmetric(8, 2), Inset::new(8, 2, 8, 2));
        assert_eq!(Inset::horizontal(5), Inset::new(5, 0, 5, 0));
        assert_eq!(Inset::vertical(5), Inset::new(0, 5, 0, 5));
        assert_eq!(Inset::ZERO, Inset::default());
    }

    #[test]
    fn totals() {
        let i = Inset::new(1, 2, 3, 4);
        assert_eq!(i.width(), 4);
        assert_eq!(i.height(), 6);
        assert_eq!(i.origin(), Position::new(1, 2));
    }

    /// The `Vec4` mapping must stay `x,y,z,w -> left,top,right,bottom`, since
    /// every existing `.padding(Vec4::new(..))` call site depends on it.
    #[test]
    fn vec4_roundtrip_preserves_side_order() {
        let v = Vec4::new(1, 2, 3, 4);
        let i: Inset = v.into();
        assert_eq!(i, Inset::new(1, 2, 3, 4));
        assert_eq!(Vec4::from(i), v);
    }

    #[test]
    fn from_scalar_and_array() {
        assert_eq!(Inset::from(6), Inset::all(6));
        assert_eq!(Inset::from([1, 2, 3, 4]), Inset::new(1, 2, 3, 4));
    }

    #[test]
    fn add_is_componentwise() {
        assert_eq!(
            Inset::new(1, 2, 3, 4) + Inset::all(1),
            Inset::new(2, 3, 4, 5)
        );
    }
}

#[cfg(test)]
mod rect_tests {
    use super::*;

    #[test]
    fn accessors() {
        let r = Rect::new(10, 20, 30, 40);
        assert_eq!(r.right(), 40);
        assert_eq!(r.bottom(), 60);
        assert_eq!(r.origin(), Position::new(10, 20));
        assert_eq!(r.size(), Size::new(30, 40));
        assert_eq!(Rect::from_parts(r.origin(), r.size()), r);
        assert!(!r.is_empty());
        assert!(Rect::new(0, 0, 0, 5).is_empty());
    }

    #[test]
    fn inset_shrinks_and_moves_origin() {
        let r = Rect::new(0, 0, 100, 100);
        assert_eq!(r.inset(Inset::all(10)), Rect::new(10, 10, 80, 80));
        assert_eq!(
            r.inset(Inset::new(5, 10, 15, 20)),
            Rect::new(5, 10, 80, 70)
        );
    }

    /// Over-inset collapses the size to zero but still moves the origin,
    /// matching CSS content-box behaviour.
    #[test]
    fn inset_clamps_size_not_origin() {
        let r = Rect::new(0, 0, 10, 10);
        assert_eq!(r.inset(Inset::all(50)), Rect::new(50, 50, 0, 0));
    }

    #[test]
    fn outset_is_inverse_of_inset() {
        let r = Rect::new(20, 20, 100, 100);
        let i = Inset::new(1, 2, 3, 4);
        assert_eq!(r.inset(i).outset(i), r);
    }

    #[test]
    fn translate_moves_without_resizing() {
        let r = Rect::new(1, 2, 3, 4);
        assert_eq!(r.translate(Position::new(10, 20)), Rect::new(11, 22, 3, 4));
    }

    #[test]
    fn clamp_inside_pushes_back_from_trailing_edge() {
        let bounds = Rect::new(0, 0, 100, 100);
        let r = Rect::new(90, 90, 30, 30);
        assert_eq!(r.clamp_inside(bounds), Rect::new(70, 70, 30, 30));
    }

    #[test]
    fn clamp_inside_pushes_back_from_leading_edge() {
        let bounds = Rect::new(0, 0, 100, 100);
        let r = Rect::new(-15, -5, 30, 30);
        assert_eq!(r.clamp_inside(bounds), Rect::new(0, 0, 30, 30));
    }

    #[test]
    fn clamp_inside_leaves_fitting_rect_alone() {
        let bounds = Rect::new(0, 0, 100, 100);
        let r = Rect::new(20, 30, 10, 10);
        assert_eq!(r.clamp_inside(bounds), r);
    }

    /// When the rect cannot fit, the leading edge wins and overflow spills off
    /// the trailing edge — what a too-tall popup should do.
    #[test]
    fn clamp_inside_oversized_pins_to_leading_edge() {
        let bounds = Rect::new(0, 0, 100, 100);
        let r = Rect::new(40, 40, 300, 300);
        assert_eq!(r.clamp_inside(bounds), Rect::new(0, 0, 300, 300));
    }

    #[test]
    fn clamp_inside_respects_non_zero_bounds_origin() {
        let bounds = Rect::new(50, 50, 100, 100);
        let r = Rect::new(0, 0, 20, 20);
        assert_eq!(r.clamp_inside(bounds), Rect::new(50, 50, 20, 20));
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec4_new_and_fields() {
        let v = Vec4::new(1, 2, 3, 4);
        assert_eq!(v.x, 1);
        assert_eq!(v.y, 2);
        assert_eq!(v.z, 3);
        assert_eq!(v.w, 4);
    }

    #[test]
    fn vec4_splat() {
        let v = Vec4::splat(7);
        assert_eq!(v, Vec4::new(7, 7, 7, 7));
    }

    #[test]
    fn vec4_add_sub() {
        let a = Vec4::new(1, 2, 3, 4);
        let b = Vec4::new(10, 20, 30, 40);
        assert_eq!(a + b, Vec4::new(11, 22, 33, 44));
        assert_eq!(b - a, Vec4::new(9, 18, 27, 36));
    }

    #[test]
    fn vec4_add_assign() {
        let mut a = Vec4::new(1, 1, 1, 1);
        a += Vec4::new(2, 3, 4, 5);
        assert_eq!(a, Vec4::new(3, 4, 5, 6));
    }

    #[test]
    fn vec4_neg() {
        let a = Vec4::new(1i32, -2, 3, -4);
        assert_eq!(-a, Vec4::new(-1, 2, -3, 4));
    }

    #[test]
    fn vec4_scalar_add() {
        let a = Vec4::new(1, 2, 3, 4);
        assert_eq!(a + 10, Vec4::new(11, 12, 13, 14));
    }

    #[test]
    fn vec4_as_slice_roundtrip() {
        let v = Vec4::new(5u32, 6, 7, 8);
        assert_eq!(v.as_slice(), &[5, 6, 7, 8]);
        let arr: [u32; 4] = v.into();
        assert_eq!(arr, [5, 6, 7, 8]);
        let back: Vec4<u32> = arr.into();
        assert_eq!(back, v);
    }

    #[test]
    fn vec4_as_slice_mut_mutates_fields() {
        let mut v = Vec4::new(0, 0, 0, 0);
        for (i, slot) in v.as_slice_mut().iter_mut().enumerate() {
            *slot = i as i32 + 1;
        }
        assert_eq!(v, Vec4::new(1, 2, 3, 4));
    }

    #[test]
    fn size_min_max() {
        let a = Size::new(10, 20);
        let b = Size::new(15, 5);
        assert_eq!(a.max(b), Size::new(15, 20));
        assert_eq!(a.min(b), Size::new(10, 5));
    }

    #[test]
    fn size_from_tuple() {
        let s: Size<i32> = (3, 4).into();
        assert_eq!(s, Size::new(3, 4));
    }

    #[test]
    fn position_plus_size() {
        let p = Position::new(10, 20);
        let s = Size::new(3, 4);
        assert_eq!(p + s, Position::new(13, 24));
        assert_eq!(p - s, Position::new(7, 16));
    }

    #[test]
    fn position_plus_size_assign() {
        let mut p = Position::new(1, 1);
        p += Size::new(10, 20);
        assert_eq!(p, Position::new(11, 21));
        p -= Size::new(1, 1);
        assert_eq!(p, Position::new(10, 20));
    }
}
