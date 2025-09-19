use crate::{
    model::{Position, Size},
    render::pipeline::PipelineKey,
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
    pub data1: [f32; 4],
    pub data2: [u32; 4],
}

impl Primitive {
    pub fn new(position: Position<i32>, size: Size<i32>, data1: [f32; 4], data2: [u32; 4]) -> Self {
        Self {
            position: [position.x as f32, position.y as f32],
            size: [size.width as f32, size.height as f32],
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
                    format: wgpu::VertexFormat::Float32x4,
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
    pub(crate) kind: PipelineKey,
    position: Position<i32>,
    size: Size<i32>,
    data1: [f32; 4],
    data2: [u32; 4],
}

impl Instance {
    pub fn new(
        kind: PipelineKey,
        position: Position<i32>,
        size: Size<i32>,
        data1: [f32; 4],
        data2: [u32; 4],
    ) -> Self {
        Self {
            kind,
            position,
            size,
            data1,
            data2,
        }
    }

    pub(crate) fn to_primitive(&self) -> Primitive {
        Primitive::new(self.position, self.size, self.data1, self.data2)
    }
}
