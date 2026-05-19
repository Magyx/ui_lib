use crate::{
    model::{Color, Position, Size},
    render::{
        pipeline::PipelineKey,
        texture::{TextureHandle, pack_unorm4x8},
    },
};

pub const QUAD_VERTICES: &[Vertex] = &[
    Vertex { uv: [0.0, 0.0] },
    Vertex { uv: [1.0, 0.0] },
    Vertex { uv: [0.0, 1.0] },
    Vertex { uv: [1.0, 1.0] },
];
pub const QUAD_INDICES: &[u16] = &[0, 1, 2, 2, 1, 3];

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 10,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Primitive {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub data1: [u32; 4],
    pub data2: [u32; 4],
}

impl Primitive {
    pub fn new(position: Position<f32>, size: Size<f32>, data1: [u32; 4], data2: [u32; 4]) -> Self {
        Self {
            position: [position.x, position.y],
            size: [size.width, size.height],
            data1,
            data2,
        }
    }
}

impl Primitive {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Primitive>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Uint32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint32x4,
                },
            ],
        }
    }
}

pub mod shape_flags {
    pub const HAS_BORDER: u8 = 1 << 0;
    pub const HAS_SHADOW: u8 = 1 << 1;
    pub const GRADIENT_V: u8 = 1 << 2;
}

#[inline]
pub fn pack_shape_params(
    corner_radius: f32,
    border_width: f32,
    shadow_radius: f32,
    flags: u8,
) -> u32 {
    let r = corner_radius.clamp(0.0, 255.0) as u32;
    let b = (border_width * 4.0).clamp(0.0, 255.0) as u32;
    let s = shadow_radius.clamp(0.0, 255.0) as u32;
    r | (b << 8) | (s << 16) | ((flags as u32) << 24)
}

#[inline]
pub fn unpack_shape_params(packed: u32) -> (f32, f32, f32, u8) {
    let r = (packed & 0xFF) as f32;
    let b = ((packed >> 8) & 0xFF) as f32 * 0.25;
    let s = ((packed >> 16) & 0xFF) as f32;
    let flags = ((packed >> 24) & 0xFF) as u8;
    (r, b, s, flags)
}

#[derive(Clone, Copy, Debug)]
pub struct PrimitiveStyle {
    pub fill: Color,
    pub corner_radius: f32,
    pub border_width: f32,
    pub border_color: Color,
    pub shadow_radius: f32,
    pub shadow_color: Color,
    pub gradient_end: Option<Color>,
}

impl PrimitiveStyle {
    /// Minimal style: just a fill color with no rounding, border, or effects.
    pub fn flat(fill: Color) -> Self {
        Self {
            fill,
            corner_radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            shadow_radius: 0.0,
            shadow_color: Color::TRANSPARENT,
            gradient_end: None,
        }
    }

    /// Rounded rectangle with no border.
    pub fn rounded(fill: Color, corner_radius: f32) -> Self {
        Self {
            corner_radius,
            ..Self::flat(fill)
        }
    }

    pub fn with_border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = color;
        self
    }

    pub fn with_shadow(mut self, radius: f32, color: Color) -> Self {
        self.shadow_radius = radius;
        self.shadow_color = color;
        self
    }

    pub fn with_gradient_end(mut self, color: Color) -> Self {
        self.gradient_end = Some(color);
        self
    }

    #[inline]
    fn pack(&self) -> (u32, u32, u32) {
        let mut flags: u8 = 0;
        if self.border_width > 0.0 && self.border_color.a() > 0 {
            flags |= shape_flags::HAS_BORDER;
        }
        if self.shadow_radius > 0.0 && self.shadow_color.a() > 0 {
            flags |= shape_flags::HAS_SHADOW;
        }

        // aux_color: shadow_color if shadow is active, gradient_end if gradient,
        // otherwise transparent. Shadow and gradient share the aux slot;
        // if both are set, shadow takes priority (gradient_end is less critical).
        let aux_color;
        if let Some(end) = self.gradient_end {
            flags |= shape_flags::GRADIENT_V;
            aux_color = end;
        } else if flags & shape_flags::HAS_SHADOW != 0 {
            aux_color = self.shadow_color;
        } else {
            aux_color = Color::TRANSPARENT;
        }

        let shape = pack_shape_params(
            self.corner_radius,
            self.border_width,
            self.shadow_radius,
            flags,
        );
        (shape, self.border_color.0, aux_color.0)
    }
}

