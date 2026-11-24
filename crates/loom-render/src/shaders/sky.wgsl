struct Frame {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    towards: vec4<f32>,
    light: vec4<f32>,
    ambient: vec4<f32>,
    sky: vec4<f32>,
}

@group(0) @binding(0) var<uniform> frame: Frame;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(index & 1u) * 4.0 - 1.0;
    let y = f32(index >> 1u) * 4.0 - 1.0;
    return vec4<f32>(x, y, 1.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(frame.sky.rgb, 0.0);
}
