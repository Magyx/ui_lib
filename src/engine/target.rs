use std::{sync::Arc, time::Instant};

use super::{Engine, builder::TargetConfig};
use crate::{context::Context, gpu::Globals, model::Size, widget::Element};

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct TargetId(pub(super) u32);

#[derive(Default)]
pub(super) struct TargetIdAlloc {
    next: u32,
}
impl TargetIdAlloc {
    fn alloc(&mut self) -> TargetId {
        let id = TargetId(self.next);
        self.next = self.next.checked_add(1).expect("TargetId overflow");
        id
    }
}

pub struct Target<'a> {
    pub surface: wgpu::Surface<'a>,
    pub config: wgpu::SurfaceConfiguration,
    /// Logical size of the surface (in logical pixels)
    pub size: Size<u32>,
    /// Device-pixel ratio. Physical size = logical size x scale_factor
    pub scale_factor: f64,
    pub globals: Globals,
    pub(super) ctx: Context,

    pub(super) start_time: Instant,
    pub(super) last_frame_time: Instant,
    pub(super) root: Option<Element>,
}

impl<'a> Engine<'a> {
    pub(super) fn create_target<T>(
        &mut self,
        target: Arc<T>,
        physical_size: Size<u32>,
        scale_factor: f64,
        cfg: &TargetConfig,
    ) -> crate::Result<TargetId>
    where
        T: wgpu::rwh::HasWindowHandle
            + wgpu::rwh::HasDisplayHandle
            + Sized
            + std::marker::Sync
            + std::marker::Send
            + 'a,
    {
        let physical_size = physical_size.max(Size::new(1, 1));
        let sf = scale_factor.max(1.0);
        let logical_size = Size::new(
            (physical_size.width as f64 / sf).round() as u32,
            (physical_size.height as f64 / sf).round() as u32,
        )
        .max(Size::new(1, 1));

        let instance = self
            .gpu
            .instance
            .as_ref()
            .ok_or(crate::error::InitError::NoInstance)?;
        let surface = instance
            .create_surface(target.clone())
            .map_err(crate::error::InitError::CreateSurface)?;

        let adapter = self
            .gpu
            .adapter
            .as_ref()
            .ok_or(crate::error::InitError::NoInstance)?;
        let surface_caps = surface.get_capabilities(adapter);
        let format = match cfg.format {
            Some(f) if surface_caps.formats.contains(&f) => f,
            _ => {
                use wgpu::TextureFormat::{Bgra8UnormSrgb, Rgba8UnormSrgb};
                [Bgra8UnormSrgb, Rgba8UnormSrgb]
                    .into_iter()
                    .find(|f| surface_caps.formats.contains(f))
                    .or_else(|| surface_caps.formats.iter().copied().find(|f| f.is_srgb()))
                    .unwrap_or(Bgra8UnormSrgb)
            }
        };

        let present_mode = match cfg.present_mode {
            Some(pm) if surface_caps.present_modes.contains(&pm) => pm,
            _ => {
                if surface_caps
                    .present_modes
                    .contains(&wgpu::PresentMode::AutoVsync)
                {
                    wgpu::PresentMode::AutoVsync
                } else {
                    surface_caps
                        .present_modes
                        .first()
                        .copied()
                        .unwrap_or(wgpu::PresentMode::Fifo)
                }
            }
        };

        let alpha_mode = match cfg.alpha_mode {
            Some(am) if surface_caps.alpha_modes.contains(&am) => am,
            _ => {
                if surface_caps
                    .alpha_modes
                    .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
                {
                    wgpu::CompositeAlphaMode::PreMultiplied
                } else if surface_caps
                    .alpha_modes
                    .contains(&wgpu::CompositeAlphaMode::Inherit)
                {
                    wgpu::CompositeAlphaMode::Inherit
                } else {
                    surface_caps
                        .alpha_modes
                        .first()
                        .copied()
                        .unwrap_or(wgpu::CompositeAlphaMode::PreMultiplied)
                }
            }
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: physical_size.width,
            height: physical_size.height,
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: cfg.max_frame_latency.max(1),
        };

        surface.configure(&self.gpu.device, &config);

        let now = Instant::now();
        let target = Target {
            surface,
            config,
            size: logical_size,
            scale_factor: sf,
            globals: Globals {
                window_size: [logical_size.width as f32, logical_size.height as f32],
                time: 0.0,
                delta_time: 0.0,
                mouse_pos: [0.0, 0.0],
                mouse_buttons: 0,
                frame: 0,
                scale: sf as f32,
                _pad: 0.0,
            },
            ctx: Context::new(),

            start_time: now,
            last_frame_time: now,

            root: None,
        };

        if !self.pipeline_registry.has_default_pipelines() {
            self.pipeline_registry.register_default_pipelines(
                &self.gpu,
                &target.config.format,
                self.renderer.textures.layout(),
                self.immediate_size,
            );
        }
        let fmt = target.config.format;
        for reg in std::mem::take(&mut self.pending_pipelines) {
            self.pipeline_registry.register(
                reg,
                &self.gpu,
                &fmt,
                self.renderer.textures.layout(),
                self.immediate_size,
            );
        }

        let tid = self.target_alloc.alloc();
        self.targets.insert(tid, target);

        if self.primary_target.is_none() {
            self.primary_target = Some(tid);
        }

        Ok(tid)
    }

    #[inline]
    pub(super) fn primary_target_id(&self) -> Option<TargetId> {
        self.primary_target
    }

    #[inline]
    pub(super) fn primary_target(&self) -> Option<&Target<'a>> {
        self.primary_target_id()
            .and_then(|id| self.targets.get(&id))
    }

    pub fn attach_target<T>(
        &mut self,
        target: Arc<T>,
        physical_size: Size<u32>,
        scale_factor: f64,
    ) -> TargetId
    where
        T: wgpu::rwh::HasWindowHandle
            + wgpu::rwh::HasDisplayHandle
            + Sized
            + std::marker::Sync
            + std::marker::Send
            + 'a,
    {
        let cfg = self.target_defaults;
        self.create_target(target, physical_size, scale_factor, &cfg)
            .expect("wgpu: failed to create target surface")
    }
    pub fn attach_target_with<T>(
        &mut self,
        target: Arc<T>,
        physical_size: Size<u32>,
        scale_factor: f64,
        cfg: TargetConfig,
    ) -> TargetId
    where
        T: wgpu::rwh::HasWindowHandle
            + wgpu::rwh::HasDisplayHandle
            + Sized
            + std::marker::Sync
            + std::marker::Send
            + 'a,
    {
        self.create_target(target, physical_size, scale_factor, &cfg)
            .expect("wgpu: failed to create target surface")
    }
    pub fn try_attach_target_with<T>(
        &mut self,
        target: Arc<T>,
        physical_size: Size<u32>,
        scale_factor: f64,
        cfg: TargetConfig,
    ) -> crate::Result<TargetId>
    where
        T: wgpu::rwh::HasWindowHandle
            + wgpu::rwh::HasDisplayHandle
            + Sized
            + std::marker::Sync
            + std::marker::Send
            + 'a,
    {
        self.create_target(target, physical_size, scale_factor, &cfg)
    }
    pub fn detach_target(&mut self, tid: &TargetId) {
        if self.targets.remove(tid).is_some() && self.primary_target == Some(*tid) {
            if self.primary_target == Some(*tid) && !self.targets.is_empty() {
                self.primary_target = self.targets.keys().next().copied();
            } else {
                _ = self.primary_target.take();
            }
        }
    }
}