#[derive(Debug)]
pub struct Instance {
    pub(crate) primitive: Primitive,
    pub(crate) kind: PipelineKey,
    clip: Option<[u32; 4]>,
}

impl Instance {
    pub fn new(
        kind: PipelineKey,
        position: Position<f32>,
        size: Size<f32>,
        data1: [u32; 4],
        data2: [u32; 4],
    ) -> Self {
        Self {
            primitive: Primitive {
                position: [position.x, position.y],
                size: [size.width, size.height],
                data1,
                data2,
            },
            kind,
            clip: None,
        }
    }

    pub fn ui(position: Position<f32>, size: Size<f32>, color: Color) -> Self {
        Self {
            primitive: Primitive {
                position: [position.x, position.y],
                size: [size.width, size.height],
                data1: [color.0, 0, 0, 0],
                data2: [0, 0, 0, 0],
            },
            kind: PipelineKey::Ui,
            clip: None,
        }
    }

    pub fn ui_tex(
        position: Position<f32>,
        size: Size<f32>,
        tint: Color,
        handle: TextureHandle,
    ) -> Self {
        Self {
            primitive: Primitive {
                position: [position.x, position.y],
                size: [size.width, size.height],
                data1: [tint.0, 0, 0, 0],
                data2: [
                    handle.slot_gen,
                    handle.scale_packed,
                    handle.offset_packed,
                    0,
                ],
            },
            kind: PipelineKey::Ui,
            clip: None,
        }
    }

    pub fn ui_tex_fit(
        position: Position<f32>,
        size: Size<f32>,
        tint: Color,
        handle: TextureHandle,
        content_scale: [f32; 2],
        content_offset: [f32; 2],
    ) -> Self {
        let content_fit = pack_unorm4x8([
            content_scale[0],
            content_scale[1],
            content_offset[0],
            content_offset[1],
        ]);
        Self {
            primitive: Primitive {
                position: [position.x, position.y],
                size: [size.width, size.height],
                data1: [tint.0, 0, 0, 0],
                data2: [
                    handle.slot_gen,
                    handle.scale_packed,
                    handle.offset_packed,
                    content_fit,
                ],
            },
            kind: PipelineKey::Ui,
            clip: None,
        }
    }

    pub fn ui_rounded(
        position: Position<f32>,
        size: Size<f32>,
        fill: Color,
        corner_radius: f32,
        border_width: i32,
        border_color: Color,
    ) -> Self {
        let style = PrimitiveStyle::flat(fill).with_border(border_width as f32, border_color);
        let style = PrimitiveStyle {
            corner_radius,
            ..style
        };
        Self::ui_styled(position, size, style)
    }

    pub fn ui_styled(position: Position<f32>, size: Size<f32>, style: PrimitiveStyle) -> Self {
        let (shape_params, border_packed, aux_packed) = style.pack();
        Self {
            primitive: Primitive {
                position: [position.x, position.y],
                size: [size.width, size.height],
                data1: [style.fill.0, shape_params, border_packed, aux_packed],
                data2: [0, 0, 0, 0],
            },
            kind: PipelineKey::Ui,
            clip: None,
        }
    }

    #[inline]
    pub(crate) fn add_clip(&mut self, x: i32, y: i32, w: i32, h: i32) {
        if w > 0 && h > 0 {
            self.clip = Some([x.max(0) as u32, y.max(0) as u32, w as u32, h as u32]);
        } else {
            self.clip = Some([0, 0, 0, 0]);
        }
    }

    pub fn scissor(&self) -> Option<[u32; 4]> {
        self.clip
    }

    pub fn primitive(&self) -> &Primitive {
        &self.primitive
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Color, Position, Size};

    #[test]
    fn ui_instance_packs_color_into_data1_low_word() {
        let color = Color::rgba(0x11, 0x22, 0x33, 0x44);
        let inst = Instance::ui(Position::new(1.0, 2.0), Size::new(10.0, 20.0), color);
        assert_eq!(inst.primitive.data1[0], color.0);
        assert_eq!(inst.primitive.data1[1], 0);
        assert_eq!(inst.primitive.data2, [0, 0, 0, 0]);
        assert!(inst.clip.is_none());
    }

