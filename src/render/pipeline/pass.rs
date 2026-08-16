use crate::gpu::Gpu;

/// How a pipeline uses the depth attachment. Ordered: merging takes the max.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub enum DepthUse {
    /// No depth attachment needed. Pipelines built with `depth_stencil: None`.
    #[default]
    None,
    /// Test against depth but never write it. The engine can then attach depth
    /// read-only, which is what makes the depth texture legal to *sample* in
    /// the same pass.
    Read,
    /// Test and write.
    Write,
}

/// What a pipeline needs from the shared render pass.
///
/// Returned by [`Pipeline::requirements`](super::Pipeline::requirements). Must
/// be a pure function of the pipeline *type* — it is queried before any
/// instance exists.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct PassRequirements {
    pub depth: DepthUse,
    /// Request an independent depth range per batch.
    ///
    /// One shared depth buffer means two overlapping widgets fight: the first
    /// writes depth, the second is rejected, and painter's order silently
    /// breaks. Setting this makes the renderer hand each of this pipeline's
    /// batches its own slice of `[0, 1]` via the viewport depth range, in
    /// reverse batch order, so later-painted widgets sit nearer the camera.
    /// Occlusion stays correct *within* a batch and is impossible *between*
    /// batches.
    ///
    /// Ignored when `depth == DepthUse::None`.
    pub isolate_depth: bool,
    pub stencil: bool,
    /// Minimum sample count. `0` or `1` means "don't care". The merged value
    /// is clamped to what the adapter supports for the surface format.
    pub sample_count: u32,
    /// Preferred depth format. `None` lets the engine choose. Two pipelines
    /// naming *different* formats is a configuration error, reported at
    /// registration rather than as a wgpu validation failure mid-frame.
    pub depth_format: Option<wgpu::TextureFormat>,
}

impl PassRequirements {
    pub const NONE: Self = Self {
        depth: DepthUse::None,
        isolate_depth: false,
        stencil: false,
        sample_count: 0,
        depth_format: None,
    };

    /// Test and write depth, isolated per batch. The right default for any
    /// widget that renders a 3D scene into its own rect.
    pub const DEPTH: Self = Self {
        depth: DepthUse::Write,
        isolate_depth: true,
        stencil: false,
        sample_count: 0,
        depth_format: None,
    };

    pub const fn with_depth(mut self, depth: DepthUse) -> Self {
        self.depth = depth;
        self
    }
    pub const fn with_sample_count(mut self, n: u32) -> Self {
        self.sample_count = n;
        self
    }
    pub const fn with_stencil(mut self) -> Self {
        self.stencil = true;
        self
    }

    /// Fold `other` in. Returns `Err` on a genuine conflict.
    pub fn merge(self, other: Self) -> Result<Self, PassConflict> {
        if let (Some(a), Some(b)) = (self.depth_format, other.depth_format)
            && a != b
        {
            return Err(PassConflict::DepthFormat(a, b));
        }

        Ok(Self {
            depth: self.depth.max(other.depth),
            isolate_depth: self.isolate_depth || other.isolate_depth,
            stencil: self.stencil || other.stencil,
            sample_count: self.sample_count.max(other.sample_count),
            depth_format: self.depth_format.or(other.depth_format),
        })
    }

    /// Does satisfying `self` also satisfy `other`? Used to decide whether a
    /// newly registered pipeline forces a rebuild.
    pub fn covers(self, other: Self) -> bool {
        self.merge(other).is_ok_and(|m| m == self)
    }

    /// Turn requirements into the concrete pass description, clamped to what
    /// the adapter actually supports.
    pub fn resolve(self, gpu: &Gpu, color_format: wgpu::TextureFormat) -> PassConfig {
        let depth_format = match (self.depth, self.stencil) {
            (DepthUse::None, false) => None,
            (_, true) => Some(
                self.depth_format
                    .unwrap_or(wgpu::TextureFormat::Depth24PlusStencil8),
            ),
            (_, false) => Some(
                self.depth_format
                    .unwrap_or(wgpu::TextureFormat::Depth32Float),
            ),
        };

        // An injected device (`EngineBuilder::with_wgpu`) has no adapter, so we
        // cannot query format features — stay at 1x rather than guess.
        let sample_count = match (self.sample_count, gpu.adapter.as_ref()) {
            (0 | 1, _) | (_, None) => 1,
            (want, Some(adapter)) => {
                let color_ok = adapter.get_texture_format_features(color_format).flags;
                let depth_ok = depth_format
                    .map(|f| adapter.get_texture_format_features(f).flags)
                    .unwrap_or(color_ok);
                // Walk down to the highest supported count <= want.
                [8u32, 4, 2]
                    .into_iter()
                    .find(|&n| {
                        n <= want
                            && color_ok.sample_count_supported(n)
                            && depth_ok.sample_count_supported(n)
                    })
                    .unwrap_or(1)
            }
        };

        PassConfig {
            color_format,
            depth_format,
            depth_read_only: self.depth != DepthUse::Write,
            sample_count,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PassConflict {
    DepthFormat(wgpu::TextureFormat, wgpu::TextureFormat),
}

impl std::fmt::Display for PassConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DepthFormat(a, b) => write!(
                f,
                "two pipelines require incompatible depth formats ({a:?} and {b:?}); \
                 leave `depth_format: None` on one of them to accept the engine's choice"
            ),
        }
    }
}
impl std::error::Error for PassConflict {}

