#![forbid(unsafe_code)]
#![allow(
    clippy::missing_errors_doc,
    reason = "one doc line a public item; each error type names its failures"
)]

mod fibre;
mod mesh;
mod model;
mod noise;
mod recipe;
mod sky;
mod terrain;

pub use fibre::{Pixels, fibre_map};
pub use mesh::{CHUNK_METRES, ChunkData, ChunkKey, Lod, MeshData};
pub use model::{Model, Moves, Shape, SoftPart};
pub use recipe::{Biome, BiomeId, Climate, Felt, Hem, Map, RecipeError, Relief, River};
pub use sky::{Sky, SkyKey, SkyNow};
pub use terrain::{BiomeMix, Terrain};
