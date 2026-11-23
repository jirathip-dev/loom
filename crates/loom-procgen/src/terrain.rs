use std::collections::BTreeMap;

use glam::Vec2;
use loom_core::Seed;

use crate::noise::{Noise, lerp, smoothstep};
use crate::recipe::{Biome, BiomeId, Climate, Hem, Map, RecipeError, Relief, River};

const WATER: f32 = 0.0;

/// Seeded terrain made from a map's recipe: heights, water, biomes and chunk meshes; made again on load, never saved.
#[derive(Clone, Debug)]
pub struct Terrain {
    map: Map,
    size: Vec2,
    biomes: Vec<(BiomeId, Biome)>,
    relief: Noise,
    moisture: Noise,
    phase: f32,
}

/// The nearest two biomes at a point and how far it is towards the other, 0 to one half.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiomeMix {
    pub main: BiomeId,
    pub other: BiomeId,
    pub blend: f32,
}

impl Terrain {
    /// The terrain the seed and the recipe make.
    pub fn new(
        seed: Seed,
        map: &Map,
        biomes: &BTreeMap<BiomeId, Biome>,
    ) -> Result<Self, RecipeError> {
        map.check(biomes)?;
        let listed = map
            .biomes
            .iter()
            .filter_map(|id| biomes.get(id).map(|biome| (*id, *biome)))
            .collect();
        Ok(Self::made(seed, map.clone(), listed))
    }

    /// A flat terrain: height 0, one biome, no water.
    #[must_use]
    pub fn flat(size: (u32, u32)) -> Self {
        let map = Map {
            size,
            relief: Relief {
                height: 0.0,
                span: 1.0,
                octaves: 1,
            },
            river: River {
                width: 0.0,
                depth: 0.0,
                bend: 0.0,
            },
            hem: Hem {
                width: 0.0,
                rise: 0.0,
            },
            moisture_span: 1.0,
            biomes: vec![BiomeId::default()],
        };
        Self::made(Seed(0), map, vec![(BiomeId::default(), Biome::default())])
    }

    fn made(seed: Seed, map: Map, biomes: Vec<(BiomeId, Biome)>) -> Self {
        Self {
            size: Vec2::new(metres(map.size.0), metres(map.size.1)),
            map,
            biomes,
            relief: Noise::new(seed, 1),
            moisture: Noise::new(seed, 2),
            phase: Noise::new(seed, 3).unit() * core::f32::consts::TAU,
        }
    }

    /// The map's size in metres, x by z.
    #[must_use]
    pub const fn size(&self) -> Vec2 {
        self.size
    }

    /// Whether `at` is on the map.
    #[must_use]
    pub fn inside(&self, at: Vec2) -> bool {
        at.cmpge(Vec2::ZERO).all() && at.cmple(self.size).all()
    }

    /// The ground's height at `at`.
    #[must_use]
    pub fn height(&self, at: Vec2) -> f32 {
        self.shape(at).0
    }

    /// The water's surface height where `at` is in the river.
    #[must_use]
    pub fn water(&self, at: Vec2) -> Option<f32> {
        self.shape(at).1.filter(|_| self.inside(at))
    }

    /// Whether `at` is on the map and out of the water.
    #[must_use]
    pub fn dry(&self, at: Vec2) -> bool {
        self.inside(at) && self.shape(at).1.is_none()
    }

    /// Whether every point from `from` to `to`, half a metre apart, is dry.
    #[must_use]
    pub fn dry_line(&self, from: Vec2, to: Vec2) -> bool {
        let step = 0.5 / from.distance(to).max(0.5);
        let mut t = 0.0;
        while t < 1.0 {
            if !self.dry(from.lerp(to, t)) {
                return false;
            }
            t += step;
        }
        self.dry(to)
    }

    /// The nearest two biomes to the climate at `at`.
    #[must_use]
    pub fn biome(&self, at: Vec2) -> BiomeMix {
        let here = Vec2::new(self.lift(at), self.wet(at));
        let mut near: Vec<(f32, BiomeId)> = self
            .biomes
            .iter()
            .map(|(id, biome)| (here.distance(climate(biome.at)), *id))
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let (main, other) = match near.as_slice() {
            [main, other, ..] => (*main, *other),
            [main] => (*main, *main),
            [] => ((0.0, BiomeId::default()), (0.0, BiomeId::default())),
        };
        let total = main.0 + other.0;
        BiomeMix {
            main: main.1,
            other: other.1,
            blend: if total > 0.0 { main.0 / total } else { 0.0 },
        }
    }

    pub(crate) fn felt_colour(&self, id: BiomeId) -> [f32; 3] {
        self.biomes
            .iter()
            .find(|(each, _)| *each == id)
            .map_or([0.0; 3], |(_, biome)| biome.felt.colour)
    }

    fn lift(&self, at: Vec2) -> f32 {
        let relief = self.map.relief;
        self.relief.fbm(at / relief.span, relief.octaves)
    }

    fn wet(&self, at: Vec2) -> f32 {
        self.moisture.fbm(at / self.map.moisture_span, 3)
    }

    fn shape(&self, at: Vec2) -> (f32, Option<f32>) {
        let Map {
            relief, river, hem, ..
        } = self.map;
        let mut ground = relief.height * self.lift(at);
        if hem.width > 0.0 {
            let edge = at.min(self.size - at).min_element();
            ground += hem.rise * (1.0 - smoothstep(0.0, hem.width, edge));
        }
        if river.width <= 0.0 {
            return (ground, None);
        }
        let half = river.width / 2.0;
        let turn = at.y / self.size.y * core::f32::consts::TAU * 1.5 + self.phase;
        let off = (at.x - (self.size.x / 2.0 + river.bend * libm::sinf(turn))).abs();
        if off < half {
            let bed = 1.0 - (off / half) * (off / half);
            return (WATER - river.depth * bed, Some(WATER));
        }
        let bank = smoothstep(half, half + river.width, off);
        (lerp(WATER, ground, bank), None)
    }
}

fn climate(at: Climate) -> Vec2 {
    Vec2::new(at.height, at.moisture)
}

#[expect(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    reason = "a map is far under 2^24 m across, where every whole metre is exact in f32"
)]
pub(crate) const fn metres(whole: u32) -> f32 {
    whole as f32
}
