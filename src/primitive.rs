use std::mem;

use crate::{
    model::{Color, Position, Rect, Size},
    render::{
        pack::{pack_shape_params, pack_unorm4x8, shape_flags},
        pipeline::{Pipeline, PipelineId, ui::UiPipeline},
        texture::TextureHandle,
    },
};

/// Per-instance data a pipeline consumes.
///
/// The [`Pod`](bytemuck::Pod) bound is what makes a type safe to hand to the
/// GPU as raw bytes; `layout` tells wgpu how to read them back. Implement it
/// for any `#[repr(C)]` struct a custom pipeline wants to draw with.
pub trait InstanceData: bytemuck::Pod + Send + Sync + 'static {
    fn layout() -> wgpu::VertexBufferLayout<'static>;
}

/// Declares that `Self` reads `D` as its per-instance data.
pub trait Instanced<D: InstanceData>: Pipeline {}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Primitive {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub data1: [u32; 4],
    pub data2: [u32; 4],
}

impl Primitive {
    pub fn new(position: Position<f32>, size: Size<f32>, data1: [u32; 4], data2: [u32; 4]) -> Self {
        Self {
            position: [position.x, position.y],
            size: [size.width, size.height],
            data1,
            data2,
        }
    }
}

impl InstanceData for Primitive {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
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
                    format: wgpu::VertexFormat::Uint32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint32x4,
                },
            ],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PrimitiveStyle {
    pub fill: Color,
    pub corner_radius: f32,
    pub border_width: f32,
    pub border_color: Color,
    pub shadow_radius: f32,
    pub shadow_color: Color,
    pub gradient_end: Option<Color>,
}

impl PrimitiveStyle {
    /// Minimal style: just a fill color with no rounding, border, or effects.
    pub fn flat(fill: Color) -> Self {
        Self {
            fill,
            corner_radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            shadow_radius: 0.0,
            shadow_color: Color::TRANSPARENT,
            gradient_end: None,
        }
    }

    /// Rounded rectangle with no border.
    pub fn rounded(fill: Color, corner_radius: f32) -> Self {
        Self {
            corner_radius,
            ..Self::flat(fill)
        }
    }

    pub fn with_border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = color;
        self
    }

    pub fn with_shadow(mut self, radius: f32, color: Color) -> Self {
        self.shadow_radius = radius;
        self.shadow_color = color;
        self
    }

    pub fn with_gradient_end(mut self, color: Color) -> Self {
        self.gradient_end = Some(color);
        self
    }

    #[inline]
    fn pack(&self) -> (u32, u32, u32) {
        let mut flags: u8 = 0;
        if self.border_width > 0.0 && self.border_color.a() > 0 {
            flags |= shape_flags::HAS_BORDER;
        }
        if self.shadow_radius > 0.0 && self.shadow_color.a() > 0 {
            flags |= shape_flags::HAS_SHADOW;
        }

        // aux_color: shadow_color if shadow is active, gradient_end if gradient,
        // otherwise transparent. Shadow and gradient share the aux slot;
        // if both are set, shadow takes priority (gradient_end is less critical).
        let aux_color;
        if let Some(end) = self.gradient_end {
            flags |= shape_flags::GRADIENT_V;
            aux_color = end;
        } else if flags & shape_flags::HAS_SHADOW != 0 {
            aux_color = self.shadow_color;
        } else {
            aux_color = Color::TRANSPARENT;
        }

        let shape = pack_shape_params(
            self.corner_radius,
            self.border_width,
            self.shadow_radius,
            flags,
        );
        (shape, self.border_color.0, aux_color.0)
    }
}

pub struct Instance<D: InstanceData = Primitive> {
    pub(crate) data: D,
    pub(crate) kind: PipelineId,
}
impl<D: InstanceData> Instance<D> {
    /// Emit `data` through `P`.
    pub fn of<P: Instanced<D>>(data: D) -> Self {
        Self {
            data,
            kind: PipelineId::of::<P>(),
        }
    }
}
impl Instance<Primitive> {
    /// Emit a [`Primitive`] through `P`.
    pub fn new<P: Instanced<Primitive>>(
        position: Position<f32>,
        size: Size<f32>,
        data1: [u32; 4],
        data2: [u32; 4],
    ) -> Self {
        Self::of::<P>(Primitive {
            position: [position.x, position.y],
            size: [size.width, size.height],
            data1,
            data2,
        })
    }

