use crate::{Instance, Vertex};
use wgpu::{
    BindGroup, BindGroupLayout, BindGroupLayoutEntry, BindingResource, BindingType,
    BufferBindingType, CompareFunction, DepthStencilState, Device, Face, RenderPipeline,
    ShaderModuleDescriptor, ShaderStages, TextureFormat, TextureSampleType, TextureViewDimension,
    VertexAttribute,
};

pub const HDR: TextureFormat = TextureFormat::Rgba16Float;
pub const DEPTH: TextureFormat = TextureFormat::Depth32Float;

const VERTEX_ATTRIBUTES: [VertexAttribute; 4] = wgpu::vertex_attr_array![
    0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Unorm8x4,
];
const INSTANCE_ATTRIBUTES: [VertexAttribute; 5] = wgpu::vertex_attr_array![
    4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4,
];

pub struct Lit {
    pub frame_layout: BindGroupLayout,
    pub material_layout: BindGroupLayout,
    pub light_layout: BindGroupLayout,
    pub pipeline: RenderPipeline,
}

pub struct Recipe<'a> {
    pub label: &'a str,
    pub shader: ShaderModuleDescriptor<'a>,
    pub layouts: &'a [&'a BindGroupLayout],
    pub mesh: bool,
    pub cull: Option<Face>,
    pub target: Option<TextureFormat>,
    pub depth: Option<DepthStencilState>,
}

pub fn uniform(binding: u32, visibility: ShaderStages) -> BindGroupLayoutEntry {
    entry(
        binding,
        visibility,
        BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
    )
}

pub fn texture(
    binding: u32,
    sample_type: TextureSampleType,
    view_dimension: TextureViewDimension,
) -> BindGroupLayoutEntry {
    entry(
        binding,
        ShaderStages::FRAGMENT,
        BindingType::Texture {
            sample_type,
            view_dimension,
            multisampled: false,
        },
    )
}

pub fn entry(binding: u32, visibility: ShaderStages, ty: BindingType) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility,
        ty,
        count: None,
    }
}

pub fn layout(device: &Device, label: &str, entries: &[BindGroupLayoutEntry]) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries,
    })
}

pub fn group(
    device: &Device,
    layout: &BindGroupLayout,
    resources: &[BindingResource],
) -> BindGroup {
    let entries: Vec<wgpu::BindGroupEntry> = (0..)
        .zip(resources)
        .map(|(binding, resource)| wgpu::BindGroupEntry {
            binding,
            resource: resource.clone(),
        })
        .collect();
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
        entries: &entries,
    })
}

pub fn depth(write: bool, compare: CompareFunction) -> DepthStencilState {
    DepthStencilState {
        format: DEPTH,
        depth_write_enabled: Some(write),
        depth_compare: Some(compare),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

pub fn pipeline(device: &Device, recipe: Recipe) -> RenderPipeline {
    let shader = device.create_shader_module(recipe.shader);
    let layouts: Vec<Option<&BindGroupLayout>> = recipe.layouts.iter().copied().map(Some).collect();
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(recipe.label),
        bind_group_layouts: &layouts,
        immediate_size: 0,
    });
    let buffers = [
        Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>().try_into().unwrap_or(0),
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<Instance>().try_into().unwrap_or(0),
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &INSTANCE_ATTRIBUTES,
        }),
    ];
    let targets = [recipe.target.map(Into::into)];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(recipe.label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: if recipe.mesh { &buffers } else { &[] },
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: recipe.cull,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: recipe.depth,
        multisample: wgpu::MultisampleState::default(),
        fragment: recipe.target.map(|_| wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

impl Lit {
    pub fn new(device: &Device) -> Self {
        let frame_layout = layout(
            device,
            "frame",
            &[uniform(0, ShaderStages::VERTEX_FRAGMENT)],
        );
        let material_layout = layout(
            device,
            "material",
            &[
                uniform(0, ShaderStages::FRAGMENT),
                texture(
                    1,
                    TextureSampleType::Float { filterable: true },
                    TextureViewDimension::D2,
                ),
                entry(
                    2,
                    ShaderStages::FRAGMENT,
                    BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                ),
            ],
        );
        let light_layout = layout(
            device,
            "light",
            &[
                texture(0, TextureSampleType::Depth, TextureViewDimension::D2Array),
                entry(
                    1,
                    ShaderStages::FRAGMENT,
                    BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                ),
                texture(
                    2,
                    TextureSampleType::Float { filterable: false },
                    TextureViewDimension::D2,
                ),
            ],
        );
        let pipeline = pipeline(
            device,
            Recipe {
                label: "lit",
                shader: wgpu::include_wgsl!("../shaders/lit.wgsl"),
                layouts: &[&frame_layout, &material_layout, &light_layout],
                mesh: true,
                cull: Some(Face::Back),
                target: Some(HDR),
                depth: Some(depth(false, CompareFunction::LessEqual)),
            },
        );
        Self {
            frame_layout,
            material_layout,
            light_layout,
            pipeline,
        }
    }
}
