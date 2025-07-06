struct VertexInput {
    // instance buffer
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,

    // vertex buffer
    @location(10) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) local_pos: vec2<f32>,
    @location(3) size: vec2<f32>,
};

struct Globals {
    window_size: vec2<f32>,
};

var<push_constant> globals: Globals;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
    let local_pos = uv * in.size;
    let world_pos = in.position + local_pos;

    let ndc = vec2<f32>(
        (world_pos.x / globals.window_size.x) * 2.0 - 1.0,
        1.0 - (world_pos.y / globals.window_size.y) * 2.0
    );

    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = uv;
    out.color = in.color;
    out.local_pos = local_pos;
    out.size = in.size;

    return out;
}

@fragment
fn fs_main(out: VertexOutput) -> @location(0) vec4<f32> {
    return out.color;
}
