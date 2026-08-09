use wgpu::util::DeviceExt;

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
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
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

pub struct QuadGeometry {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
}

impl QuadGeometry {
    pub fn new(device: &wgpu::Device) -> Self {
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Index Buffer"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertices,
            indices,
            index_count: QUAD_INDICES.len() as u32,
        }
    }

    /// Bind the quad at slot 0 plus the index buffer. Call from
    /// [`Pipeline::bind`](crate::render::pipeline::Pipeline::bind).
    pub fn bind(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
    }

    /// Bind this batch's slice of the shared instance buffer and draw. Call
    /// from [`Pipeline::draw`](crate::render::pipeline::Pipeline::draw).
    ///
    /// Slicing from `byte_offset` makes instance 0 of the draw the batch's
    /// first instance, so the range is always `0..count`.
    pub fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        instances: &wgpu::Buffer,
        byte_offset: u64,
        count: u32,
    ) {
        pass.set_vertex_buffer(1, instances.slice(byte_offset..));
        pass.draw_indexed(0..self.index_count, 0, 0..count);
    }
}
