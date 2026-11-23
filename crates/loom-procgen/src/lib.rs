#![forbid(unsafe_code)]
#![allow(
    clippy::missing_errors_doc,
    reason = "one doc line a public item; each error type names its failures"
)]

mod mesh;
mod model;
mod noise;
mod recipe;
mod terrain;

pub use mesh::{CHUNK_METRES, ChunkData, ChunkKey, Lod, MeshData};
pub use model::{Model, Moves, Shape, SoftPart};
pub use recipe::{Biome, BiomeId, Climate, Felt, Hem, Map, RecipeError, Relief, River};
pub use terrain::{BiomeMix, Terrain};
