struct VertexInput {
    // instance buffer
    @location(0) position: vec2<f32>, // top-left in pixels
    @location(1) size: vec2<f32>,     // width/height in pixels
    @location(2) data1: vec4<f32>,    // unused here
    @location(3) data2: vec4<u32>,    // unused here

    // vertex buffer
    @location(10) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv_screen: vec2<f32>, // screen pixel coords
};

struct Globals {
    window_size: vec2<f32>,
    time: f32,
    delta_time: f32,
    mouse_pos: vec2<f32>,
    mouse_buttons: u32,
    frame: u32,
};
var<push_constant> globals : Globals;

/* -------------------- math & utils -------------------- */
fn radians(a: f32) -> f32 { return a * 3.14159265359 / 180.0; }
fn saturate(x: f32) -> f32 { return clamp(x, 0.0, 1.0); }

fn rotate_x(a_deg: f32) -> mat3x3<f32> {
    let a = radians(a_deg);
    let s = sin(a);
    let c = cos(a);
    return mat3x3<f32>(
        1.0, 0.0, 0.0,
        0.0, c, -s,
        0.0, s, c
    );
}
fn rotate_y(a_deg: f32) -> mat3x3<f32> {
    let a = radians(a_deg);
    let s = sin(a);
    let c = cos(a);
    return mat3x3<f32>(
        c, 0.0, s,
        0.0, 1.0, 0.0,
        -s, 0.0, c
    );
}

fn linear_to_srgb(c_in: vec3<f32>) -> vec3<f32> {
    // WGSL pow on negative is undefined — keep non-negative
    let c = max(c_in, vec3<f32>(0.0, 0.0, 0.0));
    let p = 1.0 / 2.2;
    return vec3<f32>(pow(c.r, p), pow(c.g, p), pow(c.b, p));
}

/* -------------------- ray & camera -------------------- */
struct Ray {
    origin: vec3<f32>,
    dir: vec3<f32>
};
struct Sphere {
    origin: vec3<f32>,
    radius: f32
};
struct Hit {
    t: f32,
    ok: bool,
    p: vec3<f32>,
    n: vec3<f32>
};
const NO_HIT: Hit = Hit(1e9, false, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 0.0));

fn primary_ray(cam_pt: vec3<f32>, eye: vec3<f32>, look_at: vec3<f32>) -> Ray {
    var fwd = normalize(look_at - eye);
    var up = vec3<f32>(0.0, 1.0, 0.0);
    let right = normalize(cross(up, fwd));
    up = normalize(cross(fwd, right));
    return Ray(eye, normalize(fwd + up * cam_pt.y + right * cam_pt.x));
}

struct Camera {
    eye: vec3<f32>,
    look_at: vec3<f32>
};
fn setup_camera() -> Camera {
    return Camera(vec3<f32>(0.0, 0.0, -2.5), vec3<f32>(0.0, 0.0, 2.0));
}

/* -------------------- noise (iq) -------------------- */
fn hash1(n: f32) -> f32 {
    return fract(sin(n) * 753.5453123);
}
fn noise_iq(x: vec3<f32>) -> f32 {
    let p = floor(x);
    var f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    let n = p.x + p.y * 157.0 + 113.0 * p.z;

    let n0 = mix(hash1(n + 0.0), hash1(n + 1.0), f.x);
    let n1 = mix(hash1(n + 157.0), hash1(n + 158.0), f.x);
    let n2 = mix(hash1(n + 113.0), hash1(n + 114.0), f.x);
    let n3 = mix(hash1(n + 270.0), hash1(n + 271.0), f.x);

    let a = mix(n0, n1, f.y);
    let b = mix(n2, n3, f.y);
    return mix(a, b, f.z);
}

fn fbm4(pos: vec3<f32>, lac: f32, init_gain: f32, gain: f32, basis_abs: bool) -> f32 {
    var p = pos;
    var H = init_gain;
    var t = 0.0;
    for (var i = 0; i < 4; i = i + 1) {
        let v = noise_iq(p);
        let b = select(v, abs(v * 2.0 - 1.0), basis_abs);
        t = t + b * H;
        p = p * lac;
        H = H * gain;
    }
    return t;
}

