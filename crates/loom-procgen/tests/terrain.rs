use std::collections::BTreeMap;
use std::io::Write;
use std::process::Command;

use glam::{Vec2, Vec3};
use loom_core::{Seed, StateHash};
use loom_procgen::{
    Biome, BiomeId, CHUNK_METRES, ChunkKey, Climate, Felt, Hem, Lod, Map, RecipeError, Relief,
    River, Terrain,
};

const SEED: Seed = Seed(0x5EED);
const CHILD: &str = "LOOM_PROCGEN_GRID_CHILD";

fn biome(height: f32, moisture: f32) -> Biome {
    Biome {
        at: Climate { height, moisture },
        felt: Felt {
            colour: [height, moisture, 0.5],
            ..Felt::default()
        },
    }
}

fn recipe() -> (Map, BTreeMap<BiomeId, Biome>) {
    let biomes = BTreeMap::from([
        (BiomeId(1), biome(0.3, 0.3)),
        (BiomeId(2), biome(0.3, 0.7)),
        (BiomeId(3), biome(0.7, 0.3)),
        (BiomeId(4), biome(0.7, 0.7)),
    ]);
    let map = Map {
        size: (1024, 1024),
        relief: Relief {
            height: 60.0,
            span: 256.0,
            octaves: 4,
        },
        river: River {
            width: 16.0,
            depth: 3.0,
            bend: 120.0,
        },
        hem: Hem {
            width: 48.0,
            rise: 40.0,
        },
        moisture_span: 300.0,
        biomes: biomes.keys().copied().collect(),
    };
    (map, biomes)
}

fn terrain() -> Result<Terrain, RecipeError> {
    let (map, biomes) = recipe();
    Terrain::new(SEED, &map, &biomes)
}

fn grid(every: u16) -> impl Iterator<Item = Vec2> {
    let steps = 1024 / every;
    (0..=steps).flat_map(move |j| {
        (0..=steps).map(move |i| Vec2::new(f32::from(i * every), f32::from(j * every)))
    })
}

fn grid_hash(terrain: &Terrain) -> StateHash {
    let bytes: Vec<u8> = grid(4)
        .flat_map(|at| terrain.height(at).to_bits().to_le_bytes())
        .collect();
    StateHash::of(&bytes)
}

#[test]
fn grid_hash_child() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os(CHILD).is_some() {
        writeln!(std::io::stdout(), "grid {}", grid_hash(&terrain()?).0)?;
    }
    Ok(())
}

#[test]
fn the_same_seed_and_recipe_give_the_same_heights_in_two_processes()
-> Result<(), Box<dyn std::error::Error>> {
    let out = Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "grid_hash_child",
            "--nocapture",
            "--test-threads",
            "1",
        ])
        .env(CHILD, "1")
        .output()?;
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout)?;
    let theirs = text
        .split_whitespace()
        .skip_while(|word| *word != "grid")
        .nth(1)
        .ok_or("the child printed no grid hash")?;
    assert_eq!(theirs, grid_hash(&terrain()?).0.to_string());

    let (map, biomes) = recipe();
    let other = Terrain::new(Seed(SEED.0 + 1), &map, &biomes)?;
    assert_ne!(grid_hash(&other), grid_hash(&terrain()?));
    Ok(())
}

#[test]
fn every_listed_biome_covers_at_least_a_twentieth_of_the_land() -> Result<(), RecipeError> {
    let terrain = terrain()?;
    let mut count: BTreeMap<BiomeId, u32> = BTreeMap::new();
    let mut land = 0;
    for at in grid(8).filter(|at| terrain.dry(*at)) {
        *count.entry(terrain.biome(at).main).or_default() += 1;
        land += 1;
    }
    for id in recipe().0.biomes {
        let share = count.get(&id).copied().unwrap_or(0);
        assert!(share * 20 >= land, "{id:?} covers {share} of {land}");
    }
    Ok(())
}

fn water_runs(terrain: &Terrain, edge: impl Fn(f32) -> Vec2) -> usize {
    let wet: Vec<bool> = (0..=1024u16)
        .map(|k| terrain.water(edge(f32::from(k))).is_some())
        .collect();
    wet.windows(2)
        .filter(|pair| matches!(pair, [false, true]))
        .count()
        + usize::from(wet.first() == Some(&true))
}

#[test]
fn one_river_crosses_the_map_from_one_edge_to_the_opposite() -> Result<(), RecipeError> {
    let terrain = terrain()?;
    assert_eq!(water_runs(&terrain, |x| Vec2::new(x, 0.0)), 1);
    assert_eq!(water_runs(&terrain, |x| Vec2::new(x, 1024.0)), 1);
    assert_eq!(water_runs(&terrain, |z| Vec2::new(0.0, z)), 0);
    assert_eq!(water_runs(&terrain, |z| Vec2::new(1024.0, z)), 0);
    for z in (0..=1024u16).step_by(16) {
        assert_eq!(water_runs(&terrain, |x| Vec2::new(x, f32::from(z))), 1);
    }
    Ok(())
}

