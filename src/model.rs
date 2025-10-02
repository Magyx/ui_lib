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
            pub fn new($( $field: T ),+) -> Self {
                Self { $( $field ),+ }
            }

            pub fn splat(value: T) -> Self where T: Copy {
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

define_vector!(Vec2, 2, x, y);
define_vector!(Vec3, 3, x, y, z);
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
    pub const fn splat(c: u8) -> Self {
        Self::rgba(c, c, c, c)
    }

    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 0xFF)
    }

    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self((r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24))
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
        ((self.0 & 0x00_FF_00_00) >> 16) as u8
    }

    #[inline]
    pub fn g(&self) -> u8 {
        ((self.0 & 0x00_00_FF_00) >> 8) as u8
    }

    #[inline]
    pub fn b(&self) -> u8 {
        (self.0 & 0x00_00_00_FF) as u8
    }

    #[inline]
    pub fn a(&self) -> u8 {
        ((self.0 & 0xFF_00_00_00) >> 24) as u8
    }
}
