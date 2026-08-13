const SLOT_BITS: u32 = 12u;
const SLOT_MASK: u32 = (1u << SLOT_BITS) - 1u;

const GEN_BITS: u32 = 32u - SLOT_BITS;
const GEN_MASK: u32 = (1u << GEN_BITS) - 1u;

// Flag bits in shape_params byte 3
const FLAG_HAS_BORDER: u32 = 1u;
const FLAG_HAS_SHADOW: u32 = 2u;
const FLAG_GRADIENT_V: u32 = 4u;

struct VertexInput {
    // instance buffer
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) style: vec4<u32>,
    @location(3) tex: vec4<u32>,

    // vertex buffer
    @location(10) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv_tex: vec2<f32>,
    @location(2) @interpolate(flat) slot_plus_one: u32,
    @location(3) @interpolate(flat) gen: u32,
    @location(4) local_uv: vec2<f32>,
    @location(5) rect_size_px: vec2<f32>,
    @location(6) @interpolate(flat) shape_params: u32,
    @location(7) border_color: vec4<f32>,
    @location(8) aux_color: vec4<f32>,
};

struct Globals {
    window_size: vec2<f32>,
    mouse_pos: vec2<f32>,
    mouse_buttons: u32,
    time: f32,
    delta_time: f32,
    frame: u32,
    scale: f32,
};

var<push_constant> globals: Globals;

@group(0) @binding(0) var tex_arr: binding_array<texture_2d<f32>>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<storage, read> gens: array<u32>;

// Colors arrive as sRGB-encoded bytes.
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + vec3(0.055)) / 1.055, vec3(2.4));
    return select(hi, lo, c <= vec3(0.04045));
}

fn unpack_color(packed: u32) -> vec4<f32> {
    let c = unpack4x8unorm(packed);
    return vec4(srgb_to_linear(c.rgb), c.a);
}

// The blend state is `One / OneMinusSrcAlpha`, i.e. premultiplied. Every
// fragment return must be premultiplied to match.
//
// Order matters: decode sRGB first, premultiply second. Premultiplying gamma
// values and decoding afterwards is a different (wrong) result, and is a bug
// egui shipped for a while.
fn premultiply(c: vec4<f32>) -> vec4<f32> {
    return vec4(c.rgb * c.a, c.a);
}

fn sd_rounded_box(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let r = min(radius, min(half_size.x, half_size.y));
    let q = abs(p) - half_size + vec2(r);
    return length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);

    let local_pos = uv * in.size;
    let world_pos = in.position + local_pos;
    let ndc = vec2<f32>(
        (world_pos.x / globals.window_size.x) * 2.0 - 1.0,
        1.0 - (world_pos.y / globals.window_size.y) * 2.0
    );

    let packed = in.tex.x;
    let slot_plus_one = (packed & SLOT_MASK);
    let gen = (packed >> SLOT_BITS);

    let scale = unpack2x16unorm(in.tex.y);
    let offs = unpack2x16unorm(in.tex.z);
    let cf = unpack4x8unorm(in.tex.w);
    let uv_loc = uv * cf.xy + cf.zw;

    let uv_tex = uv * scale + offs;

    var out: VertexOutput;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.slot_plus_one = slot_plus_one;
    out.gen = gen;
    out.color = unpack_color(in.style.x);
    out.uv_tex = uv_tex;

    // SDF varyings
    out.local_uv = uv;
    out.rect_size_px = in.size * globals.scale;
    out.shape_params = in.style.y;
    out.border_color = unpack_color(in.style.z);
    out.aux_color = unpack_color(in.style.w);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Fast path: no shape params — original flat/textured behavior, zero overhead.
    if in.shape_params == 0u {
        if in.slot_plus_one == 0u {
            return premultiply(in.color);
        }

        let idx = in.slot_plus_one - 1u;
        if (gens[idx] & GEN_MASK) != in.gen {
            return vec4<f32>(0.0);
        }

        // `tex_arr` is an *Srgb format, so the sample is already linear, and
        // its alpha is straight -- matching how the SDF path treats it below.
        let c = textureSample(tex_arr[idx], samp, in.uv_tex);
        return premultiply(c * in.color);
    }

    // Unpack shape parameters
    let corner_radius_raw = f32(in.shape_params & 0xFFu);
    let border_width_raw = f32((in.shape_params >> 8u) & 0xFFu) * 0.25;
    let shadow_radius_raw = f32((in.shape_params >> 16u) & 0xFFu);
    let flags = (in.shape_params >> 24u) & 0xFFu;

    // Scale to physical pixels for SDF evaluation
    let scale = globals.scale;
    let corner_radius = corner_radius_raw * scale;
    let border_width = border_width_raw * scale;
    let shadow_radius = shadow_radius_raw * scale;

    let has_border = (flags & FLAG_HAS_BORDER) != 0u;
    let has_shadow = (flags & FLAG_HAS_SHADOW) != 0u;
    let has_gradient = (flags & FLAG_GRADIENT_V) != 0u;

    // Convert local UV [0,1] to pixel coordinates centered at rect center
    let rect_size = in.rect_size_px;
    let half_size = rect_size * 0.5;
    let p = in.local_uv * rect_size - half_size;

    // Outer SDF distance
    let d = sd_rounded_box(p, half_size, corner_radius);

    // Determine fill color
    var fill: vec4<f32>;
    if in.slot_plus_one == 0u {
        fill = in.color;
    } else {
        let idx = in.slot_plus_one - 1u;
        if (gens[idx] & GEN_MASK) != in.gen {
            return vec4<f32>(0.0);
        }
        let tex_color = textureSample(tex_arr[idx], samp, in.uv_tex);
        fill = tex_color * in.color;
    }

    // Vertical gradient
    if has_gradient {
        fill = mix(fill, in.aux_color, in.local_uv.y);
    }

    // Anti-aliased shape mask: 1 inside, 0 outside, smooth at edge
    let shape_alpha = 1.0 - smoothstep(-0.5, 0.5, d);

    var result: vec4<f32>;

    if has_border && border_width > 0.0 {
        // Inner SDF for the fill region (inset by border width)
        let inner_radius = max(corner_radius - border_width, 0.0);
        let inner_half = half_size - vec2(border_width);
        let d_inner = sd_rounded_box(p, max(inner_half, vec2(0.0)), inner_radius);

        let inner_alpha = 1.0 - smoothstep(-0.5, 0.5, d_inner);
        let border_alpha = shape_alpha - inner_alpha;

        let fill_pm = premultiply(fill) * inner_alpha;
        let border_pm = premultiply(in.border_color) * border_alpha;

        result = fill_pm + border_pm;
    } else {
        result = premultiply(fill) * shape_alpha;
    }

    // Shadow (outer glow)
    if has_shadow && shadow_radius > 0.0 {
        let shadow_alpha_raw = 1.0 - smoothstep(0.0, shadow_radius, d);
        let shadow_alpha = shadow_alpha_raw * (1.0 - shape_alpha);

        let shadow_pm = premultiply(in.aux_color) * shadow_alpha;

        // Shadow behind shape
        result = result + shadow_pm * (1.0 - result.a);
    }

    return result;
}