    pub fn ui(position: Position<f32>, size: Size<f32>, color: Color) -> Self {
        Self {
            data: Primitive {
                position: [position.x, position.y],
                size: [size.width, size.height],
                data1: [color.0, 0, 0, 0],
                data2: [0, 0, 0, 0],
            },
            kind: PipelineId::of::<UiPipeline>(),
        }
    }

    pub fn ui_tex(
        position: Position<f32>,
        size: Size<f32>,
        tint: Color,
        handle: TextureHandle,
    ) -> Self {
        Self {
            data: Primitive {
                position: [position.x, position.y],
                size: [size.width, size.height],
                data1: [tint.0, 0, 0, 0],
                data2: [
                    handle.slot_gen,
                    handle.scale_packed,
                    handle.offset_packed,
                    0,
                ],
            },
            kind: PipelineId::of::<UiPipeline>(),
        }
    }

    pub fn ui_tex_fit(
        position: Position<f32>,
        size: Size<f32>,
        tint: Color,
        handle: TextureHandle,
        content_scale: [f32; 2],
        content_offset: [f32; 2],
    ) -> Self {
        let content_fit = pack_unorm4x8([
            content_scale[0],
            content_scale[1],
            content_offset[0],
            content_offset[1],
        ]);
        Self {
            data: Primitive {
                position: [position.x, position.y],
                size: [size.width, size.height],
                data1: [tint.0, 0, 0, 0],
                data2: [
                    handle.slot_gen,
                    handle.scale_packed,
                    handle.offset_packed,
                    content_fit,
                ],
            },
            kind: PipelineId::of::<UiPipeline>(),
        }
    }

    pub fn ui_rounded(
        position: Position<f32>,
        size: Size<f32>,
        fill: Color,
        corner_radius: f32,
        border_width: i32,
        border_color: Color,
    ) -> Self {
        let style = PrimitiveStyle::flat(fill).with_border(border_width as f32, border_color);
        let style = PrimitiveStyle {
            corner_radius,
            ..style
        };
        Self::ui_styled(position, size, style)
    }

