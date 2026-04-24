use crate::{consts::DEFAULT_MAX_TEXTURES, graphics::Gpu, model::Size};

#[inline]
pub fn pack_unorm2x16(xy: [f32; 2]) -> u32 {
    let q = |v: f32| -> u32 { (v.clamp(0.0, 1.0) * 65535.0 + 0.5).floor() as u32 };
    q(xy[0]) | (q(xy[1]) << 16)
}

#[inline]
pub fn pack_unorm4x8(xyzw: [f32; 4]) -> u32 {
    let q = |v: f32| -> u32 { (v.clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u32 };
    q(xyzw[0]) | (q(xyzw[1]) << 8) | (q(xyzw[2]) << 16) | (q(xyzw[3]) << 24)
}

// up to 4096 textures (12 bits), 20-bit generations
pub const SLOT_BITS: u32 = 12;
pub const SLOT_MASK: u32 = (1 << SLOT_BITS) - 1;
#[inline]
pub fn pack_slot_gen(slot: usize, generation: u32) -> u32 {
    (generation << SLOT_BITS) | ((slot + 1) as u32 & SLOT_MASK)
}
#[inline]
pub fn unpack_slot_gen(packed: u32) -> (usize, u32) {
    let slot_plus_one = packed & SLOT_MASK;
    let generation = packed >> SLOT_BITS;
    (slot_plus_one as usize - 1, generation)
}

fn dummy_bind_group(device: &wgpu::Device) -> wgpu::BindGroup {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("dummy"),
        entries: &[],
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("dummy"),
        layout: &layout,
        entries: &[],
    })
}

pub struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub struct Atlas {
    slot_index: usize,
    generation: u32,
    size_px: Size<u32>,
    cursor_x: u32,
    cursor_y: u32,
    row_h: u32,
}

impl Atlas {
    fn new(slot_index: usize, generation: u32, size_px: Size<u32>) -> Self {
        Self {
            slot_index,
            generation,
            size_px,
            cursor_x: 0,
            cursor_y: 0,
            row_h: 0,
        }
    }

