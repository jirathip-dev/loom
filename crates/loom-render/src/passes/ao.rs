use crate::passes::lit::{Recipe, group, layout, pipeline, texture};
use wgpu::{
    BindGroup, BindGroupLayout, BindingResource, Device, RenderPipeline, TextureFormat,
    TextureSampleType, TextureView, TextureViewDimension,
};

pub const OCCLUSION: TextureFormat = TextureFormat::R8Unorm;

pub struct Ao {
    pub layout: BindGroupLayout,
    pub pipeline: RenderPipeline,
}

impl Ao {
    pub fn new(device: &Device, frame_layout: &BindGroupLayout) -> Self {
        let unfiltered = TextureSampleType::Float { filterable: false };
        let layout = layout(
            device,
            "ao",
            &[
                texture(0, TextureSampleType::Depth, TextureViewDimension::D2),
                texture(1, unfiltered, TextureViewDimension::D2),
            ],
        );
        let pipeline = pipeline(
            device,
            Recipe {
                label: "ao",
                shader: wgpu::include_wgsl!("../shaders/ao.wgsl"),
                layouts: &[frame_layout, &layout],
                mesh: false,
                cull: None,
                target: Some(OCCLUSION),
                depth: None,
            },
        );
        Self { layout, pipeline }
    }

    pub fn bind(&self, device: &Device, depth: &TextureView, normal: &TextureView) -> BindGroup {
        group(
            device,
            &self.layout,
            &[
                BindingResource::TextureView(depth),
                BindingResource::TextureView(normal),
            ],
        )
    }
}
