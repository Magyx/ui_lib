use crate::{
    graphics::{Config, Globals},
    model::{Color, Position, Size, Vector4},
    utils,
};
use std::path::PathBuf;
use wgpu::util::DeviceExt;

const DEFAULT_MAX_INSTANCES: u64 = 10_000;
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

pub(crate) struct PrimitiveBundle {
    shader_path: PathBuf,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: Option<wgpu::Buffer>,
    instance_buffer: wgpu::Buffer,

    num_indices: u32,
    num_instances: u32,
}

impl PrimitiveBundle {
    pub fn primitive(
        config: &Config,
        texture_array_layout: &wgpu::BindGroupLayout,
        max_instances: Option<u64>,
    ) -> PrimitiveBundle {
        Self::new(
            "Primitive",
            std::path::Path::new("ui/src/shaders/primitive_shader.wgsl"),
            QUAD_VERTICES,
            QUAD_INDICES,
            max_instances.unwrap_or(DEFAULT_MAX_INSTANCES),
            config,
            texture_array_layout,
        )
    }

    pub fn new(
        name: &str,
        shader_path: &std::path::Path,
        vertices: &[Vertex],
        indices: &[u16],
        max_instances: u64,
        config: &Config,
        texture_array_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader_module = utils::wgsl::load_wgsl(&config.device, shader_path, name);

        let render_pipeline_layout =
            config
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Primitive Render Pipeline Layout"),
                    bind_group_layouts: &[texture_array_layout],
                    push_constant_ranges: &[wgpu::PushConstantRange {
                        stages: wgpu::ShaderStages::VERTEX,
                        range: 0..std::mem::size_of::<Globals>() as u32,
                    }],
                });
        let render_pipeline =
            config
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Primitive Render Pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader_module,
                        entry_point: Some("vs_main"),
                        buffers: &[Vertex::desc(), Primitive::desc()],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader_module,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: config.config.format,
                            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                    cache: None,
                });

        let vertex_buffer = config
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Primitive Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = if indices.is_empty() {
            None
        } else {
            Some(
                config
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Primitive Index Buffer"),
                        contents: bytemuck::cast_slice(indices),
                        usage: wgpu::BufferUsages::INDEX,
                    }),
            )
        };

        let instance_buffer = config.device.create_buffer(&wgpu::wgt::BufferDescriptor {
            label: Some("Primitive Instance Buffer"),
            size: std::mem::size_of::<Primitive>() as u64 * max_instances,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let num_indices = if !indices.is_empty() {
            indices.len()
        } else {
            vertices.len()
        } as u32;

        Self {
            shader_path: shader_path.to_path_buf(),
            render_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,

            num_indices,
            num_instances: 0,
        }
    }

    pub fn reload(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        texture_array_layout: &wgpu::BindGroupLayout,
    ) {
        let shader_module = utils::wgsl::load_wgsl(device, &self.shader_path, "Primitive");
        self.render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Primitive Render Pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Primitive Layout"),
                    bind_group_layouts: &[texture_array_layout],
                    push_constant_ranges: &[wgpu::PushConstantRange {
                        stages: wgpu::ShaderStages::VERTEX,
                        range: 0..std::mem::size_of::<Globals>() as u32,
                    }],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc(), Primitive::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
    }

    pub(crate) fn update(&mut self, queue: &wgpu::Queue, instances: &[Primitive]) {
        self.num_instances = instances.len() as u32;
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
    }

    pub(crate) fn render(
        &self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        globals: &Globals,
        texture_bind_group: &wgpu::BindGroup,
        clear_color: &mut Option<wgpu::Color>,
    ) {
        if self.num_instances <= 0 {
            return;
        }

        let load = if let Some(clear_color) = clear_color.take() {
            wgpu::LoadOp::Clear(clear_color)
        } else {
            wgpu::LoadOp::Load
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Primitive Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, bytemuck::bytes_of(globals));

        if let Some(index_buffer) = self.index_buffer.as_ref() {
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..self.num_instances);
        } else {
            render_pass.draw(0..self.num_indices, 0..self.num_instances);
        }
    }
}
