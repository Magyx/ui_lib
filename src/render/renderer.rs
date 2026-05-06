use wgpu::util::DeviceExt;

use crate::{
    consts::DEFAULT_MAX_INSTANCES,
    graphics::{Globals, Gpu, Target},
    primitive::{Instance, Primitive, QUAD_INDICES, QUAD_VERTICES},
    render::{
        pipeline::{PipelineKey, PipelineRegistry},
        text::TextSystem,
        texture::TextureRegistry,
    },
};

struct DrawCommand<'a> {
    pipe: &'a PipelineKey,
    base: u32,
    amount: u32,
    clip: [u32; 4],
}

pub(crate) struct Renderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    number_of_indices: u32,
    instance_buffer: wgpu::Buffer,

    pub(crate) textures: TextureRegistry,
    pub(crate) text: TextSystem,
}

impl Renderer {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pipeline Vertex Buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pipeline Index Buffer"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let number_of_indices = QUAD_INDICES.len() as u32;

        let instance_buffer = device.create_buffer(&wgpu::wgt::BufferDescriptor {
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
            textures: TextureRegistry::new(device),
            text: TextSystem::default(),
        }
    }

    pub fn render<'a, M>(
        &self,
        gpu: &Gpu,
        target: &Target<'a, M>,
        pipeline_registry: &mut PipelineRegistry,
        globals: &Globals,
        instances: &[Instance],
        primitives: &[Primitive],
    ) -> Result<(), wgpu::SurfaceError> {
        let output = {
            crate::scope!("wgpu:get_current_texture");
            match target.surface.get_current_texture() {
                Ok(o) => o,
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    return Err(wgpu::SurfaceError::Outdated);
                }
                Err(wgpu::SurfaceError::Timeout) => return Ok(()),
                Err(e) => return Err(e),
            }
        };

        crate::scope!("encode+submit");
        let view = &output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let sw = target.config.width;
        let sh = target.config.height;
        let default_clip = [0, 0, sw, sh];

        let mut draw_commands = Vec::new();

        let mut base = 0u32;
        let mut current_key: Option<&PipelineKey> = None;
        let mut current_clip = default_clip;
        for (i, instance) in instances.iter().enumerate() {
            let mut clip = instance.scissor().unwrap_or(default_clip);
            clip[0] = clip[0].min(sw.saturating_sub(1));
            clip[1] = clip[1].min(sh.saturating_sub(1));
            let max_w = sw.saturating_sub(clip[0]);
            let max_h = sh.saturating_sub(clip[1]);
            clip[2] = clip[2].min(max_w);
            clip[3] = clip[3].min(max_h);

            let need_new_segment =
                current_key.map(|k| k != &instance.kind).unwrap_or(true) || clip != current_clip;

            if need_new_segment {
                if let Some(key) = current_key
                    && current_clip[2] > 0
                    && current_clip[3] > 0
                {
                    draw_commands.push(DrawCommand {
                        pipe: key,
                        base,
                        amount: i as u32 - base,
                        clip: current_clip,
                    });
                }
                current_key = Some(&instance.kind);
                current_clip = clip;
                base = i as u32;
            }
        }
        if let Some(key) = current_key {
            draw_commands.push(DrawCommand {
                pipe: key,
                base,
                amount: instances.len() as u32 - base,
                clip: current_clip,
            });
        }

        gpu.queue
            .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(primitives));

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
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

            for command in &draw_commands {
                pipeline_registry.apply_pipeline(
                    command.pipe,
                    globals,
                    self.textures.bind_group(),
                    &mut pass,
                );
                let [x, y, w, h] = command.clip;
                pass.set_scissor_rect(x, y, w.max(1), h.max(1));
                pass.draw_indexed(
                    0..self.number_of_indices,
                    0,
                    command.base..(command.base + command.amount),
                );
            }
        }

        crate::plot!("ui.draw_commands", draw_commands.len() as f64);
        gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