#[test]
fn the_hem_rises_at_all_four_edges() -> Result<(), RecipeError> {
    let terrain = terrain()?;
    let edges: [&dyn Fn(f32) -> Vec2; 4] = [
        &|k| Vec2::new(k, 0.0),
        &|k| Vec2::new(k, 1024.0),
        &|k| Vec2::new(0.0, k),
        &|k| Vec2::new(1024.0, k),
    ];
    for edge in edges {
        let line: Vec<Vec2> = (0..=1024u16).map(|k| edge(f32::from(k))).collect();
        let away_from_the_river = line
            .windows(49)
            .filter(|window| window.iter().all(|at| terrain.dry(*at)));
        let dry: Vec<f32> = away_from_the_river
            .filter_map(|window| window.get(24))
            .map(|at| terrain.height(*at))
            .collect();
        assert!(dry.len() > 900);
        assert!(dry.iter().all(|height| *height >= 40.0));
    }
    Ok(())
}

#[test]
fn the_hills_use_the_recipes_height_range() -> Result<(), RecipeError> {
    let terrain = terrain()?;
    let inner: Vec<f32> = grid(8)
        .filter(|at| at.min(Vec2::splat(1024.0) - *at).min_element() > 48.0 && terrain.dry(*at))
        .map(|at| terrain.height(at))
        .collect();
    let low = inner.iter().copied().fold(f32::INFINITY, f32::min);
    let high = inner.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    assert!(low >= 0.0 && high <= 60.0, "{low} to {high}");
    assert!(high - low >= 40.0, "{low} to {high}");
    Ok(())
}

fn border(terrain: &Terrain, key: ChunkKey, x: f32) -> Vec<Vec3> {
    let ground = terrain.chunk(key).ground;
    let step = 64 / key.lod.step() + 1;
    let top = usize::try_from(step * step).unwrap_or(0);
    let mut edge: Vec<Vec3> = ground
        .at
        .iter()
        .take(top)
        .copied()
        .filter(|at| (at.x - x).abs() < f32::EPSILON)
        .collect();
    edge.sort_by(|a, b| a.z.total_cmp(&b.z));
    edge
}

#[test]
fn a_chunk_has_the_same_border_heights_as_its_neighbour_at_each_level() -> Result<(), RecipeError> {
    let terrain = terrain()?;
    let seam = f32::from(u16::try_from(CHUNK_METRES).unwrap_or(0)) * 8.0;
    for lod in [Lod::Near, Lod::Mid, Lod::Far] {
        let west = border(&terrain, ChunkKey { x: 7, z: 3, lod }, seam);
        let east = border(&terrain, ChunkKey { x: 8, z: 3, lod }, seam);
        assert_eq!(
            west.len(),
            usize::try_from(64 / lod.step() + 1).unwrap_or(0)
        );
        assert_eq!(west, east);
    }
    Ok(())
}

#[test]
fn a_chunk_carries_a_skirt_below_its_border_and_water_where_the_river_runs()
-> Result<(), RecipeError> {
    let terrain = terrain()?;
    let chunk = terrain.chunk(ChunkKey {
        x: 2,
        z: 2,
        lod: Lod::Mid,
    });
    let grid_len = 33 * 33;
    assert_eq!(chunk.ground.at.len(), grid_len + 4 * 32);
    assert_eq!(chunk.ground.index.len(), 32 * 32 * 6 + 4 * 32 * 6);
    let skirt_lowest = chunk.ground.at.iter().skip(grid_len).map(|at| at.y);
    let ground_lowest = chunk.ground.at.iter().take(grid_len).map(|at| at.y);
    assert!(
        skirt_lowest.fold(f32::INFINITY, f32::min) < ground_lowest.fold(f32::INFINITY, f32::min)
    );
    assert!(chunk.ground.normal.iter().take(grid_len).all(|n| n.y > 0.0));

    let wet = (0..16).any(|x| {
        terrain
            .chunk(ChunkKey {
                x,
                z: 0,
                lod: Lod::Far,
            })
            .water
            .is_some()
    });
    assert!(wet);
    assert!(
        terrain
            .chunk(ChunkKey {
                x: 0,
                z: 0,
                lod: Lod::Far
            })
            .water
            .is_none()
    );
    Ok(())
}

#[test]
fn a_flat_terrain_is_level_dry_and_one_biome() {
    let flat = Terrain::flat((256, 128));
    assert_eq!(flat.size(), Vec2::new(256.0, 128.0));
    for at in [Vec2::ZERO, Vec2::new(100.0, 64.0), Vec2::new(256.0, 128.0)] {
        assert!(flat.height(at).abs() < f32::EPSILON);
        assert_eq!(flat.water(at), None);
        assert!(flat.dry(at));
        assert_eq!(flat.biome(at).main, BiomeId::default());
    }
    assert!(!flat.inside(Vec2::new(257.0, 0.0)));
    assert!(flat.dry_line(Vec2::ZERO, Vec2::new(256.0, 128.0)));
}

#[test]
fn a_recipe_that_lists_an_unknown_biome_is_refused() {
    let (mut map, biomes) = recipe();
    map.biomes.push(BiomeId(9));
    assert_eq!(
        Terrain::new(SEED, &map, &biomes).err(),
        Some(RecipeError::UnknownBiome(BiomeId(9)))
    );
}

#[test]
fn a_dry_line_across_the_river_is_not_dry() -> Result<(), RecipeError> {
    let terrain = terrain()?;
    assert!(!terrain.dry_line(Vec2::new(60.0, 512.0), Vec2::new(964.0, 512.0)));
    Ok(())
}