/* -------------------- sdf terrain -------------------- */
const PLANET: Sphere = Sphere(vec3<f32>(0.0, 0.0, 0.0), 1.0);
const MAX_HEIGHT: f32 = 0.4;

fn sdf_terrain(p: vec3<f32>) -> vec2<f32> {
    let h0 = fbm4(p * 2.0987, 2.0244, 0.454, 0.454, false);
    let n0 = smoothstep(0.35, 1.0, h0);

    let h1 = fbm4(p * 1.50987 + vec3<f32>(1.9489, 2.435, 0.5483), 2.0244, 0.454, 0.454, true);
    let n1 = smoothstep(0.6, 1.0, h1);

    let n = n0 + n1;
    let sdf = length(p) - PLANET.radius - n * MAX_HEIGHT;
    return vec2<f32>(sdf, n / MAX_HEIGHT);
}

fn terrain_normal(p: vec3<f32>) -> vec3<f32> {
    let e = 0.001;
    let dx = vec3<f32>(e, 0.0, 0.0);
    let dy = vec3<f32>(0.0, e, 0.0);
    let dz = vec3<f32>(0.0, 0.0, e);
    let fx1 = sdf_terrain(p + dx).x;
    let fx0 = sdf_terrain(p - dx).x;
    let fy1 = sdf_terrain(p + dy).x;
    let fy0 = sdf_terrain(p - dy).x;
    let fz1 = sdf_terrain(p + dz).x;
    let fz0 = sdf_terrain(p - dz).x;
    return normalize(vec3<f32>(fx1 - fx0, fy1 - fy0, fz1 - fz0));
}

/* -------------------- sphere intersect -------------------- */
fn hit_sphere(r: Ray, s: Sphere) -> Hit {
    let rc = s.origin - r.origin;
    let tca = dot(rc, r.dir);
    if tca < 0.0 { return NO_HIT; }
    let d2 = dot(rc, rc) - tca * tca;
    let rad2 = s.radius * s.radius;
    if d2 > rad2 { return NO_HIT; }
    let thc = sqrt(rad2 - d2);
    var t0 = tca - thc;
    let t1 = tca + thc;
    if t0 < 0.0 { t0 = t1; }
    let p = r.origin + r.dir * t0;
    let n = normalize((p - s.origin) / s.radius);
    return Hit(t0, true, p, n);
}

/* -------------------- background & lighting -------------------- */
fn background(dir: vec3<f32>) -> vec3<f32> {
    // clamp base before pow to avoid NaNs
    let sun = max(dot(dir, vec3<f32>(0.0, 0.0, 1.0)), 0.0);
    var sky = mix(vec3<f32>(0.0, 0.05, 0.2),
        vec3<f32>(0.15, 0.3, 0.4),
        1.0 - dir.y);
    let sun_col = vec3<f32>(1.0, 0.9, 0.55);
    sky = sky + sun_col * min(pow(sun, 30.0) * 5.0, 1.0);
    sky = sky + sun_col * min(pow(sun, 10.0) * 0.6, 1.0);
    return sky;
}

fn lights(L: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    var diff = vec3<f32>(0.0, 0.0, 0.0);
    diff = diff + max(0.0, dot(L, n)) * vec3<f32>(7.0, 5.0, 3.0);
    let hemi = clamp(0.25 + 0.5 * n.y, 0.0, 1.0);
    diff = diff + hemi * vec3<f32>(0.4, 0.6, 0.8) * 0.2;
    let amb = clamp(0.12 + 0.8 * max(0.0, dot(-L, n)), 0.0, 1.0);
    diff = diff + amb * vec3<f32>(0.4, 0.5, 0.6);
    return diff;
}

