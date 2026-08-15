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
    pub fn xywh(&self) -> (i32, i32, i32, i32) {
        (self.x, self.y, self.w, self.h)
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
