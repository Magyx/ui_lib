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
    pub fn new(position: Position<i32>, size: Size<i32>, data1: [u32; 4], data2: [u32; 4]) -> Self {
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
    pub(crate) kind: PipelineKey,
    position: Position<i32>,
    size: Size<i32>,
    data1: [u32; 4],
    data2: [u32; 4],

    clip: Option<[u32; 4]>,
}

impl Instance {
    pub fn new(
        kind: PipelineKey,
        position: Position<i32>,
        size: Size<i32>,
        data1: [u32; 4],
        data2: [u32; 4],
    ) -> Self {
        Self {
            kind,
            position,
            size,
            data1,
            data2,

            clip: None,
        }
    }

    pub fn ui(position: Position<i32>, size: Size<i32>, color: Color) -> Self {
        Self {
            kind: PipelineKey::Ui,
            position,
            size,
            data1: [color.0, 0, 0, 0],
            data2: [0, 0, 0, 0],

            clip: None,
        }
    }

    pub fn ui_tex(
        position: Position<i32>,
        size: Size<i32>,
        tint: Color,
        handle: TextureHandle,
    ) -> Self {
        Self {
            kind: PipelineKey::Ui,
            position,
            size,
            data1: [tint.0, 0, 0, 0],
            data2: [
                handle.slot_gen,
                handle.scale_packed,
                handle.offset_packed,
                0,
            ],

            clip: None,
        }
    }

    pub fn ui_tex_fit(
        position: Position<i32>,
        size: Size<i32>,
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
            kind: PipelineKey::Ui,
            position,
            size,
            data1: [tint.0, 0, 0, 0],
            data2: [
                handle.slot_gen,
                handle.scale_packed,
                handle.offset_packed,
                content_fit,
            ],
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

    pub fn translate(&mut self, dx: i32, dy: i32) {
        self.position.x += dx;
        self.position.y += dy;
    }

    pub(crate) fn scissor(&self) -> Option<[u32; 4]> {
        self.clip
    }

    pub(crate) fn to_primitive(&self) -> Primitive {
        Primitive::new(self.position, self.size, self.data1, self.data2)
    }
}
