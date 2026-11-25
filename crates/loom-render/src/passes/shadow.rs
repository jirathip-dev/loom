use crate::passes::lit::{DEPTH, Recipe, depth, group, layout, pipeline, uniform};
use glam::camera::rh::{proj::directx, view::look_at_mat4};
use glam::{Mat4, Vec3};
use wgpu::{
    BindGroup, Buffer, CompareFunction, Device, RenderPipeline, Sampler, ShaderStages, TextureView,
};

pub const CASCADES: usize = 3;
const MAP_SIZE: u32 = 2048;
const SLICES: [(f32, f32); CASCADES] = [(0.0, 8.0), (8.0, 30.0), (30.0, 120.0)];
const BACKOFF: f32 = 60.0;

pub struct Shadow {
    pub pipeline: RenderPipeline,
    pub layers: [TextureView; CASCADES],
    pub map: TextureView,
    pub sampler: Sampler,
    pub cascades: [(Buffer, BindGroup); CASCADES],
}

fn maps(device: &Device) -> ([TextureView; CASCADES], TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shadow"),
        size: wgpu::Extent3d {
            width: MAP_SIZE,
            height: MAP_SIZE,
            depth_or_array_layers: 3,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let layers = std::array::from_fn(|layer| {
        texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2),
            base_array_layer: u32::try_from(layer).unwrap_or(0),
            array_layer_count: Some(1),
            ..wgpu::TextureViewDescriptor::default()
        })
    });
    let map = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..wgpu::TextureViewDescriptor::default()
    });
    (layers, map)
}

impl Shadow {
    pub fn new(device: &Device) -> Self {
        let (layers, map) = maps(device);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            compare: Some(CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let matrix_layout = layout(device, "cascade", &[uniform(0, ShaderStages::VERTEX)]);
        let cascades = std::array::from_fn(|_| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cascade"),
                size: size_of::<Mat4>().try_into().unwrap_or(0),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bound = group(device, &matrix_layout, &[buffer.as_entire_binding()]);
            (buffer, bound)
        });
        let pipeline = pipeline(
            device,
            Recipe {
                label: "shadow",
                shader: wgpu::include_wgsl!("../shaders/shadow.wgsl"),
                layouts: &[&matrix_layout],
                mesh: true,
                cull: None,
                target: None,
                depth: Some(wgpu::DepthStencilState {
                    bias: wgpu::DepthBiasState {
                        constant: 2,
                        slope_scale: 2.0,
                        clamp: 0.0,
                    },
                    ..depth(true, CompareFunction::Less)
                }),
            },
        );
        Self {
            pipeline,
            layers,
            map,
            sampler,
            cascades,
        }
    }
}

/// Each cascade's light view and projection, fitted round its slice of the eye's frustum.
pub fn cascades(view_proj: Mat4, towards: Vec3) -> [Mat4; CASCADES] {
    let inverse = view_proj.inverse();
    let edges = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)].map(|(x, y)| {
        let near = inverse.project_point3(Vec3::new(x, y, 0.0));
        (near, inverse.project_point3(Vec3::new(x, y, 1.0)) - near)
    });
    let towards = towards.normalize_or(Vec3::Y);
    let up = if towards.y.abs() > 0.99 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    SLICES.map(|(from, to)| {
        let corners: Vec<Vec3> = edges
            .iter()
            .flat_map(|&(near, along)| {
                let reach = along.length();
                [from, to].map(|metres| near + along * (metres / reach).min(1.0))
            })
            .collect();
        let centre = corners.iter().sum::<Vec3>() / 8.0;
        let radius = corners
            .iter()
            .map(|corner| corner.distance(centre))
            .fold(0.0, f32::max)
            .ceil();
        let eye = centre + towards * (radius + BACKOFF);
        let view = look_at_mat4(eye, centre, up);
        directx::orthographic(
            -radius,
            radius,
            -radius,
            radius,
            0.0,
            2.0 * radius + BACKOFF,
        ) * view
    })
}