    pub fn ui_styled(position: Position<f32>, size: Size<f32>, style: PrimitiveStyle) -> Self {
        let (shape_params, border_packed, aux_packed) = style.pack();
        Self {
            data: Primitive {
                position: [position.x, position.y],
                size: [size.width, size.height],
                data1: [style.fill.0, shape_params, border_packed, aux_packed],
                data2: [0, 0, 0, 0],
            },
            kind: PipelineId::of::<UiPipeline>(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayerShift {
    /// Same layer as the parent. The default, and what every widget that
    /// doesn't care about z-order returns.
    #[default]
    Inherit,
    /// The parent's effective layer plus `n`.
    Above(u16),
    /// A reserved band above everything, for tooltips that must beat a modal
    /// they can't see. `TOP_BASE + n`.
    Top(u16),
}
impl LayerShift {
    pub const TOP_BASE: u16 = u16::MAX / 2;

    #[inline]
    pub fn resolve(self, parent: u16) -> u16 {
        match self {
            LayerShift::Inherit => parent,
            LayerShift::Above(n) => parent.saturating_add(n),
            LayerShift::Top(n) => Self::TOP_BASE.saturating_add(n),
        }
    }
}

/// One run of instances sharing a pipeline and a clip.
pub struct Batch {
    pub id: PipelineId,
    pub byte_offset: u64,
    pub count: u32,
    pub clip: Option<Rect>,
    pub layer: u16,
}

/// Instances emitted during one paint pass, in draw order.
pub struct InstanceStore {
    data: Vec<u32>,
    batches: Vec<Batch>,
    count: usize,
    clip: Option<Rect>,
    layer: u16,
}
impl Default for InstanceStore {
    fn default() -> Self {
        Self::new()
    }
}
impl InstanceStore {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            batches: Vec::new(),
            count: 0,
            clip: None,
            layer: 0,
        }
    }

    /// Total instances pushed this frame, across every pipeline.
    pub fn len(&self) -> usize {
        self.count
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn batches(&self) -> &[Batch] {
        &self.batches
    }

    pub fn view<D: InstanceData>(&self, batch: &Batch) -> &[D] {
        let start = batch.byte_offset as usize;
        let end = start + std::mem::size_of::<D>() * batch.count as usize;
        bytemuck::cast_slice(&self.bytes()[start..end])
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.data)
    }

    pub(crate) fn clear(&mut self) {
        self.data.clear();
        self.batches.clear();
        self.clip = None;
        self.count = 0;
        self.layer = 0;
    }
    pub(crate) fn set_clip(&mut self, clip: Option<Rect>) -> Option<Rect> {
        mem::replace(&mut self.clip, clip)
    }

    pub(crate) fn set_layer(&mut self, layer: u16) -> u16 {
        mem::replace(&mut self.layer, layer)
    }

    #[inline]
    pub fn layer(&self) -> u16 {
        self.layer
    }

    pub fn push_raised<D: InstanceData>(&mut self, i: Instance<D>, n: u16) {
        let prev = self.layer;
        self.layer = prev.saturating_add(n);
        self.push(i);
        self.layer = prev;
    }

    pub(crate) fn draw_order(&self, out: &mut Vec<usize>) {
        out.clear();
        out.extend(0..self.batches.len());
        if self.batches.iter().any(|b| b.layer != 0) {
            out.sort_by_key(|&i| self.batches[i].layer);
        }
    }

    pub fn push<D: InstanceData>(&mut self, i: Instance<D>) {
        debug_assert_eq!(
            std::mem::size_of::<D>() % 4,
            0,
            "instance stride must be a multiple of 4"
        );
        debug_assert!(
            std::mem::align_of::<D>() >= 4,
            "instance data must be at least 4-byte aligned"
        );

        let byte_offset = (self.data.len() * 4) as u64;
        self.data
            .extend_from_slice(bytemuck::cast_slice(std::slice::from_ref(&i.data)));
        self.count += 1;

        match self.batches.last_mut() {
            Some(b) if b.id == i.kind && b.clip == self.clip && b.layer == self.layer => {
                debug_assert_eq!(
                    byte_offset,
                    b.byte_offset + b.count as u64 * std::mem::size_of::<D>() as u64,
                    "instances in one batch must be contiguous and equally sized; \
                     does this pipeline implement Instanced for more than one type?"
                );
                b.count += 1;
            }
            _ => self.batches.push(Batch {
                id: i.kind,
                byte_offset,
                count: 1,
                clip: self.clip,
                layer: self.layer,
            }),
        }
    }
}

#[cfg(test)]
mod layer_tests {
    use super::*;

    #[test]
    fn inherit_is_the_default() {
        assert_eq!(LayerShift::default(), LayerShift::Inherit);
        assert_eq!(LayerShift::Inherit.resolve(7), 7);
    }

    #[test]
    fn above_is_relative_to_the_parent() {
        assert_eq!(LayerShift::Above(10).resolve(0), 10);
        assert_eq!(LayerShift::Above(10).resolve(100), 110);
    }

    /// The propagation case: a popup inside a modal must land *above* the
    /// modal's own content, which is what additive shifts buy over absolute
    /// ones — an absolute `10` inside a `100` modal would sit underneath it.
    #[test]
    fn nested_shifts_compose_upward() {
        let modal = LayerShift::Above(1000).resolve(0);
        let content = LayerShift::Inherit.resolve(modal);
        let dropdown = LayerShift::Above(10).resolve(content);
        assert_eq!((modal, content, dropdown), (1000, 1000, 1010));
        assert!(dropdown > content);
    }

    #[test]
    fn top_escapes_any_ancestor() {
        let deep = LayerShift::Above(500).resolve(LayerShift::Above(1000).resolve(0));
        assert!(LayerShift::Top(0).resolve(deep) > deep);
    }

    #[test]
    fn shifts_saturate_rather_than_wrap() {
        let high = LayerShift::Above(u16::MAX).resolve(0);
        assert_eq!(LayerShift::Above(u16::MAX).resolve(high), u16::MAX);
        assert_eq!(LayerShift::Top(u16::MAX).resolve(0), u16::MAX);
    }
}

#[cfg(test)]
mod layer_store_tests {
    use super::*;
    use crate::model::{Color, Position, Size};

    fn ui(x: f32) -> Instance<Primitive> {
        Instance::ui(
            Position::new(x, 0.0),
            Size::new(1.0, 1.0),
            Color::rgba(255, 0, 0, 255),
        )
    }

    /// Instances at the same layer coalesce; a layer change breaks the batch
    /// the same way a clip change does.
    #[test]
    fn layer_change_breaks_the_batch() {
        let mut s = InstanceStore::new();
        s.push(ui(0.0));
        s.push(ui(1.0));
        assert_eq!(s.batches().len(), 1);
        s.set_layer(1);
        s.push(ui(2.0));
        assert_eq!(s.batches().len(), 2);
        assert_eq!(s.batches()[0].layer, 0);
        assert_eq!(s.batches()[1].layer, 1);
    }

    /// The point of `push_raised` over a paired raise/restore: the store's
    /// layer is exactly as it was, so nothing leaks onto later siblings.
    #[test]
    fn push_raised_restores_the_layer() {
        let mut s = InstanceStore::new();
        s.set_layer(100);
        s.push_raised(ui(0.0), 1);
        assert_eq!(s.layer(), 100);
        s.push(ui(1.0));
        assert_eq!(s.batches().last().unwrap().layer, 100);
    }

    #[test]
    fn push_raised_is_relative_to_the_walk() {
        let mut s = InstanceStore::new();
        s.set_layer(1000);
        s.push_raised(ui(0.0), 1);
        assert_eq!(
            s.batches()[0].layer,
            1001,
            "must compose with an ancestor's shift"
        );
    }

    /// Repeated raised pushes coalesce rather than fragmenting into one batch
    /// each, because they all resolve to the same layer.
    #[test]
    fn consecutive_raised_pushes_share_a_batch() {
        let mut s = InstanceStore::new();
        s.push_raised(ui(0.0), 1);
        s.push_raised(ui(1.0), 1);
        s.push_raised(ui(2.0), 1);
        assert_eq!(s.batches().len(), 1);
        assert_eq!(s.batches()[0].count, 3);
    }

    #[test]
    fn push_raised_saturates() {
        let mut s = InstanceStore::new();
        s.set_layer(u16::MAX);
        s.push_raised(ui(0.0), 5);
        assert_eq!(s.batches()[0].layer, u16::MAX);
        assert_eq!(s.layer(), u16::MAX);
    }

    #[test]
    fn draw_order_puts_higher_layers_last() {
        let mut s = InstanceStore::new();
        s.set_layer(5);
        s.push(ui(0.0)); // painted first, drawn last
        s.set_layer(0);
        s.push(ui(1.0));

        let mut order = Vec::new();
        s.draw_order(&mut order);
        let layers: Vec<u16> = order.iter().map(|&i| s.batches()[i].layer).collect();
        assert_eq!(layers, vec![0, 5]);
    }

    /// Ordering must not disturb the batch list itself. Paint order is a
    /// record worth keeping: batches tile `data` contiguously, and the last
    /// batch is the last thing painted.
    #[test]
    fn draw_order_leaves_the_batch_list_alone() {
        let mut s = InstanceStore::new();
        s.set_layer(5);
        s.push(ui(0.0));
        s.set_layer(0);
        s.push(ui(1.0));

        let before: Vec<(u16, u64)> = s
            .batches()
            .iter()
            .map(|b| (b.layer, b.byte_offset))
            .collect();
        let mut order = Vec::new();
        s.draw_order(&mut order);
        let after: Vec<(u16, u64)> = s
            .batches()
            .iter()
            .map(|b| (b.layer, b.byte_offset))
            .collect();
        assert_eq!(before, after);
    }

    /// Batches must keep tiling the byte buffer with no gap or overlap — the
    /// invariant `tests/paint.rs` asserts against a real paint walk.
    #[test]
    fn batches_stay_contiguous_regardless_of_layer() {
        let mut s = InstanceStore::new();
        for x in 0..6 {
            s.set_layer((x % 3) as u16);
            s.push(ui(x as f32));
        }
        let stride = std::mem::size_of::<Primitive>() as u64;
        let mut next = 0u64;
        let mut total = 0usize;
        for b in s.batches() {
            assert_eq!(b.byte_offset, next, "gap or overlap between batches");
            assert!(b.count > 0, "empty batch");
            next += b.count as u64 * stride;
            total += b.count as usize;
        }
        assert_eq!(total, s.len());
    }

    /// Equal layers keep the order they were painted in, so a modal's scrim
    /// still draws under its own children.
    #[test]
    fn draw_order_is_stable_within_a_layer() {
        let mut s = InstanceStore::new();
        for x in 0..3 {
            s.set_layer(0);
            s.push(ui(x as f32));
            s.set_layer(1);
            s.push(ui(x as f32));
        }
        let mut order = Vec::new();
        s.draw_order(&mut order);

        for want in [0u16, 1] {
            let offs: Vec<u64> = order
                .iter()
                .map(|&i| &s.batches()[i])
                .filter(|b| b.layer == want)
                .map(|b| b.byte_offset)
                .collect();
            assert!(
                offs.windows(2).all(|w| w[0] < w[1]),
                "layer {want} lost paint order"
            );
        }
    }

    /// Every batch is drawn exactly once, whatever the layers.
    #[test]
    fn draw_order_is_a_permutation() {
        let mut s = InstanceStore::new();
        for x in 0..6 {
            s.set_layer((x % 3) as u16);
            s.push(ui(x as f32));
        }
        let mut order = Vec::new();
        s.draw_order(&mut order);
        let mut seen = order.clone();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), s.batches().len());
        assert_eq!(order.len(), s.batches().len());
    }

