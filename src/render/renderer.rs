use crate::{
    graphics::{Globals, Gpu, Target},
    primitive::{InstanceStore, Primitive},
    render::{
        pipeline::{DrawCtx, PipelineId, PipelineRegistry},
        texture::TextureRegistry,
    },
};

pub(crate) struct Renderer {
    instance_buffer: wgpu::Buffer,
    instance_capacity: u64,

    pub(crate) textures: TextureRegistry,
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
            instance_capacity: max_instances * std::mem::size_of::<Primitive>() as u64,
            instance_buffer,
            textures: TextureRegistry::new(device),
        }
    }

    fn ensure_instance_capacity(&mut self, device: &wgpu::Device, needed: u64) -> bool {
        if needed <= self.instance_capacity {
            return false;
        }

        let new_cap = needed.max(1).next_power_of_two();
        self.instance_buffer = device.create_buffer(&wgpu::wgt::BufferDescriptor {
            label: Some("Pipeline Instance Buffer"),
            size: new_cap,
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

        self.ensure_instance_capacity(&gpu.device, store.bytes().len() as u64);

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

            let sf = globals.scale;
            let lw = globals.window_size[0].ceil() as i32;
            let lh = globals.window_size[1].ceil() as i32;
            let default_clip = [0, 0, lw, lh];
            let mut bound: Option<PipelineId> = None;
            for batch in store.batches() {
                let [x, y, w, h] = batch.clip.unwrap_or(default_clip);
                let left = x.clamp(0, lw);
                let top = y.clamp(0, lh);
                let right = (x + w).clamp(0, lw);
                let bottom = (y + h).clamp(0, lh);

                let px = (left as f32 * sf) as u32;
                let py = (top as f32 * sf) as u32;
                let pw = ((right as f32 * sf).ceil() as u32).saturating_sub(px);
                let ph = ((bottom as f32 * sf).ceil() as u32).saturating_sub(py);

                if pw == 0 || ph == 0 {
                    continue;
                }
                let Some(pipeline) = pipeline_registry.get_mut(batch.id) else {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("batch emitted for an unregistered pipeline: {:?}", batch.id);
                    debug_assert!(false, "batch emitted for an unregistered pipeline");
                    continue;
                };

                // Geometry and bind groups only change when the pipeline does;
                // consecutive batches differing only by scissor skip this.
                if bound != Some(batch.id) {
                    pipeline.bind(&ctx, &mut pass);
                    bound = Some(batch.id);
                }
                pass.set_scissor_rect(px, py, pw, ph);
                pipeline.draw(&ctx, &mut pass, batch.byte_offset as u64, batch.count);
            }
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        crate::plot!("ui.batches", store.batches().len() as f64);

        Ok(())
    }
}