    // TODO: alloc using LRU
    fn alloc(&mut self, w: u32, h: u32) -> Option<AtlasRect> {
        if w > self.size_px.width || h > self.size_px.height {
            return None;
        }
        if self.cursor_x + w > self.size_px.width {
            self.cursor_x = 0;
            self.cursor_y += self.row_h;
            self.row_h = 0;
        }
        if self.cursor_y + h > self.size_px.height {
            return None;
        }

        let rect = AtlasRect {
            x: self.cursor_x,
            y: self.cursor_y,
            w,
            h,
        };
        self.cursor_x += w;
        if h > self.row_h {
            self.row_h = h;
        }
        Some(rect)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct TextureHandle {
    pub slot_gen: u32,
    pub scale_packed: u32,
    pub offset_packed: u32,
    pub size_px: Size<u32>,
}

#[derive(Clone)]
struct TexSlot {
    tex: wgpu::Texture,
    view: wgpu::TextureView,
}

pub struct TextureRegistry {
    layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,

    views: Vec<Option<TexSlot>>,
    gens: Vec<u32>,
    gens_buffer: wgpu::Buffer,

    free: Vec<usize>,
    placeholder_view: wgpu::TextureView,
}

impl TextureRegistry {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("UI Texture Array BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: std::num::NonZeroU32::new(DEFAULT_MAX_TEXTURES),
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("UI Texture Sampler"),
            ..Default::default()
        });

        let placeholder = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("UI Placeholder Tex"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let placeholder_view = placeholder.create_view(&Default::default());

        let n = DEFAULT_MAX_TEXTURES as usize;
        let views = vec![None; n];
        let gens = vec![0u32; n];

        let gens_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI Texture Generations Buffer"),
            size: (std::mem::size_of::<u32>() * n) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut reg = Self {
            layout,
            bind_group: dummy_bind_group(device),
            sampler,

            views,
            gens,
            gens_buffer,
            free: (0..n).rev().collect(),
            placeholder_view,
        };
        reg.update_bind_group(device);
        reg
    }

    fn update_bind_group(&mut self, device: &wgpu::Device) {
        let mut slice: Vec<&wgpu::TextureView> = Vec::with_capacity(self.views.len());
        for v in &self.views {
            slice.push(
                v.as_ref()
                    .map(|s| &s.view)
                    .unwrap_or(&self.placeholder_view),
            );
        }

        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UI Texture Array BG"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(&slice),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.gens_buffer.as_entire_binding(),
                },
            ],
        });
    }

    pub fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn load_rgba8(
        &mut self,
        gpu: &Gpu,
        width: u32,
        height: u32,
        pixels_rgba8: &[u8],
    ) -> TextureHandle {
        let idx = self
            .free
            .pop()
            .expect("Texture slots exhausted; bump DEFAULT_MAX_TEXTURES");

        let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("UI Image"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        gpu.queue.write_texture(
            tex.as_image_copy(),
            pixels_rgba8,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let view = tex.create_view(&Default::default());

        self.views[idx] = Some(TexSlot { tex, view });

        gpu.queue.write_buffer(
            &self.gens_buffer,
            (std::mem::size_of::<u32>() * idx) as u64,
            bytemuck::cast_slice(&[self.gens[idx]]),
        );
        self.update_bind_group(&gpu.device);

        TextureHandle {
            slot_gen: pack_slot_gen(idx, self.gens[idx]),
            scale_packed: pack_unorm2x16([1.0, 1.0]),
            offset_packed: pack_unorm2x16([0.0, 0.0]),
            size_px: Size::new(width, height),
        }
    }

    pub fn update_rgba8(&mut self, gpu: &Gpu, handle: TextureHandle, pixels_rgba8: &[u8]) -> bool {
        let (idx, generation) = unpack_slot_gen(handle.slot_gen);
        if idx >= self.views.len() {
            return false;
        }
        if self.gens[idx] != generation {
            return false;
        }
        let Some(slot) = &self.views[idx] else {
            return false;
        };

        let w = handle.size_px.width;
        let h = handle.size_px.height;

        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &slot.tex,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            pixels_rgba8,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        true
    }

    pub fn unload(&mut self, gpu: &Gpu, handle: TextureHandle) -> bool {
        let (idx, generation) = unpack_slot_gen(handle.slot_gen);
        if idx >= self.views.len() {
            return false;
        }
        if self.gens[idx] != generation {
            return false;
        }

        self.views[idx] = None;
        self.gens[idx] = self.gens[idx].wrapping_add(1);
        self.free.push(idx);

        gpu.queue.write_buffer(
            &self.gens_buffer,
            (std::mem::size_of::<u32>() * idx) as u64,
            bytemuck::cast_slice(&[self.gens[idx]]),
        );
        self.update_bind_group(&gpu.device);
        true
    }

    pub fn create_atlas(&mut self, gpu: &Gpu, width: u32, height: u32) -> Atlas {
        let idx = self
            .free
            .pop()
            .expect("Texture slots exhausted; bump DEFAULT_MAX_TEXTURES");
        let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("UI Atlas"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        self.views[idx] = Some(TexSlot { tex, view });

        gpu.queue.write_buffer(
            &self.gens_buffer,
            (std::mem::size_of::<u32>() * idx) as u64,
            bytemuck::cast_slice(&[self.gens[idx]]),
        );
        self.update_bind_group(&gpu.device);

        Atlas::new(idx, self.gens[idx], Size::new(width, height))
    }

    pub fn load_into_atlas(
        &mut self,
        gpu: &Gpu,
        atlas: &mut Atlas,
        w: u32,
        h: u32,
        pixels_rgba8: &[u8],
    ) -> Option<TextureHandle> {
        let rect = atlas.alloc(w, h)?;
        let slot = self.views[atlas.slot_index]
            .as_ref()
            .expect("atlas slot missing");

        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &slot.tex,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: rect.x,
                    y: rect.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            pixels_rgba8,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        let scale = [
            w as f32 / atlas.size_px.width as f32,
            h as f32 / atlas.size_px.height as f32,
        ];
        let offs = [
            rect.x as f32 / atlas.size_px.width as f32,
            rect.y as f32 / atlas.size_px.height as f32,
        ];

        Some(TextureHandle {
            slot_gen: pack_slot_gen(atlas.slot_index, atlas.generation),
            scale_packed: pack_unorm2x16(scale),
            offset_packed: pack_unorm2x16(offs),
            size_px: Size::new(w, h),
        })
    }

    pub fn destroy_atlas(&mut self, gpu: &Gpu, atlas: &mut Atlas) {
        let idx = atlas.slot_index;

        self.gens[idx] = self.gens[idx].wrapping_add(1);
        gpu.queue.write_buffer(
            &self.gens_buffer,
            (std::mem::size_of::<u32>() * idx) as u64,
            bytemuck::cast_slice(&[self.gens[idx]]),
        );

        self.views[idx] = None;
        self.update_bind_group(&gpu.device);
        self.free.push(idx);

        atlas.size_px = Size::new(0, 0);
        atlas.cursor_x = 0;
        atlas.cursor_y = 0;
        atlas.row_h = 0;
        atlas.generation = self.gens[idx];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================
    // pack_unorm2x16 / pack_unorm4x8
    // ========================================================
    //
    // These are the GPU-format contracts used to pack atlas UV
    // offsets/scales and content-fit ratios into u32. A rounding bug
    // here silently corrupts UVs on screen — worth careful coverage.

    #[test]
    fn pack_unorm2x16_corners() {
        assert_eq!(pack_unorm2x16([0.0, 0.0]), 0);
        // 1.0 * 65535 + 0.5 -> 65535.5, floor -> 65535 = 0xFFFF.
        assert_eq!(pack_unorm2x16([1.0, 1.0]), 0xFFFF_FFFF);
        // x only: low 16 bits set.
        assert_eq!(pack_unorm2x16([1.0, 0.0]), 0x0000_FFFF);
        // y only: high 16 bits set.
        assert_eq!(pack_unorm2x16([0.0, 1.0]), 0xFFFF_0000);
    }

    #[test]
    fn pack_unorm2x16_halfway() {
        // 0.5 * 65535 + 0.5 = 32768.0, floor = 32768 = 0x8000.
        let p = pack_unorm2x16([0.5, 0.5]);
        assert_eq!(p & 0xFFFF, 0x8000);
        assert_eq!(p >> 16, 0x8000);
    }

    #[test]
    fn pack_unorm2x16_clamps_out_of_range_inputs() {
        // Negative values clamp to 0.
        assert_eq!(pack_unorm2x16([-1.0, -100.0]), 0);
        // Values above 1.0 clamp to 1.0 -> 0xFFFF each.
        assert_eq!(pack_unorm2x16([2.0, 1_000.0]), 0xFFFF_FFFF);
    }

    #[test]
    fn pack_unorm4x8_corners() {
        assert_eq!(pack_unorm4x8([0.0, 0.0, 0.0, 0.0]), 0);
        // 1.0 * 255 + 0.5 = 255.5, floor = 255.
        assert_eq!(pack_unorm4x8([1.0, 1.0, 1.0, 1.0]), 0xFFFF_FFFF);
    }

    #[test]
    fn pack_unorm4x8_each_channel_independent() {
        // Only low byte.
        assert_eq!(pack_unorm4x8([1.0, 0.0, 0.0, 0.0]), 0x0000_00FF);
        // Only second byte.
        assert_eq!(pack_unorm4x8([0.0, 1.0, 0.0, 0.0]), 0x0000_FF00);
        // Only third byte.
        assert_eq!(pack_unorm4x8([0.0, 0.0, 1.0, 0.0]), 0x00FF_0000);
        // Only high byte.
        assert_eq!(pack_unorm4x8([0.0, 0.0, 0.0, 1.0]), 0xFF00_0000);
    }

    #[test]
    fn pack_unorm4x8_clamps_out_of_range_inputs() {
        assert_eq!(pack_unorm4x8([-0.5, -1.0, -10.0, -100.0]), 0);
        assert_eq!(pack_unorm4x8([2.0, 3.0, 100.0, f32::INFINITY]), 0xFFFF_FFFF);
    }

    // ========================================================
    // pack_slot_gen / unpack_slot_gen
    // ========================================================
    //
    // The encoding stores (slot + 1) in the low SLOT_BITS and the
    // generation in the upper bits. Note the +1/-1 offset: slot 0 is
    // encoded as 1 in the low bits, which lets a fully-zero packed
    // value mean "no texture".

    #[test]
    fn slot_gen_roundtrip_slot_zero() {
        let packed = pack_slot_gen(0, 0);
        assert_eq!(
            packed, 1,
            "slot 0 + gen 0 should encode as 1 (zero means unset)"
        );
        let (slot, r#gen) = unpack_slot_gen(packed);
        assert_eq!((slot, r#gen), (0, 0));
    }

    #[test]
    fn slot_gen_roundtrip_named_slots() {
        for slot in [0, 1, 7, 255, 1000, SLOT_MASK as usize - 1] {
            for r#gen in [0u32, 1, 42, 1_000_000] {
                let packed = pack_slot_gen(slot, r#gen);
                let (s, g) = unpack_slot_gen(packed);
                assert_eq!(
                    (s, g),
                    (slot, r#gen),
                    "roundtrip failed for slot={slot} gen={gen}"
                );
            }
        }
    }

    #[test]
    fn slot_gen_layout_is_low_slot_bits_for_slot() {
        // Generation bits should not leak into the slot portion.
        let packed = pack_slot_gen(5, 0xABCD);
        assert_eq!(packed & SLOT_MASK, 5 + 1);
        assert_eq!(packed >> SLOT_BITS, 0xABCD);
    }

    #[test]
    fn slot_gen_max_slot_fits_in_slot_bits() {
        // SLOT_MASK - 1 is the largest slot index whose (slot + 1)
        // still fits in SLOT_BITS (because (SLOT_MASK - 1) + 1 == SLOT_MASK).
        let max_slot = (SLOT_MASK as usize) - 1;
        let packed = pack_slot_gen(max_slot, 0);
        let (s, _) = unpack_slot_gen(packed);
        assert_eq!(s, max_slot);
    }

    // ========================================================
    // Atlas::alloc - shelf packing state machine
    // ========================================================

    fn fresh_atlas(w: u32, h: u32) -> Atlas {
        // Atlas::new is private; we're in the same module, so this works
        // from inside `mod tests` only.
        Atlas::new(0, 0, Size::new(w, h))
    }

    #[test]
    fn alloc_single_fits_at_origin() {
        let mut a = fresh_atlas(256, 256);
        let r = a.alloc(32, 16).expect("should fit");
        assert_eq!((r.x, r.y, r.w, r.h), (0, 0, 32, 16));
    }

    #[test]
    fn alloc_second_fits_next_to_first_on_same_row() {
        let mut a = fresh_atlas(256, 256);
        let r1 = a.alloc(32, 16).unwrap();
        let r2 = a.alloc(24, 16).unwrap();
        assert_eq!((r1.x, r1.y), (0, 0));
        assert_eq!(
            (r2.x, r2.y),
            (32, 0),
            "second rect starts where first ended on x"
        );
    }

    #[test]
    fn alloc_row_h_tracks_tallest_on_current_row() {
        // Pack: [10x5][10x20][10x5] — row height should be 20 by the end.
        let mut a = fresh_atlas(100, 100);
        let _ = a.alloc(10, 5).unwrap();
        let _ = a.alloc(10, 20).unwrap(); // bumps row_h to 20
        let _ = a.alloc(10, 5).unwrap();

        // Now force a wrap. The next rect should land at y = 20 (the
        // tallest row so far), not y = 5.
        // Fill the rest of row 1 (width 100, we used 30, so 70 left).
        let _ = a.alloc(70, 1).unwrap();
        // Next alloc of any size triggers the wrap.
        let wrap = a.alloc(5, 5).unwrap();
        assert_eq!(
            wrap.y, 20,
            "new row should start below the tallest on prev row"
        );
        assert_eq!(wrap.x, 0, "new row starts at x=0");
    }

    #[test]
    fn alloc_wraps_to_new_row_when_current_row_full() {
        // 10-wide atlas, first row holds one 6x4 rect. Next 6x4 needs to wrap.
        let mut a = fresh_atlas(10, 20);
        let _ = a.alloc(6, 4).unwrap();
        let r = a.alloc(6, 4).unwrap();
        assert_eq!(r.x, 0);
        assert_eq!(r.y, 4, "second row starts at row_h of first");
    }

    #[test]
    fn alloc_rejects_rect_larger_than_atlas() {
        let mut a = fresh_atlas(32, 32);
        assert!(a.alloc(33, 10).is_none(), "too wide");
        assert!(a.alloc(10, 33).is_none(), "too tall");
        assert!(a.alloc(100, 100).is_none(), "both");
    }

    #[test]
    fn alloc_rejects_when_vertical_space_exhausted() {
        // A tiny atlas that fits exactly one row of 5x5.
        let mut a = fresh_atlas(5, 5);
        let _ = a.alloc(5, 5).unwrap();
        // Next request needs to wrap to y=5 but the atlas ends at y=5.
        assert!(a.alloc(1, 1).is_none());
    }

    #[test]
    fn alloc_exactly_fits_atlas_width_on_single_row() {
        let mut a = fresh_atlas(100, 50);
        let r = a.alloc(100, 50).expect("exact-fit should succeed");
        assert_eq!((r.x, r.y, r.w, r.h), (0, 0, 100, 50));
        // Next alloc has nowhere to go.
        assert!(a.alloc(1, 1).is_none());
    }

    #[test]
    fn alloc_zero_sized_rect_succeeds_and_does_not_advance_cursor() {
        // `w > self.size_px.width` is the only rejection, so zero is allowed.
        // Zero-width also means the x cursor doesn't move.
        let mut a = fresh_atlas(100, 100);
        let r = a.alloc(0, 0).expect("zero-sized alloc should succeed");
        assert_eq!((r.w, r.h), (0, 0));

        // A subsequent real alloc should still start at x=0 because
        // cursor_x only advances by w, and w was 0.
        let r2 = a.alloc(10, 10).unwrap();
        assert_eq!((r2.x, r2.y), (0, 0));
    }

    #[test]
    fn alloc_many_small_rects_fills_multiple_rows() {
        // 40x40 atlas filled with 10x10 rects should pack 4x4 = 16 rects.
        let mut a = fresh_atlas(40, 40);
        let mut allocated = 0;
        while a.alloc(10, 10).is_some() {
            allocated += 1;
        }
        assert_eq!(allocated, 16, "a 40x40 atlas should fit 16 10x10 tiles");
        // One more attempt must still fail.
        assert!(a.alloc(10, 10).is_none());
    }

    #[test]
    fn alloc_mixed_heights_waste_vertical_space() {
        // Shelf packers waste space below short rects in a tall row.
        // Verify this is the behaviour (regression guard in case we
        // later switch to a smarter packer).
        let mut a = fresh_atlas(100, 100);
        let _ = a.alloc(100, 80).unwrap(); // row_h becomes 80
        // Force wrap.
        let r = a.alloc(1, 1).unwrap();
        assert_eq!(r.y, 80, "gap below short rects is still 80 tall");
    }

    // ========================================================
    // TextureHandle
    // ========================================================

    #[test]
    fn texture_handle_default_is_all_zero() {
        // slot_gen == 0 is the sentinel for "no texture" — relied on by
        // TextSystem::upload_glyph which returns TextureHandle::default()
        // for oversized or zero-size glyphs.
        let h = TextureHandle::default();
        assert_eq!(h.slot_gen, 0);
        assert_eq!(h.scale_packed, 0);
        assert_eq!(h.offset_packed, 0);
        assert_eq!(h.size_px, Size::new(0, 0));
    }

    #[test]
    fn texture_handle_copy_and_eq() {
        let h1 = TextureHandle {
            slot_gen: pack_slot_gen(3, 7),
            scale_packed: pack_unorm2x16([1.0, 1.0]),
            offset_packed: pack_unorm2x16([0.25, 0.5]),
            size_px: Size::new(32, 32),
        };
        let h2 = h1; // Copy
        assert_eq!(h1, h2);
        assert_ne!(h1, TextureHandle::default());
    }
}