fn shade_terrain(pos: vec3<f32>, w_normal: vec3<f32>, rot: mat3x3<f32>, df: vec2<f32>) -> vec3<f32> {
    let c_water = vec3<f32>(0.015, 0.110, 0.455);
    let c_grass = vec3<f32>(0.086, 0.132, 0.018);
    let c_beach = vec3<f32>(0.153, 0.172, 0.121);
    let c_rock = vec3<f32>(0.080, 0.050, 0.030);
    let c_snow = vec3<f32>(0.600, 0.600, 0.600);

    let h = df.y;
    let normal = terrain_normal(pos);
    let NdotUp = dot(normal, normalize(pos));

    let snow_mix = smoothstep(0.4, 1.0, h);
    let rock = mix(c_rock, c_snow, smoothstep(1.0 - 0.3 * snow_mix, 1.0 - 0.2 * snow_mix, NdotUp));
    let grass = mix(c_grass, rock, smoothstep(0.211, 0.351, h));
    let shore = mix(c_beach, grass, smoothstep(0.17, 0.211, h));
    let water = mix(c_water * 0.5, c_water, smoothstep(0.0, 0.05, h));

    let L = normalize(rot * vec3<f32>(1.0, 1.0, 0.0));
    let lit_land = lights(L, normal) * shore;
    let lit_ocean = lights(L, w_normal) * water;

    return mix(lit_ocean, lit_land, smoothstep(0.05, 0.17, h));
}

/* -------------------- clouds (simple volume) -------------------- */
struct Vol {
    origin: vec3<f32>,
    pos: vec3<f32>,
    height: f32,
    coeff_absorb: f32,
    T: f32,
    C: vec3<f32>,
    alpha: f32,
};

fn vol_begin(origin: vec3<f32>, coeff: f32) -> Vol {
    return Vol(origin, origin, 0.0, coeff, 1.0, vec3<f32>(0.0, 0.0, 0.0), 0.0);
}

fn vol_integrate(vol: ptr<function, Vol>, density: f32, dt: f32) {
    let T_i = exp(-(*vol).coeff_absorb * density * dt);
    (*vol).T = (*vol).T * T_i;
    let illum = exp((*vol).height) / 0.055;
    (*vol).C = (*vol).C + (*vol).T * illum * density * dt;
    (*vol).alpha = (*vol).alpha + (1.0 - T_i) * (1.0 - (*vol).alpha);
}

fn clouds_map(vol: ptr<function, Vol>, t_step: f32) {
    let p = (*vol).pos * 3.2343 + vec3<f32>(0.35, 13.35, 2.67);
    var dens = fbm4(p, 2.0276, 0.5, 0.5, true);
    let coverage = 0.29475675;
    let fuzzy = 0.0335;
    dens = dens * smoothstep(coverage, coverage + fuzzy, dens);
    let band = smoothstep(0.2, 0.65, (*vol).height) * (1.0 - smoothstep(0.35, 0.65, (*vol).height));
    dens = dens * band;
    vol_integrate(vol, dens, t_step);
}

fn march_clouds(eye: Ray, vol: ptr<function, Vol>, max_travel: f32, rot: mat3x3<f32>) {
    let steps = 75;
    let t_step = MAX_HEIGHT * 4.0 / f32(steps);
    var t = 0.0;
    for (var i = 0; i < steps; i = i + 1) {
        if t > max_travel || (*vol).alpha >= 1.0 { return; }
        let o = (*vol).origin + t * eye.dir;
        (*vol).pos = rot * (o - PLANET.origin);
        (*vol).height = (length((*vol).pos) - PLANET.radius) / MAX_HEIGHT;
        t = t + t_step;
        clouds_map(vol, t_step);
    }
}

fn march_clouds_shadow(local_origin: vec3<f32>, local_up: vec3<f32>, rot_cloud: mat3x3<f32>) -> f32 {
    var vol = vol_begin(local_origin, 30.034);
    let steps = 5;
    let t_step = MAX_HEIGHT / f32(steps);
    var t = 0.0;
    for (var i = 0; i < steps; i = i + 1) {
        let o = vol.origin + t * local_up;
        vol.pos = rot_cloud * (o - PLANET.origin);
        vol.height = (length(vol.pos) - PLANET.radius) / MAX_HEIGHT;
        t = t + t_step;
        clouds_map(&vol, t_step);
    }
    return mix(0.7, 1.0, step(vol.alpha, 0.33));
}