    #[test]
    fn draw_order_is_identity_when_everything_is_layer_zero() {
        let mut s = InstanceStore::new();
        for x in 0..4 {
            s.set_clip(Some(Rect::new(x, 0, 1, 1)));
            s.push(ui(x as f32));
        }
        let mut order = Vec::new();
        s.draw_order(&mut order);
        assert_eq!(order, (0..s.batches().len()).collect::<Vec<_>>());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Color, Position, Size};
    use crate::render::pack::unpack_shape_params;
    use crate::render::pipeline::impl_stub_pipeline;

    #[repr(C)]
    #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
    struct CustomInstance {
        center: [f32; 2],
        radius: f32,
        _pad: f32,
    }

    impl InstanceData for CustomInstance {
        fn layout() -> wgpu::VertexBufferLayout<'static> {
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<CustomInstance>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                }],
            }
        }
    }

    #[derive(Pipeline)]
    struct CustomPipeline;
    impl_stub_pipeline!(CustomPipeline);
    impl Instanced<CustomInstance> for CustomPipeline {}

    #[test]
    fn bytes_length_follows_the_instance_type() {
        let mut store = InstanceStore::new();
        store.push(Instance::ui(
            Position::new(0.0, 0.0),
            Size::new(1.0, 1.0),
            Color::WHITE,
        ));
        assert_eq!(store.bytes().len(), std::mem::size_of::<Primitive>());

        let mut custom = InstanceStore::new();
        custom.push(Instance::of::<CustomPipeline>(CustomInstance {
            center: [0.0, 0.0],
            radius: 1.0,
            _pad: 0.0,
        }));
        assert_eq!(custom.bytes().len(), std::mem::size_of::<CustomInstance>());
    }

    #[test]
    fn bytes_length_is_count_times_stride() {
        let mut store = InstanceStore::new();
        for _ in 0..3 {
            store.push(Instance::ui(
                Position::new(0.0, 0.0),
                Size::new(1.0, 1.0),
                Color::RED,
            ));
        }
        assert_eq!(store.len(), 3);
        assert_eq!(store.bytes().len(), 3 * std::mem::size_of::<Primitive>());
    }

    /// Two pipelines with different instance sizes in one buffer: the whole
    /// point of the byte store.
    #[test]
    fn mixed_instance_formats_share_one_buffer() {
        let mut store = InstanceStore::new();
        store.push(Instance::ui(
            Position::new(1.0, 2.0),
            Size::new(3.0, 4.0),
            Color::RED,
        ));
        store.push(Instance::of::<CustomPipeline>(CustomInstance {
            center: [7.0, 8.0],
            radius: 9.0,
            _pad: 0.0,
        }));

        let b = store.batches();
        assert_eq!(b.len(), 2, "different pipelines must not merge");
        assert_eq!(b[0].byte_offset, 0);
        assert_eq!(
            b[1].byte_offset as usize,
            std::mem::size_of::<Primitive>(),
            "the second batch starts where the first ended, in bytes"
        );
        assert_eq!(
            store.bytes().len(),
            std::mem::size_of::<Primitive>() + std::mem::size_of::<CustomInstance>()
        );

        // Each batch reads back at its own stride.
        assert_eq!(store.view::<Primitive>(&b[0])[0].position, [1.0, 2.0]);
        assert_eq!(store.view::<CustomInstance>(&b[1])[0].center, [7.0, 8.0]);
        assert_eq!(store.view::<CustomInstance>(&b[1])[0].radius, 9.0);
    }

    #[test]
    fn ui_instance_packs_color_into_data1_low_word() {
        let color = Color::rgba(0x11, 0x22, 0x33, 0x44);
        let inst = Instance::ui(Position::new(1.0, 2.0), Size::new(10.0, 20.0), color);
        assert_eq!(inst.data.data1[0], color.0);
        assert_eq!(inst.data.data1[1], 0);
        assert_eq!(inst.data.data2, [0, 0, 0, 0]);
    }

    #[test]
    fn ui_instance_preserves_position_and_size() {
        let inst = Instance::ui(Position::new(3.0, 7.0), Size::new(50.0, 60.0), Color::WHITE);
        assert_eq!(inst.data.position, [3.0, 7.0]);
        assert_eq!(inst.data.size, [50.0, 60.0]);
    }

    #[test]
    fn primitive_stores_f32_coords() {
        let inst = Instance::ui(
            Position::new(-7.0, -8.0),
            Size::new(0.0, 0.0),
            Color::TRANSPARENT,
        );
        assert_eq!(inst.data.position, [-7.0, -8.0]);
        assert_eq!(inst.data.size, [0.0, 0.0]);
    }

    #[test]
    fn subpixel_positions_preserved() {
        let inst = Instance::ui(Position::new(10.3, 20.7), Size::new(5.5, 8.0), Color::WHITE);
        assert_eq!(inst.data.position, [10.3, 20.7]);
        assert_eq!(inst.data.size, [5.5, 8.0]);
    }

    #[test]
    fn ui_flat_has_zero_shape_params() {
        let inst = Instance::ui(Position::new(0.0, 0.0), Size::new(10.0, 10.0), Color::RED);
        assert_eq!(
            inst.data.data1[1], 0,
            "flat ui must have shape_params == 0 for fast path"
        );
        assert_eq!(inst.data.data1[2], 0);
        assert_eq!(inst.data.data1[3], 0);
    }

    #[test]
    fn ui_rounded_packs_shape_params() {
        let inst = Instance::ui_rounded(
            Position::new(0.0, 0.0),
            Size::new(100.0, 40.0),
            Color::BLUE,
            8.0,
            1,
            Color::WHITE,
        );

        // data1[0] = fill color
        assert_eq!(inst.data.data1[0], Color::BLUE.0);

        // data1[1] = shape_params (non-zero because corner_radius=8 and border_width=1)
        let (r, b, s, flags) = unpack_shape_params(inst.data.data1[1]);
        assert_eq!(r, 8.0);
        assert_eq!(b, 1.0);
        assert_eq!(s, 0.0);
        assert_eq!(flags & shape_flags::HAS_BORDER, shape_flags::HAS_BORDER);
        assert_eq!(flags & shape_flags::HAS_SHADOW, 0);

        // data1[2] = border color
        assert_eq!(inst.data.data1[2], Color::WHITE.0);

        // data2 = no texture
        assert_eq!(inst.data.data2, [0, 0, 0, 0]);
    }

    #[test]
    fn ui_rounded_no_border_omits_border_flag() {
        let inst = Instance::ui_rounded(
            Position::new(0.0, 0.0),
            Size::new(50.0, 50.0),
            Color::RED,
            12.0,
            0,
            Color::TRANSPARENT,
        );

        let (r, b, _, flags) = unpack_shape_params(inst.data.data1[1]);
        assert_eq!(r, 12.0);
        assert_eq!(b, 0.0);
        assert_eq!(flags & shape_flags::HAS_BORDER, 0);
    }

    #[test]
    fn ui_styled_shadow() {
        let style = PrimitiveStyle::flat(Color::WHITE).with_shadow(16.0, Color::rgba(0, 0, 0, 128));
        let inst = Instance::ui_styled(Position::new(10.0, 20.0), Size::new(100.0, 60.0), style);

        let (_, _, s, flags) = unpack_shape_params(inst.data.data1[1]);
        assert_eq!(s, 16.0);
        assert_eq!(flags & shape_flags::HAS_SHADOW, shape_flags::HAS_SHADOW);

        // aux_color = shadow_color
        assert_eq!(inst.data.data1[3], Color::rgba(0, 0, 0, 128).0);
    }

    #[test]
    fn ui_styled_gradient() {
        let style = PrimitiveStyle::rounded(Color::RED, 6.0).with_gradient_end(Color::BLUE);
        let inst = Instance::ui_styled(Position::new(0.0, 0.0), Size::new(200.0, 100.0), style);

        let (_, _, _, flags) = unpack_shape_params(inst.data.data1[1]);
        assert_eq!(flags & shape_flags::GRADIENT_V, shape_flags::GRADIENT_V);
        assert_eq!(inst.data.data1[3], Color::BLUE.0);
    }

    #[test]
    fn primitive_style_flat_produces_zero_shape_params() {
        let style = PrimitiveStyle::flat(Color::GREEN);
        let (shape, border, aux) = style.pack();
        assert_eq!(shape, 0, "flat style must pack to 0 for shader fast path");
        assert_eq!(border, Color::TRANSPARENT.0);
        assert_eq!(aux, Color::TRANSPARENT.0);
    }

    #[test]
    fn ui_styled_preserves_position_and_size() {
        let style = PrimitiveStyle::rounded(Color::WHITE, 10.0).with_border(2.0, Color::BLACK);
        let inst = Instance::ui_styled(Position::new(5.5, 7.5), Size::new(120.0, 80.0), style);
        assert_eq!(inst.data.position, [5.5, 7.5]);
        assert_eq!(inst.data.size, [120.0, 80.0]);
    }
}

