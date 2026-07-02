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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Color(pub u32);

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const WHITE: Self = Self::rgba(255, 255, 255, 255);
    pub const BLACK: Self = Self::rgba(0, 0, 0, 255);
    pub const RED: Self = Self::rgba(255, 0, 0, 255);
    pub const GREEN: Self = Self::rgba(0, 255, 0, 255);
    pub const BLUE: Self = Self::rgba(0, 0, 255, 255);

    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 0xFF)
    }
    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self((r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24))
    }
    #[inline]
    pub const fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as u8;
        let g = ((hex >> 8) & 0xFF) as u8;
        let b = (hex & 0xFF) as u8;

        Self::rgb(r, g, b)
    }
    #[inline]
    pub const fn from_hexa(hex: u32) -> Self {
        let r = ((hex >> 24) & 0xFF) as u8;
        let g = ((hex >> 16) & 0xFF) as u8;
        let b = ((hex >> 8) & 0xFF) as u8;
        let a = (hex & 0xFF) as u8;

        Self::rgba(r, g, b, a)
    }

    #[inline]
    pub fn as_rgba_tuple(self) -> (u8, u8, u8, u8) {
        (self.r(), self.g(), self.b(), self.a())
    }

    #[inline]
    pub fn as_rgba(self) -> [u8; 4] {
        [self.r(), self.g(), self.b(), self.a()]
    }

    #[inline]
    pub fn r(&self) -> u8 {
        (self.0 & 0x00_00_00_FF) as u8
    }

    #[inline]
    pub fn g(&self) -> u8 {
        ((self.0 & 0x00_00_FF_00) >> 8) as u8
    }

    #[inline]
    pub fn b(&self) -> u8 {
        ((self.0 & 0x00_FF_00_00) >> 16) as u8
    }

    #[inline]
    pub fn a(&self) -> u8 {
        ((self.0 & 0xFF_00_00_00) >> 24) as u8
    }
}

use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    Monospace,
    SansSerif,
    Serif,
    Name(Cow<'static, str>),
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

    #[test]
    fn color_constants() {
        assert_eq!(Color::TRANSPARENT.as_rgba(), [0, 0, 0, 0]);
        assert_eq!(Color::WHITE.as_rgba(), [255, 255, 255, 255]);
        assert_eq!(Color::BLACK.as_rgba(), [0, 0, 0, 255]);
        assert_eq!(Color::RED.as_rgba(), [255, 0, 0, 255]);
        assert_eq!(Color::GREEN.as_rgba(), [0, 255, 0, 255]);
        assert_eq!(Color::BLUE.as_rgba(), [0, 0, 255, 255]);
    }

    #[test]
    fn color_rgb_defaults_alpha_to_ff() {
        let c = Color::rgb(10, 20, 30);
        assert_eq!(c.a(), 0xFF);
        assert_eq!(c.r(), 10);
        assert_eq!(c.g(), 20);
        assert_eq!(c.b(), 30);
    }

    #[test]
    fn color_rgba_channel_order_is_abgr_packed_u32() {
        // r | g<<8 | b<<16 | a<<24
        let c = Color::rgba(0x11, 0x22, 0x33, 0x44);
        assert_eq!(c.0, 0x44_33_22_11);
        assert_eq!(c.as_rgba_tuple(), (0x11, 0x22, 0x33, 0x44));
    }

    #[test]
    fn color_channel_extraction_at_boundaries() {
        let c = Color::rgba(0, 255, 0, 255);
        assert_eq!(c.r(), 0);
        assert_eq!(c.g(), 255);
        assert_eq!(c.b(), 0);
        assert_eq!(c.a(), 255);
    }
}
