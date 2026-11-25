use crate::passes::lit::{Recipe, depth, pipeline};
use wgpu::{BindGroupLayout, CompareFunction, Device, Face, RenderPipeline, TextureFormat};

pub const NORMAL: TextureFormat = TextureFormat::Rgba16Float;

pub struct Prepass {
    pub pipeline: RenderPipeline,
}

impl Prepass {
    pub fn new(device: &Device, frame_layout: &BindGroupLayout) -> Self {
        let pipeline = pipeline(
            device,
            Recipe {
                label: "prepass",
                shader: wgpu::include_wgsl!("../shaders/prepass.wgsl"),
                layouts: &[frame_layout],
                mesh: true,
                cull: Some(Face::Back),
                target: Some(NORMAL),
                depth: Some(depth(true, CompareFunction::Less)),
            },
        );
        Self { pipeline }
    }
}
