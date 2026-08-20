use wgpu::Surface;

use super::attachment::Attachments;
use crate::{
    gpu::{Globals, Gpu},
    model::Rect,
    primitive::{InstanceStore, Primitive},
    render::{
        pipeline::{DepthUse, DrawCtx, FrameCtx, PipelineId, PipelineRegistry},
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
        attachments: &mut Attachments,
        registry: &mut PipelineRegistry,
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

        let Some(config) = registry.pass_config() else {
            // Nothing registered — nothing can draw.
            gpu.queue.present(output);
            return Presented::Ok;
        };

        let target_size = [output.texture.width(), output.texture.height()];
        attachments.ensure(&gpu.device, target_size, config, registry.generation());

        let surface_view = &output
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

        // ---- frame hooks -------------------------------------------------
        // Before the shared pass, with the encoder and the queue. Offscreen
        // passes, compute dispatches, and per-frame uploads happen here.
        let active = PipelineRegistry::active_ids(store.batches());
        {
            crate::scope!("pipeline::frame");
            for &id in &active {
                let Some(pipeline) = registry.get_mut(id) else {
                    continue;
                };
                let mut ctx = FrameCtx {
                    gpu,
                    encoder: &mut encoder,
                    globals,
                    store,
                    textures: &mut self.textures,
                    pass: config,
                    target_size,
                    id,
                };
                pipeline.frame(&mut ctx);
            }
        }

        // Depth slices: count the batches that asked to be isolated, then hand
        // them descending ranges so later-painted widgets sit nearer the
        // camera and cannot be rejected by an earlier widget's depth.
        let isolated: Vec<usize> = store
            .batches()
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                registry
                    .requirements_of(b.id)
                    .is_some_and(|r| r.isolate_depth && r.depth != DepthUse::None)
            })
            .map(|(i, _)| i)
            .collect();
        let slice = if isolated.is_empty() {
            1.0
        } else {
            1.0 / isolated.len() as f32
        };

        {
            let color_attachment = match attachments.msaa_view() {
                // Multisampled: draw into the MSAA texture, resolve to the
                // surface.
                Some(msaa) => wgpu::RenderPassColorAttachment {
                    view: msaa,
                    depth_slice: None,
                    resolve_target: Some(surface_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Discard,
                    },
                },
                None => wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                },
            };

            let depth_attachment =
                attachments
                    .depth_view()
                    .map(|view| wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: if config.depth_read_only {
                                wgpu::StoreOp::Discard
                            } else {
                                wgpu::StoreOp::Store
                            },
                        }),
                        stencil_ops: None,
                    });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: depth_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            let ctx = DrawCtx {
                globals,
                textures: self.textures.bind_group(),
                instances: &self.instance_buffer,
                store,
                depth: attachments.depth_bind(),
                pass: config,
                target_size,
            };

            let sf = globals.scale;
            let lw = globals.window_size[0].ceil() as i32;
            let lh = globals.window_size[1].ceil() as i32;
            let default_clip = Rect::new(0, 0, lw, lh);
            let mut bound: Option<PipelineId> = None;
            let mut depth_range: Option<(f32, f32)> = None;
            let mut isolated_seen = 0usize;

            for (i, batch) in store.batches().iter().enumerate() {
                let Rect { x, y, w, h } = batch.clip.unwrap_or(default_clip);
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

                // Depth range for this batch. Reverse order: the first
                // isolated batch gets the far slice.
                let want_range = if config.has_depth() {
                    if isolated.contains(&i) {
                        let k = isolated.len() - 1 - isolated_seen;
                        isolated_seen += 1;
                        Some((k as f32 * slice, (k as f32 + 1.0) * slice))
                    } else {
                        Some((0.0, 1.0))
                    }
                } else {
                    None
                };

                if let Some(range) = want_range
                    && depth_range != Some(range)
                {
                    pass.set_viewport(
                        0.0,
                        0.0,
                        target_size[0] as f32,
                        target_size[1] as f32,
                        range.0,
                        range.1,
                    );
                    depth_range = Some(range);
                }

                // Geometry and bind groups only change when the pipeline does;
                // consecutive batches differing only by scissor skip this.
                let rebind = bound != Some(batch.id);
                pass.set_scissor_rect(px, py, pw, ph);
                if registry.draw_batch(batch.id, &ctx, &mut pass, batch, rebind) {
                    bound = Some(batch.id);
                }
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
