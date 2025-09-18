use wgpu::util::DeviceExt;

use crate::{
    consts::DEFAULT_MAX_INSTANCES,
    graphics::{Config, Globals},
    primitive::{Instance, Primitive, QUAD_INDICES, QUAD_VERTICES},
    render::pipeline::{PipelineKey, PipelineRegistry},
};

struct DrawCommand<'a> {
    pipe: &'a PipelineKey,
    base: u32,
    amount: u32,
}

pub(crate) struct Renderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    number_of_indices: u32,
    instance_buffer: wgpu::Buffer,
}

impl Renderer {
    pub(crate) fn new(config: &Config) -> Self {
        let vertex_buffer = config
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Pipeline Vertex Buffer"),
                contents: bytemuck::cast_slice(QUAD_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = config
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Pipeline Index Buffer"),
                contents: bytemuck::cast_slice(QUAD_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            });
        let number_of_indices = QUAD_INDICES.len() as u32;

        let instance_buffer = config.device.create_buffer(&wgpu::wgt::BufferDescriptor {
            label: Some("Pipeline Instance Buffer"),
            size: std::mem::size_of::<Primitive>() as u64 * DEFAULT_MAX_INSTANCES,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            index_buffer,
            number_of_indices,
            instance_buffer,
        }
    }

    pub fn render(
        &self,
        config: &Config,
        pipeline_registry: &PipelineRegistry,
        globals: &Globals,
        instances: &[Instance],
    ) -> Result<(), wgpu::SurfaceError> {
        let output = match config.surface.get_current_texture() {
            Ok(o) => o,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                config.surface.configure(&config.device, &config.config);
                config.surface.get_current_texture()?
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(e) => return Err(e),
        };

        let view = &output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = config
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut draw_commands = Vec::<DrawCommand>::new();
        let mut primitives = Vec::<Primitive>::with_capacity(instances.len());

        let mut base = 0u32;
        let mut current_key: Option<&PipelineKey> = None;
        for (i, instance) in instances.iter().enumerate() {
            primitives.push(instance.to_primitive());

            if current_key.is_none() {
                current_key = Some(&instance.kind);
                base = i as u32;
            } else if let Some(key) = current_key
                && key != &instance.kind
            {
                draw_commands.push(DrawCommand {
                    pipe: key,
                    base,
                    amount: i as u32 - base,
                });
                current_key = Some(&instance.kind);
                base = i as u32;
            }
        }
        if let Some(key) = current_key {
            draw_commands.push(DrawCommand {
                pipe: key,
                base,
                amount: instances.len() as u32 - base,
            });
        }

        config.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(primitives.as_slice()),
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            for command in draw_commands.iter() {
                pipeline_registry.apply_pipeline(command.pipe, &mut pass);
                pass.set_push_constants(
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                    0,
                    bytemuck::bytes_of(globals),
                );
                pass.draw_indexed(
                    0..self.number_of_indices,
                    0,
                    command.base..(command.base + command.amount),
                );
            }
        }

        config.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