    #[test]
    fn ui_instance_preserves_position_and_size() {
        let inst = Instance::ui(Position::new(3.0, 7.0), Size::new(50.0, 60.0), Color::WHITE);
        assert_eq!(inst.primitive.position, [3.0, 7.0]);
        assert_eq!(inst.primitive.size, [50.0, 60.0]);
    }

    #[test]
    fn add_clip_normal_rect_sets_scissor() {
        let mut inst = Instance::ui(Position::new(0.0, 0.0), Size::new(1.0, 1.0), Color::WHITE);
        inst.add_clip(5, 10, 100, 200);
        assert_eq!(inst.scissor(), Some([5, 10, 100, 200]));
    }

    #[test]
    fn add_clip_zero_width_becomes_null_clip() {
        let mut inst = Instance::ui(Position::new(0.0, 0.0), Size::new(1.0, 1.0), Color::WHITE);
        // Implementation uses `w > 0 && h > 0`; non-positive means [0;4].
        inst.add_clip(5, 10, 0, 200);
        assert_eq!(inst.scissor(), Some([0, 0, 0, 0]));
    }

    #[test]
    fn add_clip_negative_height_becomes_null_clip() {
        let mut inst = Instance::ui(Position::new(0.0, 0.0), Size::new(1.0, 1.0), Color::WHITE);
        inst.add_clip(5, 10, 100, -1);
        assert_eq!(inst.scissor(), Some([0, 0, 0, 0]));
    }

    #[test]
    fn add_clip_negative_origin_clamps_to_zero() {
        let mut inst = Instance::ui(Position::new(0.0, 0.0), Size::new(1.0, 1.0), Color::WHITE);
        inst.add_clip(-10, -20, 100, 50);
        // x and y are clamped via `.max(0)` before the u32 cast.
        assert_eq!(inst.scissor(), Some([0, 0, 100, 50]));
    }

    #[test]
    fn add_clip_overwrites_previous_clip() {
        let mut inst = Instance::ui(Position::new(0.0, 0.0), Size::new(1.0, 1.0), Color::WHITE);
        inst.add_clip(1, 1, 10, 10);
        inst.add_clip(5, 5, 20, 20);
        assert_eq!(inst.scissor(), Some([5, 5, 20, 20]));
    }

    #[test]
    fn primitive_stores_f32_coords() {
        let inst = Instance::ui(
            Position::new(-7.0, -8.0),
            Size::new(0.0, 0.0),
            Color::TRANSPARENT,
        );
        assert_eq!(inst.primitive.position, [-7.0, -8.0]);
        assert_eq!(inst.primitive.size, [0.0, 0.0]);
    }

    #[test]
    fn subpixel_positions_preserved() {
        let inst = Instance::ui(Position::new(10.3, 20.7), Size::new(5.5, 8.0), Color::WHITE);
        assert_eq!(inst.primitive.position, [10.3, 20.7]);
        assert_eq!(inst.primitive.size, [5.5, 8.0]);
    }

    #[test]
    fn pack_shape_params_roundtrip() {
        let packed = pack_shape_params(8.0, 1.5, 12.0, shape_flags::HAS_BORDER);
        let (r, b, s, f) = unpack_shape_params(packed);
        assert_eq!(r, 8.0);
        assert_eq!(b, 1.5);
        assert_eq!(s, 12.0);
        assert_eq!(f, shape_flags::HAS_BORDER);
    }

    #[test]
    fn pack_shape_params_zero_is_zero() {
        assert_eq!(pack_shape_params(0.0, 0.0, 0.0, 0), 0);
    }

    #[test]
    fn pack_shape_params_clamps_overflow() {
        let packed = pack_shape_params(300.0, 100.0, 999.0, 0xFF);
        let (r, b, s, f) = unpack_shape_params(packed);
        assert_eq!(r, 255.0);
        assert_eq!(b, 63.75); // 255 * 0.25
        assert_eq!(s, 255.0);
        assert_eq!(f, 0xFF);
    }

    #[test]
    fn pack_shape_params_quarter_pixel_precision() {
        // 0.25px border → stored as 1 in the byte
        let packed = pack_shape_params(0.0, 0.25, 0.0, 0);
        let (_, b, _, _) = unpack_shape_params(packed);
        assert_eq!(b, 0.25);

        // 2.75px border
        let packed = pack_shape_params(0.0, 2.75, 0.0, 0);
        let (_, b, _, _) = unpack_shape_params(packed);
        assert_eq!(b, 2.75);
    }

