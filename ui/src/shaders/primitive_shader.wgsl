struct VertexInput {
    // instance buffer
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) kind: u32,
    @location(3) fill_color: vec4<f32>,
    @location(4) border_color: vec4<f32>,
    @location(5) border_radius: vec4<f32>,
    @location(6) border_width: vec4<f32>,

    // vertex buffer
    @location(10) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fill_color: vec4<f32>,
    @location(2) border_color: vec4<f32>,
    @location(3) border_radius: vec4<f32>,
    @location(4) border_width: vec4<f32>,
    @location(5) local_pos: vec2<f32>,
    @location(6) size: vec2<f32>,
};

struct Globals {
    cursor_position: vec2<f32>,
    window_size: vec2<f32>,
};

var<push_constant> globals: Globals;

@vertex
fn vs_main(i: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let uv = vec2<f32>(i.uv.x, 1.0 - i.uv.y);
    let local_pos = uv * i.size;
    let world_pos = i.position + local_pos;

    let ndc = vec2<f32>(
        (world_pos.x / globals.window_size.x) * 2.0 - 1.0,
        1.0 - (world_pos.y / globals.window_size.y) * 2.0
    );

    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = uv;
    out.fill_color = i.fill_color;
    out.border_color = i.border_color;
    out.border_radius = i.border_radius;
    out.border_width = i.border_width;
    out.local_pos = local_pos;
    out.size = i.size;

    return out;
}

@fragment
fn fs_main(i: VertexOutput) -> @location(0) vec4<f32> {
    let pos = i.local_pos;
    let size = i.size;
    let half_size = size * 0.5;
    let center = half_size;
    let p = pos - center;
    let abs_p = abs(p);

    // choose the corner radius that applies to this fragment
    let ix = select(0u, 1u, i.uv.x >= 0.5);
    let iy = select(0u, 1u, i.uv.y >= 0.5);
    let idx = ix | (iy << 1u);   // bit-field:  yx  (0..3)
    let corner_radius = i.border_radius[idx];

    let rect = half_size - vec2<f32>(corner_radius);
    let q = abs_p - rect;
    let dist = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - corner_radius;

    // per - side border width, already in your code
    let d_top = i.uv.y;
    let d_bottom = 1.0 - i.uv.y;
    let d_left = i.uv.x;
    let d_right = 1.0 - i.uv.x;

    let inv_top = 1.0 / (d_top + 1e-5);
    let inv_right = 1.0 / (d_right + 1e-5);
    let inv_bottom = 1.0 / (d_bottom + 1e-5);
    let inv_left = 1.0 / (d_left + 1e-5);

    let sum = inv_top + inv_right + inv_bottom + inv_left;
    let w_top = inv_top / sum;
    let w_right = inv_right / sum;
    let w_bottom = inv_bottom / sum;
    let w_left = inv_left / sum;
    let side_width = i.border_width.x * w_top + i.border_width.y * w_right + i.border_width.z * w_bottom + i.border_width.w * w_left;

    // Antialiasing ramps
    let px = fwidth(dist);
    let w = max(px, 1e-4);

    let t_in = clamp(0.5 - (dist + side_width) / w, 0.0, 1.0);  // fill ↔ border
    let t_out = clamp(0.5 - dist / w, 0.0, 1.0);                // border ↔ outside

    let outside_color = vec4<f32>(i.border_color.rgb, 0.0);

    // first blend(fill,↔ border), then second (order ↔,outside)
    var color = mix(i.fill_color, i.border_color, 1.0 - t_in);

    color = mix(color, outside_color, 1.0 - t_out);

    // premultiply alpha so ramps come out of α, not RGB
    color = vec4<f32>(color.rgb * color.a, color.a);

    return color;
}
