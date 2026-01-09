const SLOT_BITS : u32 = 12u;
const SLOT_MASK : u32 = (1u << SLOT_BITS) - 1u;

const GEN_BITS : u32 = 32u - SLOT_BITS;
const GEN_MASK : u32 = (1u << GEN_BITS) - 1u;

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
    @location(2) slot_plus_one: u32,
    @location(3) gen: u32,
};

struct Globals {
    window_size: vec2<f32>,
    mouse_pos: vec2<f32>,
    mouse_buttons: u32,
    time: f32,
    delta_time: f32,
    frame: u32,
};

var<push_constant> globals: Globals;

@group(0) @binding(0) var tex_arr: binding_array<texture_2d<f32>>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<storage, read> gens: array<u32>;

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
    out.color = unpack4x8unorm(in.style.x);
    out.uv_tex = uv_tex;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.slot_plus_one == 0u {
        return in.color;
    }

    let idx = in.slot_plus_one - 1u;
    if (gens[idx] & GEN_MASK) != in.gen {
        return vec4<f32>(0.0, 0.0, 1.0, 0.0);
    }

    let c = textureSample(tex_arr[idx], samp, in.uv_tex);
    return c * in.color;
}
