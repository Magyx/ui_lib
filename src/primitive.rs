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

    #[inline]
    pub fn add_clip(&mut self, x: i32, y: i32, w: i32, h: i32) {
        if w > 0 && h > 0 {
            self.clip = Some([x.max(0) as u32, y.max(0) as u32, w as u32, h as u32]);
        } else {
            self.clip = Some([0, 0, 0, 0]);
        }
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.primitive.position[0] += dx;
        self.primitive.position[1] += dy;
    }

    pub(crate) fn scissor(&self) -> Option<[u32; 4]> {
        self.clip
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
    fn translate_shifts_position() {
        let mut inst = Instance::ui(Position::new(10.0, 20.0), Size::new(1.0, 1.0), Color::BLACK);
        inst.translate(5.0, -3.0);
        assert_eq!(inst.primitive.position, [15.0, 17.0]);
    }

    #[test]
    fn translate_is_additive() {
        let mut inst = Instance::ui(Position::new(0.0, 0.0), Size::new(1.0, 1.0), Color::BLACK);
        inst.translate(1.0, 2.0);
        inst.translate(3.0, 4.0);
        inst.translate(-1.0, -1.0);
        assert_eq!(inst.primitive.position, [3.0, 5.0]);
    }

    #[test]
    fn translate_does_not_touch_size_or_data() {
        let mut inst = Instance::ui(Position::new(0.0, 0.0), Size::new(40.0, 50.0), Color::RED);
        let before = inst.primitive;
        inst.translate(100.0, 100.0);
        let after = inst.primitive;
        assert_eq!(before.size, after.size);
        assert_eq!(before.data1, after.data1);
        assert_eq!(before.data2, after.data2);
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
}
