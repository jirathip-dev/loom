use std::collections::BTreeMap;

use glam::{Vec2, Vec3};
use loom_procgen::{Felt, MeshData, Model, Moves, Shape, SoftPart};

const SLACK: f32 = 1e-4;

fn part(name: &str, shape: Shape, at: Vec3, red: f32) -> SoftPart {
    SoftPart {
        name: name.to_owned(),
        shape,
        at,
        felt: Felt {
            colour: [red, 0.4, 0.2],
            ..Felt::default()
        },
        moves: Moves::default(),
    }
}

fn model() -> Model {
    Model {
        parts: vec![
            part(
                "body",
                Shape::Ball {
                    radius: Vec3::new(0.3, 0.4, 0.25),
                },
                Vec3::new(0.0, 0.4, 0.0),
                0.1,
            ),
            part(
                "arm",
                Shape::Roll {
                    radius: 0.06,
                    length: 0.3,
                },
                Vec3::new(0.35, 0.5, 0.0),
                0.2,
            ),
            part(
                "belly",
                Shape::Patch {
                    size: Vec2::new(0.2, 0.25),
                },
                Vec3::new(0.0, 0.35, 0.26),
                0.3,
            ),
        ],
    }
}

fn half_extent(shape: Shape) -> Vec3 {
    match shape {
        Shape::Ball { radius } => radius,
        Shape::Roll { radius, length } => Vec3::new(length / 2.0 + radius, radius, radius),
        Shape::Patch { size } => (size / 2.0).extend(0.005),
    }
}

fn triangles(mesh: &MeshData) -> Vec<[Vec3; 3]> {
    let at = |i: u32| mesh.at.get(usize::try_from(i).ok()?).copied();
    mesh.index
        .as_chunks::<3>()
        .0
        .iter()
        .filter_map(|&[a, b, c]| Some([at(a)?, at(b)?, at(c)?]))
        .collect()
}

#[test]
fn the_same_model_gives_the_same_vertices_twice() {
    let first = model().meshes();
    let again = model().meshes();
    assert_eq!(first, again);
    let bits = |meshes: &[(MeshData, Felt)]| -> Vec<u32> {
        meshes
            .iter()
            .flat_map(|(mesh, _)| mesh.at.iter().chain(&mesh.normal))
            .flat_map(|v| v.to_array().map(f32::to_bits))
            .collect()
    };
    assert_eq!(bits(&first), bits(&again));
}

#[test]
fn every_edge_of_every_part_is_shared_by_two_triangles_wound_opposite_ways() {
    for (mesh, _) in model().meshes() {
        assert_ne!(mesh.index.len(), 0);
        assert!(mesh.index.len().is_multiple_of(3));
        let mut edges: BTreeMap<(u32, u32), usize> = BTreeMap::new();
        assert_eq!(triangles(&mesh).len() * 3, mesh.index.len());
        for &[x, y, z] in mesh.index.as_chunks::<3>().0 {
            for (a, b) in [(x, y), (y, z), (z, x)] {
                assert_ne!(a, b);
                *edges.entry((a, b)).or_default() += 1;
            }
        }
        for (&(a, b), &count) in &edges {
            assert_eq!(count, 1, "edge {a}-{b} is used {count} times one way");
            assert_eq!(edges.get(&(b, a)), Some(&1), "edge {a}-{b} has no twin");
        }
    }
}

#[test]
fn normals_and_faces_point_outward() {
    let model = model();
    for ((mesh, _), part) in model.meshes().iter().zip(&model.parts) {
        assert_eq!(mesh.normal.len(), mesh.at.len());
        for (at, normal) in mesh.at.iter().zip(&mesh.normal) {
            assert!((normal.length() - 1.0).abs() < SLACK);
            assert!(normal.dot(*at - part.at) > 0.0, "{} normal", part.name);
        }
        for [a, b, c] in triangles(mesh) {
            let face = (b - a).cross(c - a);
            assert!(face.length() > 0.0, "{} has a flat triangle", part.name);
            let middle = (a + b + c) / 3.0;
            assert!(face.dot(middle - part.at) > 0.0, "{} face", part.name);
        }
    }
}

#[test]
fn each_part_lies_inside_its_bounds() {
    let model = model();
    for ((mesh, _), part) in model.meshes().iter().zip(&model.parts) {
        let half = half_extent(part.shape);
        for at in &mesh.at {
            let off = (*at - part.at).abs();
            assert!(off.cmple(half + SLACK).all(), "{} at {at}", part.name);
        }
        let (low, high) = mesh.at.iter().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(low, high), at| (low.min(*at), high.max(*at)),
        );
        assert!(
            (high - low).cmpgt(half).all(),
            "{} fills its bounds",
            part.name
        );
    }
}

#[test]
fn each_part_has_its_own_mesh_that_moves_with_it_alone() {
    let before = model();
    let meshes = before.meshes();
    assert_eq!(meshes.len(), before.parts.len());
    for ((_, felt), part) in meshes.iter().zip(&before.parts) {
        assert_eq!(*felt, part.felt);
    }
    let step = Vec3::new(0.1, -0.2, 0.3);
    let mut after = before.clone();
    if let Some(arm) = after.parts.get_mut(1) {
        arm.at += step;
    }
    for (k, ((old, _), (new, _))) in meshes.iter().zip(&after.meshes()).enumerate() {
        assert_eq!(old.index, new.index);
        assert_eq!(old.normal, new.normal);
        let shift = if k == 1 { step } else { Vec3::ZERO };
        for (a, b) in old.at.iter().zip(&new.at) {
            assert!((*a + shift - *b).length() < SLACK, "part {k}");
        }
    }
}
