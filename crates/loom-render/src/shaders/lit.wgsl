const PI: f32 = 3.14159265;

struct Frame {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    towards: vec4<f32>,
    light: vec4<f32>,
    ambient: vec4<f32>,
    sky: vec4<f32>,
}

struct Surface {
    colour_roughness: vec4<f32>,
    sheen_fuzz: vec4<f32>,
}

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var<uniform> surface: Surface;
@group(1) @binding(1) var fibre: texture_2d<f32>;
@group(1) @binding(2) var fibre_sampler: sampler;

struct In {
    @location(0) at: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) colour: vec4<f32>,
    @location(4) place0: vec4<f32>,
    @location(5) place1: vec4<f32>,
    @location(6) place2: vec4<f32>,
    @location(7) place3: vec4<f32>,
    @location(8) tint: vec4<f32>,
}

struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) colour: vec4<f32>,
}

@vertex
fn vs_main(v: In) -> Out {
    let place = mat4x4<f32>(v.place0, v.place1, v.place2, v.place3);
    let world = place * vec4<f32>(v.at, 1.0);
    var out: Out;
    out.clip = frame.view_proj * world;
    out.world = world.xyz;
    out.normal = normalize((place * vec4<f32>(v.normal, 0.0)).xyz);
    out.uv = v.uv;
    out.colour = v.colour * v.tint;
    return out;
}

fn mapped_normal(n: vec3<f32>, world: vec3<f32>, uv: vec2<f32>, texel: vec3<f32>) -> vec3<f32> {
    let dp1 = dpdx(world);
    let dp2 = dpdy(world);
    let duv1 = dpdx(uv);
    let duv2 = dpdy(uv);
    let dp2perp = cross(dp2, n);
    let dp1perp = cross(n, dp1);
    let t = dp2perp * duv1.x + dp1perp * duv2.x;
    let b = dp2perp * duv1.y + dp1perp * duv2.y;
    let scale = inverseSqrt(max(max(dot(t, t), dot(b, b)), 1e-12));
    let local = texel * 2.0 - 1.0;
    return normalize(t * scale * local.x + b * scale * local.y + n * local.z);
}

fn ggx(n_h: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let d = n_h * n_h * (a2 - 1.0) + 1.0;
    return a2 / (PI * d * d);
}

fn smith(n_v: f32, n_l: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let gv = n_l * sqrt(n_v * n_v * (1.0 - a2) + a2);
    let gl = n_v * sqrt(n_l * n_l * (1.0 - a2) + a2);
    return 0.5 / max(gv + gl, 1e-5);
}

fn charlie(n_h: f32, fuzz: f32) -> f32 {
    let inv = 1.0 / max(fuzz, 0.05);
    let sin2 = max(1.0 - n_h * n_h, 0.0);
    return (2.0 + inv) * pow(sin2, inv * 0.5) / (2.0 * PI);
}

fn ashikhmin(n_v: f32, n_l: f32) -> f32 {
    return 1.0 / max(4.0 * (n_l + n_v - n_l * n_v), 1e-5);
}

@fragment
fn fs_main(o: Out) -> @location(0) vec4<f32> {
    let texel = textureSample(fibre, fibre_sampler, o.uv);
    let n = mapped_normal(normalize(o.normal), o.world, o.uv, texel.rgb);
    let v = normalize(frame.eye.xyz - o.world);
    let l = normalize(frame.towards.xyz);
    let h = normalize(v + l);
    let n_l = max(dot(n, l), 0.0);
    let n_v = max(dot(n, v), 1e-4);
    let n_h = max(dot(n, h), 0.0);
    let v_h = max(dot(v, h), 0.0);
    let base = surface.colour_roughness.rgb * o.colour.rgb * texel.a;
    let alpha = max(surface.colour_roughness.w * surface.colour_roughness.w, 0.002);
    let fresnel = 0.04 + 0.96 * pow(1.0 - v_h, 5.0);
    let specular = ggx(n_h, alpha) * smith(n_v, n_l, alpha) * fresnel;
    let diffuse = base / PI * (1.0 - fresnel);
    let sheen = surface.sheen_fuzz.rgb * charlie(n_h, surface.sheen_fuzz.w) * ashikhmin(n_v, n_l);
    let direct = (diffuse + vec3<f32>(specular) + sheen) * frame.light.rgb * n_l;
    return vec4<f32>(direct + frame.ambient.rgb * base, 1.0);
}
