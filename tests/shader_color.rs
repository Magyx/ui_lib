//! Headless pixel tests for the UI shader's colour handling.
//!
//! These render a single quad into an offscreen texture and read the bytes
//! back, so they check what actually lands in the framebuffer rather than what
//! the CPU-side code intended. Three things are under test:
//!
//! 1. Whether `Color`'s bytes are decoded from sRGB before the shader treats
//!    them as linear light.
//! 2. Whether the fragment shader emits premultiplied alpha, which is what the
//!    `One / OneMinusSrcAlpha` blend state expects.
//! 3. Whether `UiPipeline` still composites correctly when some *other*
//!    pipeline has pulled depth into the shared pass. `UiPipeline` declares
//!    `PassRequirements::NONE`, but it must still match whatever the pass
//!    turns out to be — that is what `PipelineCtx::depth_state_passthrough`
//!    is for, and getting it wrong is either a wgpu validation panic or
//!    silently dropped fragments.
//!
//! Needs a real device, obtained through [`Gpu::headless`], which shares its
//! feature and limit derivation with `Engine::new` so the two cannot drift.
//!
//! Backends live behind this crate's own features and `wgpu` is built with
//! `default-features = false`, so **`cargo test` alone compiles no backend and
//! these silently skip**. Run `cargo test --features vulkan` (or
//! metal/dx12/gl). `Gpu::headless` prints the reason to stderr whenever it
//! returns `None`, so check for a `ui:` line if these seem suspiciously fast.

use ui::gpu::{Globals, Gpu};
use ui::model::{Color, Position, Size};
use ui::primitive::{Instance, InstanceStore};
use ui::render::pipeline::{
    DepthUse, DrawCtx, PassConfig, PassRequirements, Pipeline, PipelineCtx, ui::UiPipeline,
};
use ui::render::texture::TextureRegistry;
use ui::wgpu;

const W: u32 = 4;
const H: u32 = 4;
/// wgpu requires buffer copy rows to be 256-byte aligned.
const ROW_BYTES: u32 = 256;

// ── harness ─────────────────────────────────────────────────────────────

fn globals() -> Globals {
    Globals {
        window_size: [W as f32, H as f32],
        mouse_pos: [0.0, 0.0],
        mouse_buttons: 0,
        time: 0.0,
        delta_time: 0.0,
        frame: 0,
        scale: 1.0,
        _pad: 0.0,
    }
}

/// Render `store` over `clear` into a `W x H` texture of `format` and return
/// the raw bytes of the top-left texel.
///
/// The bytes are whatever the format stores — for an `*Srgb` format that is
/// the *encoded* value, which is exactly what we want to inspect.
fn render_texel(
    gpu: &Gpu,
    format: wgpu::TextureFormat,
    clear: wgpu::Color,
    store: &InstanceStore,
) -> [u8; 4] {
    render_texel_in_pass(gpu, format, clear, store, PassRequirements::NONE)
}

