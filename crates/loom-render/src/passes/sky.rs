use crate::passes::lit::{HDR, Recipe, depth, pipeline};
use wgpu::{BindGroupLayout, CompareFunction, Device, RenderPipeline};

pub struct Sky {
    pub pipeline: RenderPipeline,
}

impl Sky {
    pub fn new(device: &Device, frame_layout: &BindGroupLayout) -> Self {
        let pipeline = pipeline(
            device,
            Recipe {
                label: "sky",
                shader: wgpu::include_wgsl!("../shaders/sky.wgsl"),
                layouts: &[frame_layout],
                mesh: false,
                cull: None,
                target: Some(HDR),
                depth: Some(depth(false, CompareFunction::LessEqual)),
            },
        );
        Self { pipeline }
    }
}