    #[test]
    fn ui_flat_has_zero_shape_params() {
        let inst = Instance::ui(Position::new(0.0, 0.0), Size::new(10.0, 10.0), Color::RED);
        assert_eq!(
            inst.primitive.data1[1], 0,
            "flat ui must have shape_params == 0 for fast path"
        );
        assert_eq!(inst.primitive.data1[2], 0);
        assert_eq!(inst.primitive.data1[3], 0);
    }

    #[test]
    fn ui_rounded_packs_shape_params() {
        let inst = Instance::ui_rounded(
            Position::new(0.0, 0.0),
            Size::new(100.0, 40.0),
            Color::BLUE,
            8.0,
            1,
            Color::WHITE,
        );

        // data1[0] = fill color
        assert_eq!(inst.primitive.data1[0], Color::BLUE.0);

        // data1[1] = shape_params (non-zero because corner_radius=8 and border_width=1)
        let (r, b, s, flags) = unpack_shape_params(inst.primitive.data1[1]);
        assert_eq!(r, 8.0);
        assert_eq!(b, 1.0);
        assert_eq!(s, 0.0);
        assert_eq!(flags & shape_flags::HAS_BORDER, shape_flags::HAS_BORDER);
        assert_eq!(flags & shape_flags::HAS_SHADOW, 0);

        // data1[2] = border color
        assert_eq!(inst.primitive.data1[2], Color::WHITE.0);

        // data2 = no texture
        assert_eq!(inst.primitive.data2, [0, 0, 0, 0]);
    }

    #[test]
    fn ui_rounded_no_border_omits_border_flag() {
        let inst = Instance::ui_rounded(
            Position::new(0.0, 0.0),
            Size::new(50.0, 50.0),
            Color::RED,
            12.0,
            0,
            Color::TRANSPARENT,
        );

        let (r, b, _, flags) = unpack_shape_params(inst.primitive.data1[1]);
        assert_eq!(r, 12.0);
        assert_eq!(b, 0.0);
        assert_eq!(flags & shape_flags::HAS_BORDER, 0);
    }

    #[test]
    fn ui_styled_shadow() {
        let style = PrimitiveStyle::flat(Color::WHITE).with_shadow(16.0, Color::rgba(0, 0, 0, 128));
        let inst = Instance::ui_styled(Position::new(10.0, 20.0), Size::new(100.0, 60.0), style);

        let (_, _, s, flags) = unpack_shape_params(inst.primitive.data1[1]);
        assert_eq!(s, 16.0);
        assert_eq!(flags & shape_flags::HAS_SHADOW, shape_flags::HAS_SHADOW);

        // aux_color = shadow_color
        assert_eq!(inst.primitive.data1[3], Color::rgba(0, 0, 0, 128).0);
    }

    #[test]
    fn ui_styled_gradient() {
        let style = PrimitiveStyle::rounded(Color::RED, 6.0).with_gradient_end(Color::BLUE);
        let inst = Instance::ui_styled(Position::new(0.0, 0.0), Size::new(200.0, 100.0), style);

        let (_, _, _, flags) = unpack_shape_params(inst.primitive.data1[1]);
        assert_eq!(flags & shape_flags::GRADIENT_V, shape_flags::GRADIENT_V);
        assert_eq!(inst.primitive.data1[3], Color::BLUE.0);
    }

    #[test]
    fn primitive_style_flat_produces_zero_shape_params() {
        let style = PrimitiveStyle::flat(Color::GREEN);
        let (shape, border, aux) = style.pack();
        assert_eq!(shape, 0, "flat style must pack to 0 for shader fast path");
        assert_eq!(border, Color::TRANSPARENT.0);
        assert_eq!(aux, Color::TRANSPARENT.0);
    }

    #[test]
    fn ui_styled_preserves_position_and_size() {
        let style = PrimitiveStyle::rounded(Color::WHITE, 10.0).with_border(2.0, Color::BLACK);
        let inst = Instance::ui_styled(Position::new(5.5, 7.5), Size::new(120.0, 80.0), style);
        assert_eq!(inst.primitive.position, [5.5, 7.5]);
        assert_eq!(inst.primitive.size, [120.0, 80.0]);
    }
}
