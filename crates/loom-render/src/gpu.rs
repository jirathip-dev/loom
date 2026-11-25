use crate::passes::{
    ao::{Ao, OCCLUSION},
    lit::{DEPTH, HDR, Lit, group},
    prepass::{NORMAL, Prepass},
    shadow::{CASCADES, Shadow, cascades},
    sky::Sky,
    tone::{OUTPUT, Tone},
};
use crate::{
    FrameStats, GpuError, Instance, Material, MaterialId, MeshId, Scene, TextureId, Vertex,
};
use glam::Mat4;
use std::ops::Range;
use std::time::Instant;
use wgpu::util::DeviceExt;

const ROW_ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameUniform {
    view_proj: Mat4,
    eye: [f32; 4],
    towards: [f32; 4],
    light: [f32; 4],
    ambient: [f32; 4],
    sky: [f32; 4],
    inverse: Mat4,
    cascades: [Mat4; CASCADES],
}

struct Mesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
}

struct Targets {
    hdr: wgpu::TextureView,
    depth: wgpu::TextureView,
    normal: wgpu::TextureView,
    occlusion: wgpu::TextureView,
    output: wgpu::Texture,
    output_view: wgpu::TextureView,
    tone: wgpu::BindGroup,
    ao: wgpu::BindGroup,
    light: wgpu::BindGroup,
}

type Drawn<'a> = (&'a Mesh, &'a wgpu::BindGroup, Range<u32>);

/// A GPU device with the passes, the uploaded meshes, textures and materials, and its render targets.
pub struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: (u32, u32),
    lit: Lit,
    sky: Sky,
    tone: Tone,
    shadow: Shadow,
    prepass: Prepass,
    ao: Ao,
    frame: wgpu::Buffer,
    frame_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    flat: wgpu::TextureView,
    targets: Targets,
    meshes: Vec<Option<Mesh>>,
    textures: Vec<wgpu::TextureView>,
    materials: Vec<wgpu::BindGroup>,
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(out) = future.as_mut().poll(&mut cx) {
            return out;
        }
        std::thread::yield_now();
    }
}

fn extent((width, height): (u32, u32)) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: width.max(1),
        height: height.max(1),
        depth_or_array_layers: 1,
    }
}

fn target(
    device: &wgpu::Device,
    size: (u32, u32),
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: extent(size),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn targets(device: &wgpu::Device, size: (u32, u32), gpu: (&Tone, &Ao, &Lit, &Shadow)) -> Targets {
    let (tone, ao, lit, shadow) = gpu;
    let usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
    let sampled = |format| {
        target(
            device,
            size,
            format,
            usage | wgpu::TextureUsages::TEXTURE_BINDING,
        )
        .create_view(&wgpu::TextureViewDescriptor::default())
    };
    let (hdr, depth, normal, occlusion) = (
        sampled(HDR),
        sampled(DEPTH),
        sampled(NORMAL),
        sampled(OCCLUSION),
    );
    let output = target(device, size, OUTPUT, usage | wgpu::TextureUsages::COPY_SRC);
    let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());
    let light = group(
        device,
        &lit.light_layout,
        &[
            wgpu::BindingResource::TextureView(&shadow.map),
            wgpu::BindingResource::Sampler(&shadow.sampler),
            wgpu::BindingResource::TextureView(&occlusion),
        ],
    );
    Targets {
        tone: tone.bind(device, &hdr),
        ao: ao.bind(device, &depth, &normal),
        light,
        hdr,
        depth,
        normal,
        occlusion,
        output,
        output_view,
    }
}