/// As [`render_texel`], but builds the pass from `requirements` so a test can
/// put `UiPipeline` into a pass it did not ask for.
///
/// The pass is assembled exactly the way `Renderer::render` assembles it:
/// resolve the requirements against the device, build the pipeline from the
/// resulting [`PassConfig`], then create attachments from the same config. A
/// mismatch anywhere in that chain is a wgpu validation error, so this covers
/// the negotiation as well as the colour maths.
fn render_texel_in_pass(
    gpu: &Gpu,
    format: wgpu::TextureFormat,
    clear: wgpu::Color,
    store: &InstanceStore,
    requirements: PassRequirements,
) -> [u8; 4] {
    let textures = TextureRegistry::new(&gpu.device);
    let immediate_size = std::mem::size_of::<Globals>() as u32;

    let pass_config: PassConfig = requirements.resolve(gpu, format);
    assert_eq!(
        pass_config.sample_count, 1,
        "this harness reads back a single-sampled texture; \
         add a resolve target before asking for MSAA"
    );

    let pipeline_ctx = PipelineCtx {
        gpu,
        pass: pass_config,
        texture_bgl: textures.layout(),
        immediate_size,
    };
    let mut pipeline = UiPipeline::new(&pipeline_ctx);

    let extent = wgpu::Extent3d {
        width: W,
        height: H,
        depth_or_array_layers: 1,
    };

    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("readback target"),
        size: extent,
        mip_level_count: 1,
        sample_count: pass_config.sample_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

    // Only created when the resolved config asks for it, mirroring
    // `Attachments::ensure`.
    let depth_view = pass_config.depth_format.map(|depth_format| {
        let depth = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("readback depth"),
            size: extent,
            mip_level_count: 1,
            sample_count: pass_config.sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: depth_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        depth.create_view(&wgpu::TextureViewDescriptor::default())
    });

    let instances = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("instances"),
        size: store.bytes().len().max(4) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    gpu.queue.write_buffer(&instances, 0, store.bytes());

    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (ROW_BYTES * H) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let g = globals();
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("colour test pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: depth_view.as_ref().map(|view| {
                wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let draw_ctx = DrawCtx {
            globals: &g,
            textures: textures.bind_group(),
            instances: &instances,
            store,
            depth: None,
            pass: pass_config,
            target_size: [W, H],
        };

        pipeline.bind(&draw_ctx, &mut pass);
        for batch in store.batches() {
            pipeline.draw(&draw_ctx, &mut pass, batch);
        }
    }

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ROW_BYTES),
                rows_per_image: Some(H),
            },
        },
        extent,
    );

    gpu.queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(60)),
        })
        .unwrap();

    let data = slice.get_mapped_range().unwrap();
    let texel = [data[0], data[1], data[2], data[3]];
    drop(data);
    readback.unmap();
    texel
}

/// A quad covering the whole target.
fn full_quad(color: Color) -> InstanceStore {
    let mut store = InstanceStore::new();
    store.push(Instance::ui(
        Position::new(0.0, 0.0),
        Size::new(W as f32, H as f32),
        color,
    ));
    store
}

/// Panics rather than skipping. A skipped test reports as a pass and its
/// stderr is hidden, so a silent skip looks exactly like a green run.
fn gpu() -> Gpu {
    Gpu::headless().expect(
        "no GPU device available — run with `cargo test --features vulkan` \
         (or metal/dx12/gl); see the `ui:` line above for the specific reason",
    )
}

// ── the tests ───────────────────────────────────────────────────────────

/// A mid grey should come back as the same mid grey.
///
/// `0x80` is an sRGB-*encoded* value — it is what a design tool means by 50%
/// grey. Correct handling decodes it to linear (~0.216) in the shader; the
/// `*Srgb` target then re-encodes on write and the stored byte round-trips to
/// `0x80`.
///
/// Skipping the decode makes the shader emit `0.502` as if it were linear, and
/// the write encodes it a second time, storing ~`0xBC`.
#[test]
fn srgb_color_round_trips_through_an_srgb_target() {
    let gpu = gpu();

    let store = full_quad(Color::rgb(0x80, 0x80, 0x80));
    let texel = render_texel(
        &gpu,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::Color::BLACK,
        &store,
    );

    assert!(
        texel[0].abs_diff(0x80) <= 1,
        "expected 0x80 to round-trip, got {:#04X}. \
         ~0xBC means Color's bytes are being treated as linear and the sRGB \
         target is encoding them a second time — decode in the vertex shader.",
        texel[0],
    );
}

