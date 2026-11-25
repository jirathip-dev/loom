use glam::{Vec2, Vec3};
use loom_core::Seed;

use crate::mesh::byte;
use crate::noise::Noise;
use crate::terrain::metres;

const FIBRE_LAYER: u64 = 0xF1B2;
const COARSE_CELLS: f32 = 8.0;
const FINE_CELLS: f32 = 32.0;
const RELIEF: f32 = 0.04;

/// An RGBA8 image, row by row from the top left.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pixels {
    pub size: (u32, u32),
    pub rgba: Vec<u8>,
}

/// One tileable felt texture `size` pixels square: rgb a tangent-space normal, alpha the fibre's shade.
#[must_use]
pub fn fibre_map(seed: Seed, size: u32) -> Pixels {
    let coarse = Noise::new(seed, FIBRE_LAYER);
    let fine = Noise::new(seed, FIBRE_LAYER + 1);
    let span = metres(size);
    let cells = || (0..size).flat_map(move |y| (0..size).map(move |x| (x, y)));
    let density: Vec<f32> = cells()
        .map(|(x, y)| {
            let uv = Vec2::new(metres(x), metres(y)) / span;
            0.6 * coarse.tiled_fbm(uv * COARSE_CELLS, 4, COARSE_CELLS)
                + 0.4 * fine.tiled_fbm(uv * FINE_CELLS, 2, FINE_CELLS)
        })
        .collect();
    let at = |x: u32, y: u32| {
        let (x, y) = (x % size, y % size);
        let index = usize::try_from(u64::from(y) * u64::from(size) + u64::from(x));
        index
            .ok()
            .and_then(|i| density.get(i))
            .copied()
            .unwrap_or(0.0)
    };
    let before = |v: u32| v.checked_sub(1).unwrap_or(size - 1);
    let rgba = cells()
        .flat_map(|(x, y)| {
            let slope_x = (at(x + 1, y) - at(before(x), y)) * span / 2.0;
            let slope_y = (at(x, y + 1) - at(x, before(y))) * span / 2.0;
            let normal = Vec3::new(-slope_x * RELIEF, -slope_y * RELIEF, 1.0).normalize();
            let rgb = normal * 0.5 + 0.5;
            [
                byte(rgb.x),
                byte(rgb.y),
                byte(rgb.z),
                byte(0.75 + 0.25 * at(x, y)),
            ]
        })
        .collect();
    Pixels {
        size: (size, size),
        rgba,
    }
}