#[cfg(test)]
mod batching {
    use super::*;
    use crate::render::pipeline::{Pipeline, impl_stub_pipeline};

    #[derive(Pipeline)]
    struct AltPipeline;
    impl_stub_pipeline!(AltPipeline);

    fn ui_id() -> PipelineId {
        PipelineId::of::<UiPipeline>()
    }
    fn alt_id() -> PipelineId {
        PipelineId::of::<AltPipeline>()
    }

    /// A 1x1 white quad through the UI pipeline. Contents are irrelevant to
    /// batching; only `kind` is.
    fn ui_quad() -> Instance<Primitive> {
        Instance::ui(Position::new(0.0, 0.0), Size::new(1.0, 1.0), Color::WHITE)
    }

    fn alt_quad() -> Instance<Primitive> {
        Instance::new::<AltPipeline>(Position::new(0.0, 0.0), Size::new(1.0, 1.0), [0; 4], [0; 4])
    }

    /// Batches tile the byte buffer with no gaps or overlaps, none is empty,
    /// and their counts sum to the instance count. The renderer slices the
    /// buffer at these offsets, so a violation draws the wrong bytes.
    ///
    /// Every batch here is `Primitive`-sized; mixed strides are covered by
    /// `mixed_instance_formats_share_one_buffer`.
    fn assert_covers_all(store: &InstanceStore) {
        let stride = std::mem::size_of::<Primitive>() as u64;
        let mut next = 0u64;
        let mut total = 0usize;
        for b in store.batches() {
            assert_eq!(
                b.byte_offset, next,
                "batch does not start where the previous ended"
            );
            assert!(b.count > 0, "empty batch");
            next += b.count as u64 * stride;
            total += b.count as usize;
        }
        assert_eq!(
            next as usize,
            store.bytes().len(),
            "batches do not cover every byte"
        );
        assert_eq!(total, store.len(), "batches do not cover every instance");
    }

