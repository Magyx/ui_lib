use wgpu::Surface;

use crate::{
    gpu::{Globals, Gpu},
    primitive::{InstanceStore, Primitive},
    render::{
        pipeline::{DrawCtx, PipelineId, PipelineRegistry},
        texture::TextureRegistry,
    },
};

pub(crate) enum Presented {
    Ok,
    Suboptimal,
    SurfaceLost,
}

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
        surface: &Surface<'a>,
        pipeline_registry: &mut PipelineRegistry,
        globals: &Globals,
        store: &InstanceStore,
    ) -> Presented {
        let mut suboptimal = false;
        let output = {
            crate::scope!("wgpu:get_current_texture");
            match surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(o) => o,
                // Reconfigure and retry next frame.
                wgpu::CurrentSurfaceTexture::Suboptimal(o) => {
                    suboptimal = true;
                    o
                }
                wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                    return Presented::SurfaceLost;
                }
                // Skip this frame.
                wgpu::CurrentSurfaceTexture::Timeout
                | wgpu::CurrentSurfaceTexture::Occluded
                | wgpu::CurrentSurfaceTexture::Validation => return Presented::Ok,
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
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
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
        gpu.queue.present(output);

        crate::plot!("ui.batches", store.batches().len() as f64);

        if !suboptimal {
            Presented::Ok
        } else {
            Presented::Suboptimal
        }
    }
}
