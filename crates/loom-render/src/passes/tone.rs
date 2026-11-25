use crate::passes::lit::{Recipe, group, layout, pipeline, texture};
use wgpu::{
    BindGroupLayout, Device, RenderPipeline, TextureFormat, TextureSampleType, TextureViewDimension,
};

pub const OUTPUT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

pub struct Tone {
    pub layout: BindGroupLayout,
    pub pipeline: RenderPipeline,
}

impl Tone {
    pub fn new(device: &Device) -> Self {
        let layout = layout(
            device,
            "tone",
            &[texture(
                0,
                TextureSampleType::Float { filterable: false },
                TextureViewDimension::D2,
            )],
        );
        let pipeline = pipeline(
            device,
            Recipe {
                label: "tone",
                shader: wgpu::include_wgsl!("../shaders/tone.wgsl"),
                layouts: &[&layout],
                mesh: false,
                cull: None,
                target: Some(OUTPUT),
                depth: None,
            },
        );
        Self { layout, pipeline }
    }

    pub fn bind(&self, device: &Device, hdr: &wgpu::TextureView) -> wgpu::BindGroup {
        group(
            device,
            &self.layout,
            &[wgpu::BindingResource::TextureView(hdr)],
        )
    }
}