/// A half-transparent red over black should come out half-bright.
///
/// The blend state is `src_factor: One, dst_factor: OneMinusSrcAlpha`, which is
/// the *premultiplied* form: it expects the shader to have already multiplied
/// rgb by alpha. Over a black destination the result is then `0.5`.
///
/// If the shader emits straight alpha, `One` passes the full `1.0` through and
/// the result is `0xFF` — the fill over-brightens whatever is behind it.
///
/// Uses a non-sRGB target so no transfer function is involved and the readback
/// byte is the blend result directly.
#[test]
fn half_alpha_fill_blends_at_half_intensity() {
    let gpu = gpu();

    let store = full_quad(Color::rgba(0xFF, 0x00, 0x00, 0x80));
    let texel = render_texel(
        &gpu,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::Color::BLACK,
        &store,
    );

    assert!(
        texel[0].abs_diff(0x80) <= 2,
        "expected ~0x80 over black, got {:#04X}. \
         0xFF means the shader emits straight alpha while the blend state \
         expects premultiplied — either multiply rgb by alpha in fs_main or \
         switch src_factor to SrcAlpha.",
        texel[0],
    );
}

/// Opaque colours must be unaffected by whichever premultiply fix is chosen.
///
/// Guards against "fixing" the blend by scaling rgb somewhere that also touches
/// the alpha == 1.0 path.
#[test]
fn opaque_fill_is_unaffected_by_alpha_handling() {
    let gpu = gpu();

    let store = full_quad(Color::rgba(0xFF, 0x00, 0x00, 0xFF));
    let texel = render_texel(
        &gpu,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::Color::BLACK,
        &store,
    );

    assert_eq!(texel[0], 0xFF, "opaque red should be full intensity");
    assert_eq!(texel[1], 0x00);
    assert_eq!(texel[2], 0x00);
}

/// `UiPipeline` must render identically in a pass that has depth.
///
/// It never asks for depth, but a single 3D pipeline anywhere in the app pulls
/// depth into the shared pass and every other pipeline has to match. A
/// `depth_stencil: None` hard-coded in a pipeline is a validation error here,
/// which is why `UiPipeline` uses `ctx.depth_state_passthrough()`.
#[test]
fn ui_pipeline_renders_the_same_in_a_depth_enabled_pass() {
    let gpu = gpu();

    let store = full_quad(Color::rgba(0xFF, 0x00, 0x00, 0xFF));
    let flat = render_texel(
        &gpu,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::Color::BLACK,
        &store,
    );
    let with_depth = render_texel_in_pass(
        &gpu,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::Color::BLACK,
        &store,
        PassRequirements::DEPTH,
    );

    assert_eq!(
        flat, with_depth,
        "attaching depth changed the UI's output; \
         UiPipeline must neither test nor write depth"
    );
}

/// Painter's order must survive depth being attached.
///
/// Two coincident quads: the second one wins because the UI composites in
/// submission order. If `UiPipeline` ever declares `depth_write_enabled: true`
/// or a comparison other than `Always`, the second quad is rejected at equal
/// depth and the first one shows through — a whole-UI z-fighting bug that no
/// other test would catch.
#[test]
fn later_quads_still_paint_over_earlier_ones_with_depth_attached() {
    let gpu = gpu();

    let mut store = InstanceStore::new();
    for color in [
        Color::rgba(0xFF, 0x00, 0x00, 0xFF),
        Color::rgba(0x00, 0x00, 0xFF, 0xFF),
    ] {
        store.push(Instance::ui(
            Position::new(0.0, 0.0),
            Size::new(W as f32, H as f32),
            color,
        ));
    }

    let texel = render_texel_in_pass(
        &gpu,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::Color::BLACK,
        &store,
        PassRequirements::DEPTH,
    );

    assert_eq!(
        [texel[0], texel[1], texel[2]],
        [0x00, 0x00, 0xFF],
        "the second quad should win; red means it was depth-rejected at equal z"
    );
}

/// The pass config the UI is built against must be the trivial one.
///
/// Cheap guard on the negotiation itself: if `UiPipeline::requirements` ever
/// grows a depth or sample-count demand, every app pays for an attachment it
/// does not use.
#[test]
fn ui_pipeline_demands_nothing_from_the_pass() {
    assert_eq!(UiPipeline::requirements(), PassRequirements::NONE);
    assert_eq!(PassRequirements::NONE.depth, DepthUse::None);
}
