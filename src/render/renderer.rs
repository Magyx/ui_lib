use crate::{
    graphics::{Globals, Gpu, Target},
    primitive::{InstanceStore, Primitive},
    render::{
        pipeline::{DrawCtx, PipelineId, PipelineRegistry},
        texture::TextureRegistry,
    },
};

struct DrawCommand {
    id: PipelineId,
    base: u32,
    amount: u32,
    clip: [u32; 4],
}

pub(crate) struct Renderer {
    instance_buffer: wgpu::Buffer,
    instance_capacity: u64,
    instance_stride: u64,

    pub(crate) textures: TextureRegistry,

    draw_buf: Vec<DrawCommand>,
}

impl Renderer {
    pub(crate) fn with_capacity(device: &wgpu::Device, max_instances: u64) -> Self {
        let max_instances = max_instances.max(1);
        let instance_buffer = device.create_buffer(&wgpu::wgt::BufferDescriptor {
            label: Some("Pipeline Instance Buffer"),
            size: std::mem::size_of::<Primitive>() as u64 * max_instances,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            instance_capacity: max_instances,
            instance_stride: std::mem::size_of::<Primitive>() as u64,
            instance_buffer,
            textures: TextureRegistry::new(device),
            draw_buf: Vec::new(),
        }
    }

    fn ensure_instance_capacity(
        &mut self,
        device: &wgpu::Device,
        needed: u64,
        stride: u64,
    ) -> bool {
        debug_assert_eq!(
            stride, self.instance_stride,
            "instance buffer was allocated for a different instance type"
        );

        if needed <= self.instance_capacity {
            return false;
        }

        let new_cap = needed.max(1).next_power_of_two();
        self.instance_buffer = device.create_buffer(&wgpu::wgt::BufferDescriptor {
            label: Some("Pipeline Instance Buffer"),
            size: stride * new_cap,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.instance_capacity = new_cap;

        #[cfg(feature = "tracing")]
        tracing::debug!(new_cap, needed, "instance buffer grown");

        true
    }

    pub fn render<'a>(
        &mut self,
        gpu: &Gpu,
        target: &Target<'a>,
        pipeline_registry: &mut PipelineRegistry,
        globals: &Globals,
        store: &InstanceStore,
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

        let lw = globals.window_size[0].ceil() as u32;
        let lh = globals.window_size[1].ceil() as u32;
        let default_clip = [0, 0, lw, lh];

        self.draw_buf.clear();
        let mut base = 0u32;
        let mut current_key: Option<PipelineId> = None;
        let mut current_clip = default_clip;
        for (i, instance) in store.meta().iter().enumerate() {
            let mut clip = instance.clip.unwrap_or(default_clip);
            clip[0] = clip[0].min(lw.saturating_sub(1));
            clip[1] = clip[1].min(lh.saturating_sub(1));
            clip[2] = clip[2].min(lw.saturating_sub(clip[0]));
            clip[3] = clip[3].min(lh.saturating_sub(clip[1]));

            let need_new_segment = current_key != Some(instance.id) || clip != current_clip;

            if need_new_segment {
                if let Some(key) = current_key
                    && current_clip[2] > 0
                    && current_clip[3] > 0
                {
                    self.draw_buf.push(DrawCommand {
                        id: key,
                        base,
                        amount: i as u32 - base,
                        clip: current_clip,
                    });
                }
                current_key = Some(instance.id);
                current_clip = clip;
                base = i as u32;
            }
        }
        if let Some(key) = current_key {
            self.draw_buf.push(DrawCommand {
                id: key,
                base,
                amount: store.len() as u32 - base,
                clip: current_clip,
            });
        }

        self.ensure_instance_capacity(&gpu.device, store.len() as u64, store.stride());

        gpu.queue
            .write_buffer(&self.instance_buffer, 0, store.bytes());

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

            let ctx = DrawCtx {
                globals,
                textures: self.textures.bind_group(),
                instances: &self.instance_buffer,
            };

            let mut bound: Option<PipelineId> = None;
            for command in &self.draw_buf {
                let Some(pipeline) = pipeline_registry.get_mut(command.id) else {
                    debug_assert!(false, "batch emitted for an unregistered pipeline");
                    continue;
                };

                // Geometry and bind groups only change when the pipeline does;
                // consecutive batches differing only by scissor skip this.
                if bound != Some(command.id) {
                    pipeline.bind(&ctx, &mut pass);
                    bound = Some(command.id);
                }

                let sf = globals.scale;
                let [x, y, w, h] = command.clip;
                let px = (x as f32 * sf) as u32;
                let py = (y as f32 * sf) as u32;
                let pw = (w as f32 * sf).ceil() as u32;
                let ph = (h as f32 * sf).ceil() as u32;
                pass.set_scissor_rect(px, py, pw.max(1), ph.max(1));

                pipeline.draw(&mut pass, command.base..(command.base + command.amount));
            }
        }

        crate::plot!("ui.draw_commands", self.draw_buf.len() as f64);
        gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