    #[test]
    fn empty_store_has_no_batches() {
        let store = InstanceStore::new();
        assert!(store.batches().is_empty());
        assert_covers_all(&store);
    }

    #[test]
    fn single_push_opens_one_batch() {
        let mut store = InstanceStore::new();
        store.push(ui_quad());

        assert_eq!(store.batches().len(), 1);
        assert_eq!(store.batches()[0].id, ui_id());
        assert_eq!(store.batches()[0].byte_offset, 0);
        assert_eq!(store.batches()[0].count, 1);
        assert_eq!(store.batches()[0].clip, None);
        assert_covers_all(&store);
    }

    #[test]
    fn same_pipeline_and_clip_merge_into_one_batch() {
        let mut store = InstanceStore::new();
        for _ in 0..5 {
            store.push(ui_quad());
        }

        assert_eq!(
            store.batches().len(),
            1,
            "run of 5 should compress to 1 batch"
        );
        assert_eq!(store.batches()[0].count, 5);
        assert_eq!(store.len(), 5, "compression must not drop instance data");
        assert_covers_all(&store);
    }

    #[test]
    fn pipeline_change_splits_the_batch() {
        let mut store = InstanceStore::new();
        store.push(ui_quad());
        store.push(ui_quad());
        store.push(alt_quad());
        store.push(ui_quad());

        let b = store.batches();
        assert_eq!(b.len(), 3);
        let stride = std::mem::size_of::<Primitive>() as u64;
        assert_eq!((b[0].id, b[0].byte_offset, b[0].count), (ui_id(), 0, 2));
        assert_eq!(
            (b[1].id, b[1].byte_offset, b[1].count),
            (alt_id(), 2 * stride, 1)
        );
        assert_eq!(
            (b[2].id, b[2].byte_offset, b[2].count),
            (ui_id(), 3 * stride, 1)
        );
        assert_covers_all(&store);
    }

