const SAMPLES: u32 = 16u;
const RADIUS: f32 = 0.5;
const GOLDEN: f32 = 2.39996323;

struct Frame {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    towards: vec4<f32>,
    light: vec4<f32>,
    ambient: vec4<f32>,
    sky: vec4<f32>,
    inverse: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var depth: texture_depth_2d;
@group(1) @binding(1) var normals: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(index & 1u) * 4.0 - 1.0;
    let y = f32(index >> 1u) * 4.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

fn world(uv: vec2<f32>, d: f32) -> vec3<f32> {
    let at = frame.inverse * vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, d, 1.0);
    return at.xyz / at.w;
}

fn direction(i: u32) -> vec3<f32> {
    let k = (f32(i) + 0.5) / f32(SAMPLES);
    let y = 1.0 - 2.0 * k;
    let r = sqrt(1.0 - y * y);
    let turn = f32(i) * GOLDEN;
    let reach = mix(0.15, 1.0, fract(f32(i) * 0.618034));
    return vec3<f32>(cos(turn) * r, y, sin(turn) * r) * reach;
}

@fragment
fn fs_main(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(depth));
    let d = textureLoad(depth, vec2<i32>(at.xy), 0);
    if d >= 1.0 {
        return vec4<f32>(1.0);
    }
    let p = world(at.xy / size, d);
    let n = normalize(textureLoad(normals, vec2<i32>(at.xy), 0).xyz);
    let from_eye = distance(frame.eye.xyz, p);
    var hidden = 0.0;
    for (var i = 0u; i < SAMPLES; i++) {
        var k = direction(i);
        if dot(k, n) < 0.0 {
            k = -k;
        }
        let s = p + n * 0.02 + k * RADIUS;
        let clip = frame.view_proj * vec4<f32>(s, 1.0);
        let uv = vec2<f32>(clip.x / clip.w * 0.5 + 0.5, 0.5 - clip.y / clip.w * 0.5);
        if clip.w <= 0.0 || any(uv < vec2<f32>(0.0)) || any(uv >= vec2<f32>(1.0)) {
            continue;
        }
        let q = world(uv, textureLoad(depth, vec2<i32>(uv * size), 0));
        let behind = distance(frame.eye.xyz, s) - distance(frame.eye.xyz, q);
        let near = smoothstep(0.0, 1.0, RADIUS / max(abs(from_eye - distance(frame.eye.xyz, q)), 1e-4));
        hidden += select(0.0, near, behind > 0.02);
    }
    return vec4<f32>(1.0 - hidden / f32(SAMPLES));
}
