const SLOT_BITS: u32 = 12u;
const SLOT_MASK: u32 = (1u << SLOT_BITS) - 1u;
const GEN_BITS: u32 = 32u - SLOT_BITS;
const GEN_MASK: u32 = (1u << GEN_BITS) - 1u;

struct VertexInput {
    // instance buffer — mvp columns
    @location(0) mvp0: vec4<f32>,
    @location(1) mvp1: vec4<f32>,
    @location(2) mvp2: vec4<f32>,
    @location(3) mvp3: vec4<f32>,
    // instance buffer — normal matrix columns (xyz used)
    @location(4) nrm0: vec4<f32>,
    @location(5) nrm1: vec4<f32>,
    @location(6) nrm2: vec4<f32>,
    // instance buffer — placement and material
    @location(7) rect: vec4<f32>,    // x, y, w, h in logical px
    @location(8) tint: vec4<f32>,    // linear RGBA
    @location(9) params: vec4<u32>,  // mesh slot, tex slot_gen, uv scale, uv offset

    // mesh vertex buffer
    @location(10) position: vec3<f32>,
    @location(11) normal: vec3<f32>,
    @location(12) uv: vec2<f32>,
    @location(13) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv_tex: vec2<f32>,
    @location(3) @interpolate(flat) slot_plus_one: u32,
    @location(4) @interpolate(flat) gen: u32,
};

struct Globals {
    window_size: vec2<f32>,
    mouse_pos: vec2<f32>,
    mouse_buttons: u32,
    time: f32,
    delta_time: f32,
    frame: u32,
    scale: f32,
    _pad: f32,
};

var<immediate> globals: Globals;

@group(0) @binding(0) var tex_arr: binding_array<texture_2d<f32>>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<storage, read> gens: array<u32>;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let mvp = mat4x4<f32>(in.mvp0, in.mvp1, in.mvp2, in.mvp3);
    let clip = mvp * vec4<f32>(in.position, 1.0);

    // Remap the mesh's own NDC cube into the widget's sub-rect of the window.
    // Done in clip space — scale xy, translate by w — so the perspective
    // divide still happens afterwards and stays correct. Doing this after the
    // divide would flatten the projection.
    let ws = max(globals.window_size, vec2<f32>(1.0, 1.0));
    let center_px = in.rect.xy + in.rect.zw * 0.5;
    let center_ndc = vec2<f32>(
        (center_px.x / ws.x) * 2.0 - 1.0,
        1.0 - (center_px.y / ws.y) * 2.0,
    );
    let half_ndc = in.rect.zw / ws;

    let nrm = mat3x3<f32>(in.nrm0.xyz, in.nrm1.xyz, in.nrm2.xyz);

    let packed = in.params.y;
    let scale = unpack2x16unorm(in.params.z);
    let offs = unpack2x16unorm(in.params.w);

    var out: VertexOutput;
    out.pos = vec4<f32>(clip.xy * half_ndc + clip.w * center_ndc, clip.z, clip.w);
    out.normal = nrm * in.normal;
    out.color = in.color * in.tint;
    out.uv_tex = in.uv * scale + offs;
    out.slot_plus_one = packed & SLOT_MASK;
    out.gen = packed >> SLOT_BITS;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var base = in.color;

    if in.slot_plus_one != 0u {
        let idx = in.slot_plus_one - 1u;
        // Generation check: a recycled slot must not show the wrong texture.
        if (gens[idx] & GEN_MASK) == in.gen {
            // `tex_arr` is an *Srgb format, so the sample is already linear.
            base = base * textureSample(tex_arr[idx], samp, in.uv_tex);
        }
    }

    let n = normalize(in.normal);
    // Fixed key light plus a fill, in view space. Good enough for a widget-
    // sized scene and keeps the instance free of light parameters.
    let key = normalize(vec3<f32>(0.45, 0.75, 0.55));
    let fill = normalize(vec3<f32>(-0.5, 0.2, 0.3));
    let view = vec3<f32>(0.0, 0.0, 1.0);
    let half_v = normalize(key + view);

    let diffuse = max(dot(n, key), 0.0) + 0.25 * max(dot(n, fill), 0.0);
    let specular = pow(max(dot(n, half_v), 0.0), 48.0) * 0.3;
    let rim = pow(1.0 - max(dot(n, view), 0.0), 3.0) * 0.2;

    let lit = base.rgb * (0.16 + 0.84 * diffuse) + vec3<f32>(specular + rim);

    // Blend state is One / OneMinusSrcAlpha — output premultiplied. Values are
    // linear; the sRGB surface encodes on write.
    let a = base.a;
    return vec4<f32>(lit * a, a);
}