    #[test]
    fn clip_change_splits_the_batch() {
        let mut store = InstanceStore::new();
        store.push(ui_quad());
        store.set_clip(Some(Rect::new(10, 20, 30, 40)));
        store.push(ui_quad());

        let b = store.batches();
        assert_eq!(b.len(), 2, "same pipeline but a different clip must split");
        assert_eq!(b[0].clip, None);
        assert_eq!(b[1].clip, Some(Rect::new(10, 20, 30, 40)));
        assert_covers_all(&store);
    }

    #[test]
    fn setting_the_same_clip_does_not_split() {
        let mut store = InstanceStore::new();
        let clip = Some(Rect::new(1, 2, 3, 4));
        store.set_clip(clip);
        store.push(ui_quad());
        store.set_clip(clip);
        store.push(ui_quad());

        assert_eq!(
            store.batches().len(),
            1,
            "redundant set_clip must not fragment"
        );
        assert_eq!(store.batches()[0].count, 2);
    }

    /// Merging only ever considers the *open* batch. Returning to an earlier
    /// clip starts a fresh batch rather than reopening the old one — draw
    /// order forbids reordering instances into it.
    #[test]
    fn returning_to_an_earlier_clip_starts_a_new_batch() {
        let mut store = InstanceStore::new();
        let outer = Some(Rect::new(0, 0, 100, 100));
        let inner = Some(Rect::new(10, 10, 20, 20));

        store.set_clip(outer);
        store.push(ui_quad());
        store.set_clip(inner);
        store.push(ui_quad());
        store.set_clip(outer);
        store.push(ui_quad());

        let b = store.batches();
        assert_eq!(b.len(), 3);
        assert_eq!(b[0].clip, outer);
        assert_eq!(b[1].clip, inner);
        assert_eq!(b[2].clip, outer);
        assert_covers_all(&store);
    }

