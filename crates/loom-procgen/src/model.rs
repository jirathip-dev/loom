use std::f32::consts::{PI, TAU};

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::mesh::MeshData;
use crate::recipe::Felt;

const BANDS: u16 = 9;
const SEGMENTS: u16 = 16;
const FELT_HALF_THICKNESS: f32 = 0.005;
const WHITE: [u8; 4] = [255; 4];

/// A model's recipe: a handful of soft parts joined together, with no skeleton.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub parts: Vec<SoftPart>,
}

/// One soft part: its shape, centred at `at` in the model's space, its cloth and how it moves.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoftPart {
    pub name: String,
    pub shape: Shape,
    pub at: Vec3,
    pub felt: Felt,
    pub moves: Moves,
}

/// A ball with three radii, a roll along x capped by half balls, or a patch of felt facing z.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Shape {
    Ball { radius: Vec3 },
    Roll { radius: f32, length: f32 },
    Patch { size: Vec2 },
}

/// How much each procedural motion moves the part.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Moves {
    pub hop: f32,
    pub squash: f32,
    pub wobble: f32,
}

impl Model {
    /// One closed mesh a part, in the parts' order, each with its part's felt.
    #[must_use]
    pub fn meshes(&self) -> Vec<(MeshData, Felt)> {
        self.parts
            .iter()
            .map(|part| (part.mesh(), part.felt))
            .collect()
    }
}

impl SoftPart {
    fn mesh(&self) -> MeshData {
        match self.shape {
            Shape::Ball { radius } => hull(self.at, radius, 0.0),
            Shape::Roll { radius, length } => hull(self.at, Vec3::splat(radius), length / 2.0),
            Shape::Patch { size } => hull(self.at, (size / 2.0).extend(FELT_HALF_THICKNESS), 0.0),
        }
    }
}

fn hull(centre: Vec3, radius: Vec3, stretch: f32) -> MeshData {
    let mut mesh = MeshData::default();
    let mut point = |dir: Vec3| {
        let local = radius * dir + Vec3::X * stretch * dir.x.signum();
        mesh.push(
            centre + local,
            (dir / radius).normalize_or(dir),
            Vec2::new(local.x + local.z, local.y),
            WHITE,
        )
    };
    let first = point(Vec3::X);
    let rings: Vec<Vec<u32>> = (1..BANDS)
        .map(|band| {
            let theta = PI * f32::from(band) / f32::from(BANDS);
            (0..SEGMENTS)
                .map(|segment| {
                    let phi = TAU * f32::from(segment) / f32::from(SEGMENTS);
                    let ring = libm::sinf(theta);
                    point(Vec3::new(
                        libm::cosf(theta),
                        ring * libm::cosf(phi),
                        ring * libm::sinf(phi),
                    ))
                })
                .collect()
        })
        .collect();
    let last = point(Vec3::NEG_X);
    if let (Some(top), Some(bottom)) = (rings.first(), rings.last()) {
        for (a, b) in around(top) {
            mesh.index.extend([first, a, b]);
        }
        for (a, b) in around(bottom) {
            mesh.index.extend([a, last, b]);
        }
    }
    for pair in rings.windows(2) {
        if let [upper, lower] = pair {
            for ((a, b), (c, d)) in around(upper).zip(around(lower)) {
                mesh.quad(a, b, c, d);
            }
        }
    }
    mesh
}

fn around(ring: &[u32]) -> impl Iterator<Item = (u32, u32)> + '_ {
    ring.iter()
        .copied()
        .zip(ring.iter().copied().cycle().skip(1))
}