/// The resolved description of the shared render pass. Every pipeline is built
/// against this, and every attachment is created from it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct PassConfig {
    pub color_format: wgpu::TextureFormat,
    pub depth_format: Option<wgpu::TextureFormat>,
    /// True when no registered pipeline writes depth. The pass then attaches
    /// depth read-only, which permits sampling it from within the same pass.
    pub depth_read_only: bool,
    pub sample_count: u32,
}

impl PassConfig {
    pub fn has_depth(&self) -> bool {
        self.depth_format.is_some()
    }
    pub fn is_multisampled(&self) -> bool {
        self.sample_count > 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_takes_the_strongest_requirement() {
        let a = PassRequirements::NONE;
        let b = PassRequirements::DEPTH.with_sample_count(4);
        let m = a.merge(b).unwrap();
        assert_eq!(m.depth, DepthUse::Write);
        assert!(m.isolate_depth);
        assert_eq!(m.sample_count, 4);
    }

    #[test]
    fn read_and_write_merge_to_write() {
        let r = PassRequirements::NONE.with_depth(DepthUse::Read);
        let w = PassRequirements::NONE.with_depth(DepthUse::Write);
        assert_eq!(r.merge(w).unwrap().depth, DepthUse::Write);
    }

    #[test]
    fn merge_is_order_independent() {
        let a = PassRequirements::NONE
            .with_depth(DepthUse::Read)
            .with_stencil();
        let b = PassRequirements::DEPTH.with_sample_count(4);
        assert_eq!(a.merge(b).unwrap(), b.merge(a).unwrap());
    }

    #[test]
    fn conflicting_depth_formats_are_reported() {
        let a = PassRequirements {
            depth_format: Some(wgpu::TextureFormat::Depth32Float),
            ..PassRequirements::DEPTH
        };
        let b = PassRequirements {
            depth_format: Some(wgpu::TextureFormat::Depth16Unorm),
            ..PassRequirements::DEPTH
        };
        assert!(a.merge(b).is_err());
    }

    /// The rebuild trigger: adding a pipeline whose needs are already covered
    /// must not invalidate everything else.
    #[test]
    fn covers_is_the_rebuild_predicate() {
        let merged = PassRequirements::DEPTH.with_sample_count(4);
        assert!(merged.covers(PassRequirements::NONE));
        assert!(merged.covers(PassRequirements::NONE.with_depth(DepthUse::Read)));
        assert!(!merged.covers(PassRequirements::NONE.with_stencil()));
        assert!(!merged.covers(PassRequirements::NONE.with_sample_count(8)));
    }

    #[test]
    fn depth_is_read_only_until_something_writes() {
        let read = PassRequirements::NONE.with_depth(DepthUse::Read);
        assert!(read.resolve_stub().depth_read_only);
        assert!(!PassRequirements::DEPTH.resolve_stub().depth_read_only);
    }

    // `resolve` needs a Gpu; this mirrors its format/read-only logic for the
    // parts that do not touch the adapter.
    impl PassRequirements {
        fn resolve_stub(self) -> PassConfig {
            PassConfig {
                color_format: wgpu::TextureFormat::Bgra8UnormSrgb,
                depth_format: match (self.depth, self.stencil) {
                    (DepthUse::None, false) => None,
                    (_, true) => Some(wgpu::TextureFormat::Depth24PlusStencil8),
                    (_, false) => Some(wgpu::TextureFormat::Depth32Float),
                },
                depth_read_only: self.depth != DepthUse::Write,
                sample_count: 1,
            }
        }
    }
}
