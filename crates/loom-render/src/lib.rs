#![deny(unsafe_code)]

mod gpu;
mod scene;

mod passes {
    pub mod lit;
    pub mod sky;
    pub mod tone;
}

pub use glam::{Mat4, Vec3};
pub use gpu::Gpu;
pub use scene::{Eye, Instance, Light, Scene};

/// One corner of a mesh: the single vertex layout every mesh uses.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub at: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub colour: [u8; 4],
}

/// The one material model: a base colour, a cloth sheen and a fibre texture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    pub colour: [f32; 3],
    pub sheen: [f32; 3],
    pub roughness: f32,
    pub fuzz: f32,
    pub fibre: TextureId,
}

/// A mesh uploaded to one `Gpu`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MeshId(usize);

/// A texture uploaded to one `Gpu`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(usize);

/// A material made on one `Gpu`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialId(usize);

/// What one frame cost.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameStats {
    pub cpu_ms: f32,
    pub gpu_ms: f32,
    pub draws: u32,
    pub triangles: u32,
}

/// Why the GPU could not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuError {
    NoAdapter(String),
    NoDevice(String),
    FreedMesh(MeshId),
    UnknownMaterial(MaterialId),
    Readback(String),
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAdapter(why) => write!(f, "no GPU adapter: {why}"),
            Self::NoDevice(why) => write!(f, "no GPU device: {why}"),
            Self::FreedMesh(mesh) => write!(f, "{mesh:?} was freed"),
            Self::UnknownMaterial(material) => write!(f, "{material:?} is not on this GPU"),
            Self::Readback(why) => write!(f, "reading the frame back failed: {why}"),
        }
    }
}

impl std::error::Error for GpuError {}