fn begin<'e>(
    encoder: &'e mut wgpu::CommandEncoder,
    label: &str,
    colour: Option<&wgpu::TextureView>,
    depth: Option<(&wgpu::TextureView, wgpu::LoadOp<f32>)>,
) -> wgpu::RenderPass<'e> {
    let colour = colour.map(|view| wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            store: wgpu::StoreOp::Store,
        },
    });
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: if colour.is_some() {
            std::slice::from_ref(&colour)
        } else {
            &[]
        },
        depth_stencil_attachment: depth.map(|(view, load)| {
            wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

fn geometry(
    pass: &mut wgpu::RenderPass<'_>,
    instances: &wgpu::Buffer,
    draws: &[Drawn],
    material_slot: Option<u32>,
) {
    if instances.size() == 0 {
        return;
    }
    pass.set_vertex_buffer(1, instances.slice(..));
    for (mesh, material, range) in draws {
        if let Some(slot) = material_slot {
            pass.set_bind_group(slot, *material, &[]);
        }
        pass.set_vertex_buffer(0, mesh.vertices.slice(..));
        pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..mesh.count, 0, range.clone());
    }
}

fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pixels: &[u8],
    size: (u32, u32),
) -> wgpu::TextureView {
    let size = extent(size);
    let len = usize::try_from(u64::from(size.width) * u64::from(size.height) * 4).unwrap_or(0);
    let data: Vec<u8> = pixels
        .iter()
        .copied()
        .chain(std::iter::repeat(0))
        .take(len)
        .collect();
    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &data,
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn frame_uniform(scene: &Scene) -> FrameUniform {
    let eye = scene.eye.view.inverse().w_axis;
    let view_proj = scene.eye.proj * scene.eye.view;
    let light = scene.light.towards;
    let [lr, lg, lb] = scene.light.colour;
    let [ar, ag, ab] = scene.ambient;
    let [sr, sg, sb] = scene.sky;
    FrameUniform {
        view_proj,
        eye: eye.to_array(),
        towards: [light.x, light.y, light.z, 0.0],
        light: [lr, lg, lb, 0.0],
        ambient: [ar, ag, ab, 0.0],
        sky: [sr, sg, sb, 0.0],
        inverse: view_proj.inverse(),
        cascades: cascades(view_proj, light),
    }
}

fn ms(range: Range<Instant>) -> f32 {
    range.end.duration_since(range.start).as_secs_f32() * 1000.0
}

impl Gpu {
    /// A GPU that renders into an image of `size` pixels, with no window.
    pub fn offscreen(size: (u32, u32)) -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .map_err(|e| GpuError::NoAdapter(e.to_string()))?;
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .map_err(|e| GpuError::NoDevice(e.to_string()))?;
        let lit = Lit::new(&device);
        let sky = Sky::new(&device, &lit.frame_layout);
        let tone = Tone::new(&device);
        let shadow = Shadow::new(&device);
        let prepass = Prepass::new(&device, &lit.frame_layout);
        let ao = Ao::new(&device, &lit.frame_layout);
        let frame = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame"),
            size: size_of::<FrameUniform>().try_into().unwrap_or(0),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_group = group(&device, &lit.frame_layout, &[frame.as_entire_binding()]);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let targets = targets(&device, size, (&tone, &ao, &lit, &shadow));
        Ok(Self {
            flat: upload(&device, &queue, &[128, 128, 255, 255], (1, 1)),
            device,
            queue,
            size,
            lit,
            sky,
            tone,
            shadow,
            prepass,
            ao,
            frame,
            frame_group,
            sampler,
            targets,
            meshes: Vec::new(),
            textures: Vec::new(),
            materials: Vec::new(),
        })
    }

    pub fn mesh(&mut self, vertices: &[Vertex], indices: &[u32]) -> MeshId {
        let buffer = |contents: &[u8], usage| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents,
                    usage,
                })
        };
        let mesh = Mesh {
            vertices: buffer(bytemuck::cast_slice(vertices), wgpu::BufferUsages::VERTEX),
            indices: buffer(bytemuck::cast_slice(indices), wgpu::BufferUsages::INDEX),
            count: indices.len().try_into().unwrap_or(u32::MAX),
        };
        self.meshes.push(Some(mesh));
        MeshId(self.meshes.len() - 1)
    }

    pub fn free(&mut self, mesh: MeshId) {
        if let Some(slot) = self.meshes.get_mut(mesh.0) {
            *slot = None;
        }
    }

    /// An RGBA8 texture: in a fibre texture, rgb is the tangent-space normal and alpha the fibre's shade.
    pub fn texture(&mut self, pixels: &[u8], size: (u32, u32)) -> TextureId {
        self.textures
            .push(upload(&self.device, &self.queue, pixels, size));
        TextureId(self.textures.len() - 1)
    }

    pub fn material(&mut self, material: &Material) -> MaterialId {
        let [r, g, b] = material.colour;
        let [sr, sg, sb] = material.sheen;
        let values = [r, g, b, material.roughness, sr, sg, sb, material.fuzz];
        let uniform = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("material"),
                contents: bytemuck::cast_slice(&values),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let fibre = self.textures.get(material.fibre.0).unwrap_or(&self.flat);
        let bound = group(
            &self.device,
            &self.lit.material_layout,
            &[
                uniform.as_entire_binding(),
                wgpu::BindingResource::TextureView(fibre),
                wgpu::BindingResource::Sampler(&self.sampler),
            ],
        );
        self.materials.push(bound);
        MaterialId(self.materials.len() - 1)
    }

    /// Renders `scene` into the image and waits for the GPU to finish it.
    pub fn render(&mut self, scene: &Scene) -> Result<FrameStats, GpuError> {
        let start = Instant::now();
        let mut instances: Vec<Instance> = Vec::new();
        let mut draws = Vec::new();
        for draw in &scene.draws {
            let mesh = self
                .meshes
                .get(draw.mesh.0)
                .and_then(Option::as_ref)
                .ok_or(GpuError::FreedMesh(draw.mesh))?;
            let material = self
                .materials
                .get(draw.material.0)
                .ok_or(GpuError::UnknownMaterial(draw.material))?;
            let first = u32::try_from(instances.len()).unwrap_or(u32::MAX);
            instances.extend_from_slice(&draw.instances);
            let last = u32::try_from(instances.len()).unwrap_or(u32::MAX);
            draws.push((mesh, material, first..last));
        }
        let frame = frame_uniform(scene);
        self.queue
            .write_buffer(&self.frame, 0, bytemuck::bytes_of(&frame));
        for ((buffer, _), cascade) in self.shadow.cascades.iter().zip(&frame.cascades) {
            self.queue
                .write_buffer(buffer, 0, bytemuck::bytes_of(cascade));
        }
        let instance_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("instances"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.passes(&mut encoder, &instance_buffer, &draws);
        let submitted = Instant::now();
        self.queue.submit([encoder.finish()]);
        self.wait()?;
        let triangles = draws
            .iter()
            .map(|(mesh, _, range)| (mesh.count / 3).saturating_mul(range.end - range.start))
            .fold(0, u32::saturating_add);
        Ok(FrameStats {
            cpu_ms: ms(start..submitted),
            gpu_ms: ms(submitted..Instant::now()),
            draws: u32::try_from(draws.len()).unwrap_or(u32::MAX),
            triangles,
        })
    }

    fn passes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        instances: &wgpu::Buffer,
        draws: &[Drawn],
    ) {
        let targets = &self.targets;
        let clear = wgpu::LoadOp::Clear(1.0);
        for (layer, (_, cascade)) in self.shadow.layers.iter().zip(&self.shadow.cascades) {
            let mut pass = begin(encoder, "shadow", None, Some((layer, clear)));
            pass.set_pipeline(&self.shadow.pipeline);
            pass.set_bind_group(0, cascade, &[]);
            geometry(&mut pass, instances, draws, None);
        }
        let depth = Some((&targets.depth, clear));
        let mut pass = begin(encoder, "prepass", Some(&targets.normal), depth);
        pass.set_pipeline(&self.prepass.pipeline);
        pass.set_bind_group(0, &self.frame_group, &[]);
        geometry(&mut pass, instances, draws, None);
        drop(pass);
        let mut pass = begin(encoder, "ao", Some(&targets.occlusion), None);
        pass.set_pipeline(&self.ao.pipeline);
        pass.set_bind_group(0, &self.frame_group, &[]);
        pass.set_bind_group(1, &targets.ao, &[]);
        pass.draw(0..3, 0..1);
        drop(pass);
        let depth = Some((&targets.depth, wgpu::LoadOp::Load));
        let mut pass = begin(encoder, "lit", Some(&targets.hdr), depth);
        pass.set_bind_group(0, &self.frame_group, &[]);
        pass.set_bind_group(2, &targets.light, &[]);
        pass.set_pipeline(&self.lit.pipeline);
        geometry(&mut pass, instances, draws, Some(1));
        pass.set_pipeline(&self.sky.pipeline);
        pass.draw(0..3, 0..1);
        drop(pass);
        let mut pass = begin(encoder, "tone", Some(&targets.output_view), None);
        pass.set_pipeline(&self.tone.pipeline);
        pass.set_bind_group(0, &targets.tone, &[]);
        pass.draw(0..3, 0..1);
    }

    /// The last rendered image: RGBA8 in sRGB, row by row from the top left.
    pub fn pixels(&mut self) -> Result<Vec<u8>, GpuError> {
        let size = extent(self.size);
        let row = size.width * 4;
        let padded = row.div_ceil(ROW_ALIGN) * ROW_ALIGN;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pixels"),
            size: u64::from(padded) * u64::from(size.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            self.targets.output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: None,
                },
            },
            size,
        );
        self.queue.submit([encoder.finish()]);
        let (sent, mapped) = std::sync::mpsc::channel();
        buffer.map_async(wgpu::MapMode::Read, .., move |result| {
            sent.send(result).ok();
        });
        self.wait()?;
        mapped
            .recv()
            .map_err(|e| GpuError::Readback(e.to_string()))?
            .map_err(|e| GpuError::Readback(e.to_string()))?;
        let view = buffer
            .get_mapped_range(..)
            .map_err(|e| GpuError::Readback(e.to_string()))?;
        let row = usize::try_from(row).unwrap_or(0);
        let padded = usize::try_from(padded).unwrap_or(0);
        Ok(view
            .chunks(padded)
            .flat_map(|line| line.iter().take(row).copied())
            .collect())
    }

    fn wait(&self) -> Result<(), GpuError> {
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map(|_| ())
            .map_err(|e| GpuError::Readback(e.to_string()))
    }
}