/* -------------------- render -------------------- */
const FOV: f32 = 0.5773502691896257; // tan(radians(30.0))
const MAX_RAY_DIST: f32 = MAX_HEIGHT * 4.0;

fn render(eye: Ray) -> vec3<f32> {
    let rot_y = rotate_y(27.0);
    var rot = rotate_x(-12.0 * globals.time) * rot_y;
    var rot_cloud = rotate_x(8.0 * globals.time) * rot_y;

    if globals.mouse_buttons != 0u {
        rot = rotate_y(-globals.mouse_pos.x) * rotate_x(globals.mouse_pos.y);
        rot_cloud = rot;
    }

    var atm = PLANET;
    atm.radius = PLANET.radius + MAX_HEIGHT;

    let hit_atm = hit_sphere(eye, atm);
    if !hit_atm.ok {
        return background(eye.dir);
    }

    var t = 0.0;
    var df = vec2<f32>(1.0, MAX_HEIGHT);
    var pos = vec3<f32>(0.0, 0.0, 0.0);
    var max_cloud_ray = MAX_RAY_DIST;

    // sphere-trace terrain
    for (var i = 0; i < 120; i = i + 1) {
        if t > MAX_RAY_DIST { break; }
        let o = hit_atm.p + t * eye.dir;
        pos = rot * (o - PLANET.origin);
        df = sdf_terrain(pos);
        if df.x < 0.005 {
            max_cloud_ray = t;
            break;
        }
        t = t + df.x * 0.4567;
    }

    // clouds along the view ray
    var vol = vol_begin(hit_atm.p, 30.034);
    march_clouds(eye, &vol, max_cloud_ray, rot_cloud);

    if df.x < 0.005 {
        let terr = shade_terrain(pos, normalize(pos), rot, df);
        let local_pos = transpose(rot) * pos;
        let shadow = march_clouds_shadow(local_pos, normalize(local_pos), rot_cloud);
        let c_terr = terr * shadow;
        return mix(c_terr, vol.C, vol.alpha);
    } else {
        return mix(background(eye.dir), vol.C, vol.alpha);
    }
}

/* -------------------- pipeline entry points -------------------- */
@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    // Use the same mapping as your working UI VS
    let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
    let local_pos = uv * in.size;
    let world_pos = in.position + local_pos;

    let ws = max(globals.window_size, vec2<f32>(1.0, 1.0)); // avoid div-by-zero
    let ndc = vec2<f32>(
        (world_pos.x / ws.x) * 2.0 - 1.0,
        1.0 - (world_pos.y / ws.y) * 2.0
    );

    var out: VertexOutput;
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    out.uv_screen = world_pos; // pass pixel coords to FS
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Rebuild ray from pixel coords
    let ws = max(globals.window_size, vec2<f32>(1.0, 1.0));
    var fragCoord = in.uv_screen;
    // Convert to bottom-left origin for camera math
    fragCoord.y = ws.y - fragCoord.y;

    let aspect = vec2<f32>(ws.x / ws.y, 1.0);

    let cam = setup_camera();
    let eye = cam.eye;
    let look_at = cam.look_at;

    let point_ndc = fragCoord / ws;
    let point_cam = vec3<f32>((2.0 * point_ndc - vec2<f32>(1.0, 1.0)) * aspect * FOV, -1.0);

    let ray = primary_ray(point_cam, eye, look_at);

    var color = render(ray);

    // tonemap + gamma (keep non-negative to avoid pow NaNs)
    color = max(color, vec3<f32>(0.0, 0.0, 0.0));
    color = color / (vec3<f32>(1.0, 1.0, 1.0) + color);
    color = linear_to_srgb(color);

    return vec4<f32>(color, 1.0);
}
