use crate::{MaterialId, MeshId};
use glam::{Mat4, Vec3};

/// One placed copy of a mesh, with a colour it is multiplied by.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub place: Mat4,
    pub tint: [f32; 4],
}

/// Where the scene is seen from and how it is projected.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Eye {
    pub view: Mat4,
    pub proj: Mat4,
}

/// The sun: the direction towards it and its colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    pub towards: Vec3,
    pub colour: [f32; 3],
}

pub(crate) struct Draw {
    pub mesh: MeshId,
    pub material: MaterialId,
    pub instances: Vec<Instance>,
}

/// One frame's eye, light, sky, ambient and draws.
pub struct Scene {
    pub(crate) eye: Eye,
    pub(crate) light: Light,
    pub(crate) sky: [f32; 3],
    pub(crate) ambient: [f32; 3],
    pub(crate) draws: Vec<Draw>,
}

impl Scene {
    #[must_use]
    pub fn new(eye: Eye, light: Light, sky: [f32; 3], ambient: [f32; 3]) -> Self {
        Self {
            eye,
            light,
            sky,
            ambient,
            draws: Vec::new(),
        }
    }

    pub fn draw(&mut self, mesh: MeshId, material: MaterialId, at: &[Instance]) {
        self.draws.push(Draw {
            mesh,
            material,
            instances: at.to_vec(),
        });
    }
}
