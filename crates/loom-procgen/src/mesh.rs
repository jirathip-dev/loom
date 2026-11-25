use glam::{Vec2, Vec3};

use crate::terrain::{Terrain, metres};

/// A chunk's width and depth in metres.
pub const CHUNK_METRES: u32 = 64;

const WATER_COLOUR: [u8; 4] = [70, 120, 170, 200];

/// One chunk of terrain at one level of detail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkKey {
    pub x: i16,
    pub z: i16,
    pub lod: Lod,
}

/// A level of detail: samples 1 m, 2 m or 4 m apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lod {
    Near,
    Mid,
    Far,
}

/// A chunk's meshes: the ground with its skirt, and the water where the chunk has any.
#[derive(Clone, Debug, PartialEq)]
pub struct ChunkData {
    pub key: ChunkKey,
    pub ground: MeshData,
    pub water: Option<MeshData>,
}

/// An indexed triangle mesh, counter-clockwise seen from its front, ready for a GPU buffer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshData {
    pub at: Vec<Vec3>,
    pub normal: Vec<Vec3>,
    pub uv: Vec<Vec2>,
    pub colour: Vec<[u8; 4]>,
    pub index: Vec<u32>,
}

impl Lod {
    /// The distance between samples in metres.
    #[must_use]
    pub const fn step(self) -> u32 {
        match self {
            Self::Near => 1,
            Self::Mid => 2,
            Self::Far => 4,
        }
    }
}

impl MeshData {
    fn vertex(&mut self, at: Vec3, normal: Vec3, colour: [u8; 4]) -> u32 {
        let chunk = metres(CHUNK_METRES);
        self.push(at, normal, Vec2::new(at.x / chunk, at.z / chunk), colour)
    }

    pub(crate) fn push(&mut self, at: Vec3, normal: Vec3, uv: Vec2, colour: [u8; 4]) -> u32 {
        let index = u32::try_from(self.at.len()).unwrap_or(u32::MAX);
        self.at.push(at);
        self.normal.push(normal);
        self.uv.push(uv);
        self.colour.push(colour);
        index
    }

    pub(crate) fn quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.index.extend([a, c, b, b, c, d]);
    }
}

impl Terrain {
    /// The chunk's ground mesh, skirted so levels meet without a crack, and its water mesh.
    #[must_use]
    pub fn chunk(&self, key: ChunkKey) -> ChunkData {
        let step = metres(key.lod.step());
        let cells = CHUNK_METRES / key.lod.step();
        let corner = Vec2::new(f32::from(key.x), f32::from(key.z)) * metres(CHUNK_METRES);
        let point = |i: u32, j: u32| corner + Vec2::new(metres(i), metres(j)) * step;
        let mut ground = MeshData::default();
        let mut water = MeshData::default();
        for j in 0..=cells {
            for i in 0..=cells {
                let at = point(i, j);
                ground.vertex(
                    at3(at, self.height(at)),
                    self.normal(at, step),
                    self.colour(at),
                );
            }
        }
        let row = cells + 1;
        for j in 0..cells {
            for i in 0..cells {
                let a = j * row + i;
                ground.quad(a, a + 1, a + row, a + row + 1);
                let corners = [
                    point(i, j),
                    point(i + 1, j),
                    point(i, j + 1),
                    point(i + 1, j + 1),
                ];
                if let Some(level) = corners.iter().find_map(|at| self.water(*at)) {
                    let [a, b, c, d] =
                        corners.map(|at| water.vertex(at3(at, level), Vec3::Y, WATER_COLOUR));
                    water.quad(a, b, c, d);
                }
            }
        }
        skirt(&mut ground, cells, step * 4.0);
        ChunkData {
            key,
            ground,
            water: (!water.index.is_empty()).then_some(water),
        }
    }

    fn normal(&self, at: Vec2, step: f32) -> Vec3 {
        let dx = self.height(at + Vec2::X * step) - self.height(at - Vec2::X * step);
        let dz = self.height(at + Vec2::Y * step) - self.height(at - Vec2::Y * step);
        Vec3::new(-dx, 2.0 * step, -dz).normalize_or(Vec3::Y)
    }

    fn colour(&self, at: Vec2) -> [u8; 4] {
        let mix = self.biome(at);
        let main = Vec3::from(self.felt_colour(mix.main));
        let colour = main.lerp(Vec3::from(self.felt_colour(mix.other)), mix.blend);
        [byte(colour.x), byte(colour.y), byte(colour.z), 255]
    }
}

fn skirt(mesh: &mut MeshData, cells: u32, depth: f32) {
    let row = cells + 1;
    let ring: Vec<u32> = (0..cells)
        .chain((0..cells).map(|j| j * row + cells))
        .chain((0..cells).map(|i| cells * row + cells - i))
        .chain((0..cells).map(|j| (cells - j) * row))
        .collect();
    let lowered: Vec<u32> = ring
        .iter()
        .filter_map(|top| {
            let index = usize::try_from(*top).ok()?;
            let at = *mesh.at.get(index)? - Vec3::Y * depth;
            let normal = *mesh.normal.get(index)?;
            let colour = *mesh.colour.get(index)?;
            Some(mesh.vertex(at, normal, colour))
        })
        .collect();
    for (k, (top, low)) in ring.iter().zip(&lowered).enumerate() {
        let next = (k + 1) % ring.len();
        if let (Some(top_next), Some(low_next)) = (ring.get(next), lowered.get(next)) {
            mesh.index
                .extend([*top, *top_next, *low, *top_next, *low_next, *low]);
        }
    }
}

fn at3(at: Vec2, height: f32) -> Vec3 {
    Vec3::new(at.x, height, at.y)
}

#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0 to 255 first, so the cast only drops the fraction"
)]
pub(crate) fn byte(unit: f32) -> u8 {
    (unit.clamp(0.0, 1.0) * 255.0).round() as u8
}
