use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
/// A biome's id, as a map's recipe lists it.
pub struct BiomeId(pub u16);

/// The recipe for one map's terrain.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Map {
    pub size: (u32, u32),
    pub relief: Relief,
    pub river: River,
    pub hem: Hem,
    pub moisture_span: f32,
    pub biomes: Vec<BiomeId>,
}

/// The hills: their greatest height, the span of the broadest one and how many noise octaves shape them.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Relief {
    pub height: f32,
    pub span: f32,
    pub octaves: u32,
}

/// The one river, edge to edge: its width, its depth and how far it bends either side.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct River {
    pub width: f32,
    pub depth: f32,
    pub bend: f32,
}

/// The rim that rises at every edge: its width and how high it rises.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hem {
    pub width: f32,
    pub rise: f32,
}

/// A biome: where it grows in climate space and how its ground looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Biome {
    pub at: Climate,
    pub felt: Felt,
}

/// A point in climate space: height and moisture, each 0 to 1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Climate {
    pub height: f32,
    pub moisture: f32,
}

/// The cloth material's parameters, handed to the renderer as they are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Felt {
    pub colour: [f32; 3],
    pub sheen: [f32; 3],
    pub roughness: f32,
    pub fuzz: f32,
}

/// Why a recipe cannot make a terrain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecipeError {
    Bad(&'static str),
    NoBiomes,
    UnknownBiome(BiomeId),
}

impl fmt::Display for RecipeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bad(field) => write!(f, "the map's {field} is out of range"),
            Self::NoBiomes => write!(f, "the map lists no biome"),
            Self::UnknownBiome(id) => write!(f, "the map lists biome {}, which is not given", id.0),
        }
    }
}

impl std::error::Error for RecipeError {}

impl Map {
    pub(crate) fn check(&self, biomes: &BTreeMap<BiomeId, Biome>) -> Result<(), RecipeError> {
        let (Ok(x), Ok(z)) = (u16::try_from(self.size.0), u16::try_from(self.size.1)) else {
            return Err(RecipeError::Bad("size"));
        };
        let half = f32::from(x.min(z)) / 2.0;
        let rules = [
            (x > 0 && z > 0, "size"),
            (finite_at_least(self.relief.height, 0.0), "relief.height"),
            (
                finite_at_least(self.relief.span, f32::MIN_POSITIVE),
                "relief.span",
            ),
            ((1..=12).contains(&self.relief.octaves), "relief.octaves"),
            (finite_at_least(self.river.width, 0.0), "river.width"),
            (finite_at_least(self.river.depth, 0.0), "river.depth"),
            (
                self.river.bend.is_finite() && self.river.bend.abs() + self.river.width < half,
                "river.bend",
            ),
            (
                finite_at_least(self.hem.width, 0.0) && self.hem.width < half,
                "hem.width",
            ),
            (finite_at_least(self.hem.rise, 0.0), "hem.rise"),
            (
                finite_at_least(self.moisture_span, f32::MIN_POSITIVE),
                "moisture_span",
            ),
        ];
        if let Some((_, field)) = rules.iter().find(|(ok, _)| !ok) {
            return Err(RecipeError::Bad(field));
        }
        if self.biomes.is_empty() {
            return Err(RecipeError::NoBiomes);
        }
        match self.biomes.iter().find(|id| !biomes.contains_key(id)) {
            Some(id) => Err(RecipeError::UnknownBiome(*id)),
            None => Ok(()),
        }
    }
}

fn finite_at_least(value: f32, least: f32) -> bool {
    value.is_finite() && value >= least
}
