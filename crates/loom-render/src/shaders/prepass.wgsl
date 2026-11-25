struct Frame {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> frame: Frame;

struct In {
    @location(0) at: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(4) place0: vec4<f32>,
    @location(5) place1: vec4<f32>,
    @location(6) place2: vec4<f32>,
    @location(7) place3: vec4<f32>,
}

struct Out {
    @builtin(position) @invariant clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
}

@vertex
fn vs_main(v: In) -> Out {
    let place = mat4x4<f32>(v.place0, v.place1, v.place2, v.place3);
    var out: Out;
    out.clip = frame.view_proj * (place * vec4<f32>(v.at, 1.0));
    out.normal = (place * vec4<f32>(v.normal, 0.0)).xyz;
    return out;
}

@fragment
fn fs_main(o: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(normalize(o.normal), 1.0);
}
