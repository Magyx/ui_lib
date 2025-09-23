struct VertexInput {
    // instance buffer
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
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
    time: f32,
    delta_time: f32,
    mouse_pos: vec2<f32>,
    mouse_buttons: u32,
    frame: u32,
};

var<push_constant> globals: Globals;

@group(0) @binding(0) var tex_arr: binding_array<texture_2d<f32>>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<storage, read> gens: array<u32>;

fn unpack_unorm2x16(p: u32) -> vec2<f32> {
    let x = f32(p & 0xFFFFu) / 65535.0;
    let y = f32(p >> 16u) / 65535.0;
    return vec2<f32>(x, y);
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

    let scale = unpack_unorm2x16(in.tex.z);
    let offs = unpack_unorm2x16(in.tex.w);
    let uv_tex = uv * scale + offs;

    var out: VertexOutput;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.color = in.color;
    out.uv_tex = uv_tex;
    out.slot_plus_one = in.tex.x;
    out.gen = in.tex.y;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.slot_plus_one == 0u {
        return in.color;
    }

    let idx = in.slot_plus_one - 1u;
    if gens[idx] != in.gen {
        return vec4<f32>(0.0, 0.0, 1.0, 0.0);
    }

    let c = textureSample(tex_arr[idx], samp, in.uv_tex);
    return c * in.color;
}
