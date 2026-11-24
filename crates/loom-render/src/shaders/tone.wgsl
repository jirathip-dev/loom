@group(0) @binding(0) var hdr: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(index & 1u) * 4.0 - 1.0;
    let y = f32(index >> 1u) * 4.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

fn aces(x: vec3<f32>) -> vec3<f32> {
    return clamp(x * (2.51 * x + 0.03) / (x * (2.43 * x + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn mapped(at: vec2<i32>) -> vec3<f32> {
    let last = vec2<i32>(textureDimensions(hdr)) - 1;
    let texel = textureLoad(hdr, clamp(at, vec2<i32>(0), last), 0);
    return mix(texel.rgb, aces(texel.rgb), texel.a);
}

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@fragment
fn fs_main(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(at.xy);
    let centre = mapped(p);
    let north = mapped(p + vec2<i32>(0, -1));
    let south = mapped(p + vec2<i32>(0, 1));
    let east = mapped(p + vec2<i32>(1, 0));
    let west = mapped(p + vec2<i32>(-1, 0));
    let lc = luma(centre);
    let ln = luma(north);
    let ls = luma(south);
    let le = luma(east);
    let lw = luma(west);
    let high = max(lc, max(max(ln, ls), max(le, lw)));
    let low = min(lc, min(min(ln, ls), min(le, lw)));
    let range = high - low;
    if range < max(0.0312, high * 0.125) {
        return vec4<f32>(centre, 1.0);
    }
    let across_rows = abs(ln + ls - 2.0 * lc) >= abs(le + lw - 2.0 * lc);
    var other = east;
    if across_rows {
        other = select(south, north, abs(ln - lc) >= abs(ls - lc));
    } else {
        other = select(east, west, abs(lw - lc) >= abs(le - lc));
    }
    let blend = smoothstep(0.0, 1.0, clamp(abs((ln + ls + le + lw) * 0.25 - lc) / range, 0.0, 1.0));
    return vec4<f32>(mix(centre, other, blend * blend * 0.5), 1.0);
}
