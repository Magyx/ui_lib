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
    };
}

define_vector!(Vector2, 2, x, y);
define_vector!(Vector3, 3, x, y, z);
define_vector!(Vector4, 4, x, y, z, w);
define_vector!(Size, 2, width, height);
define_vector!(Position, 2, x, y);
define_vector!(Color, 4, r, g, b, a);

impl Color<f32> {
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    }

    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }
}

impl<T> From<(T, T)> for Size<T> {
    fn from((width, height): (T, T)) -> Self {
        Self { width, height }
    }
}

impl Size<i32> {
    pub fn to_f32(self) -> Size<f32> {
        Size::new(self.width as f32, self.height as f32)
    }
}
