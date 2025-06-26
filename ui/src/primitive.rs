use crate::model::{Color, Position, Size, Vector4};

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

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveKind {
    Solid = 0,
    Atlas = 1,
    Texture = 2,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Primitive {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub kind: u32,
    pub tex_id: u32,
    pub color_or_uv: [f32; 4],
    pub border_color: [f32; 4],
    pub border_radius: [f32; 4],
    pub border_width: [f32; 4],

    _padding: [f32; 2],
}

impl Primitive {
    pub fn color(
        position: Position<i32>,
        size: Size<i32>,
        fill_color: Color<f32>,
        border_radius: Vector4<f32>,
        border_color: Color<f32>,
        border_width: Vector4<i32>,
    ) -> Self {
        Self {
            position: [position.x as f32, position.y as f32],
            size: [size.width as f32, size.height as f32],
            kind: PrimitiveKind::Solid as u32,
            tex_id: 0,
            color_or_uv: [fill_color.r, fill_color.g, fill_color.b, fill_color.a],
            border_color: [
                border_color.r,
                border_color.g,
                border_color.b,
                border_color.a,
            ],
            border_radius: [
                border_radius.x,
                border_radius.y,
                border_radius.z,
                border_radius.w,
            ],
            border_width: [
                border_width.x as f32,
                border_width.y as f32,
                border_width.z as f32,
                border_width.w as f32,
            ],

            _padding: [0.0; 2],
        }
    }

    pub fn texture(
        position: Position<i32>,
        size: Size<i32>,
        tex_id: u32,
        uv: [f32; 4],
        border_radius: Vector4<f32>,
        border_color: Color<f32>,
        border_width: Vector4<i32>,
    ) -> Self {
        Self {
            position: [position.x as f32, position.y as f32],
            size: [size.width as f32, size.height as f32],
            kind: PrimitiveKind::Atlas as u32,
            tex_id,
            color_or_uv: uv,
            border_color: [
                border_color.r,
                border_color.g,
                border_color.b,
                border_color.a,
            ],
            border_radius: [
                border_radius.x,
                border_radius.y,
                border_radius.z,
                border_radius.w,
            ],
            border_width: [
                border_width.x as f32,
                border_width.y as f32,
                border_width.z as f32,
                border_width.w as f32,
            ],

            _padding: [0.0; 2],
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
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: 20,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 40,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 56,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 72,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