    /// `layout.rs` restores the parent clip from this return value, so the
    /// `mem::replace` contract is load-bearing for nested clipping.
    #[test]
    fn set_clip_returns_the_previous_clip() {
        let mut store = InstanceStore::new();
        assert_eq!(store.set_clip(Some(Rect::new(1, 1, 1, 1))), None);
        assert_eq!(
            store.set_clip(Some(Rect::new(2, 2, 2, 2))),
            Some(Rect::new(1, 1, 1, 1))
        );
        assert_eq!(store.set_clip(None), Some(Rect::new(2, 2, 2, 2)));
    }

    /// Culling degenerate clips is the renderer's job. The store records the
    /// batch faithfully so the renderer can decide.
    #[test]
    fn zero_area_clip_still_records_a_batch() {
        let mut store = InstanceStore::new();
        store.set_clip(Some(Rect::new(5, 5, 0, 40)));
        store.push(ui_quad());

        assert_eq!(store.batches().len(), 1);
        assert_eq!(store.batches()[0].clip, Some(Rect::new(5, 5, 0, 40)));
    }

    #[test]
    fn clear_resets_data_batches_and_clip() {
        let mut store = InstanceStore::new();
        store.set_clip(Some(Rect::new(1, 2, 3, 4)));
        store.push(ui_quad());
        store.clear();

        assert_eq!(store.len(), 0);
        assert!(store.batches().is_empty());
        assert!(store.bytes().is_empty());

        // A stale clip would silently apply to the next frame.
        store.push(ui_quad());
        assert_eq!(
            store.batches()[0].clip,
            None,
            "clear must reset the active clip, not just the batches"
        );
    }

    #[test]
    fn alternating_pipelines_do_not_compress() {
        let mut store = InstanceStore::new();
        for i in 0..6 {
            if i % 2 == 0 {
                store.push(ui_quad());
            } else {
                store.push(alt_quad());
            }
        }

        assert_eq!(
            store.batches().len(),
            6,
            "worst case for RLE: one batch per instance"
        );
        assert_covers_all(&store);
    }
}
