# loom-procgen

Recipes in, plain data out. No GPU, no threads, no files, no clock. A game's simulation calls it, so it runs
under the determinism lints (D1 to D6, `determinism.md`). Names below are decided; exact bounds and private
fields are the implementer's. A world unit is one metre; y is up.

## Terrain

```rust
pub struct Terrain { /* the seed, the recipe, the biomes; nothing that changes */ }

impl Terrain {
    pub fn new(seed: Seed, map: &Map, biomes: &BTreeMap<BiomeId, Biome>) -> Result<Self, RecipeError>;
    pub fn flat(size: (u32, u32)) -> Self;
    pub fn size(&self) -> Vec2;
    pub fn inside(&self, at: Vec2) -> bool;
    pub fn height(&self, at: Vec2) -> f32;
    pub fn water(&self, at: Vec2) -> Option<f32>;
    pub fn dry(&self, at: Vec2) -> bool;
    pub fn dry_line(&self, from: Vec2, to: Vec2) -> bool;
    pub fn biome(&self, at: Vec2) -> BiomeMix;
    pub fn chunk(&self, key: ChunkKey) -> ChunkData;
}

pub struct BiomeMix { pub main: BiomeId, pub other: BiomeId, pub blend: f32 }

pub const CHUNK_METRES: u32 = 64;
pub struct ChunkKey { pub x: i16, pub z: i16, pub lod: Lod }
pub enum Lod { Near, Mid, Far }
pub struct ChunkData { pub key: ChunkKey, pub ground: MeshData, pub water: Option<MeshData> }

pub struct MeshData {
    pub at: Vec<Vec3>,
    pub normal: Vec<Vec3>,
    pub uv: Vec<Vec2>,
    pub colour: Vec<[u8; 4]>,
    pub index: Vec<u32>,
}
pub struct Pixels { pub size: (u32, u32), pub rgba: Vec<u8> }
```

- `height` is layered noise from the seed, shaped by the recipe: relief, a carved river channel, and a rim
  that rises at every edge so the horizon is land, not void.
- `water` is the water's surface height where `at` is in the channel. `dry` is inside and not water.
- `biome` is the nearest two biomes in (height, moisture) space, so a new biome is new data and no code.
- `Lod`: samples 1 m, 2 m and 4 m apart. Each chunk mesh carries a skirt, so levels meet without a crack.
- `Terrain::flat` is height 0, one biome, no water.
- A `Terrain` is made once from the seed and the recipe and never saved: a load makes it again.

## Recipes

`Map`, `Biome`, `Model`, `Sky` and `fibre_map` are built.

Plain `serde` types; the caller reads them from wherever it keeps its data.

```rust
pub struct Map {
    pub size: (u32, u32),
    pub relief: Relief,               // height, span, octaves
    pub river: River,                 // width, depth, bend
    pub hem: Hem,                     // width, rise
    pub moisture_span: f32,
    pub biomes: Vec<BiomeId>,
}
pub struct Biome { pub at: Climate, pub felt: Felt }   // Climate: height and moisture, each 0 to 1
pub struct Felt { pub colour: [f32; 3], pub sheen: [f32; 3], pub roughness: f32, pub fuzz: f32 }

pub struct Model { pub parts: Vec<SoftPart> }
pub struct SoftPart { pub name: String, pub shape: Shape, pub at: Vec3, pub felt: Felt, pub moves: Moves }
pub enum Shape { Ball { radius: Vec3 }, Roll { radius: f32, length: f32 }, Patch { size: Vec2 } }
pub struct Moves { pub hop: f32, pub squash: f32, pub wobble: f32 }

pub struct Sky { pub keys: Vec<SkyKey> }
pub struct SkyKey { pub at: f32, pub light: [f32; 3], pub sky: [f32; 3], pub ambient: [f32; 3] }
pub struct SkyNow { pub towards: Vec3, pub light: [f32; 3], pub sky: [f32; 3], pub ambient: [f32; 3] }

impl Model { pub fn meshes(&self) -> Vec<(MeshData, Felt)>; }
impl Sky { pub fn at(&self, time: f32) -> SkyNow; }
pub fn fibre_map(seed: Seed, size: u32) -> Pixels;
```

- A model is a handful of soft parts joined together: balls, rolls and flat patches. No skeleton, no
  skinning. `Moves` says how much each procedural motion (hop, squash, wobble) moves that part.
- `Model::meshes` gives one closed mesh a part, in the parts' order, centred at the part's `at`, so a caller
  moves a part as a whole. A ball spans its three radii; a roll lies along x, `length` between the centres
  of its two half-ball ends; a patch is a 1 cm thick oval of felt `size` across in x and y, facing z.
- `Felt` is the cloth material's parameters, handed to the renderer as they are.
- `Sky::at` gives the one light's direction and colour, the sky and the ambient light for a time of day,
  blended between the keys. Time is in hours, 0 to 24, and wraps; a key's `at` is its hour. The light is
  highest at noon and never below the horizon: at night it is the moon. `Sky::default()` is a plain day of
  four keys, with a dim blue night.
- `fibre_map` is one tileable texture: fibre density and a normal map. Every material uses it at its own scale.

## Usage

```rust
let terrain = Terrain::new(seed, &map, &biomes)?;
if terrain.dry_line(from, to) { /* a step the simulation may take */ }

let chunk: ChunkData = terrain.chunk(ChunkKey { x: 7, z: 8, lod: Lod::Near });   // on a worker thread
```

The same object answers the simulation's questions and builds the view's meshes, so what is walked on is what
is drawn.
