@group(0) @binding(0) var<uniform> cascade: mat4x4<f32>;

struct In {
    @location(0) at: vec3<f32>,
    @location(4) place0: vec4<f32>,
    @location(5) place1: vec4<f32>,
    @location(6) place2: vec4<f32>,
    @location(7) place3: vec4<f32>,
}

@vertex
fn vs_main(v: In) -> @builtin(position) vec4<f32> {
    let place = mat4x4<f32>(v.place0, v.place1, v.place2, v.place3);
    return cascade * place * vec4<f32>(v.at, 1.0);
}
