use loom_core::Seed;
use loom_procgen::{Sky, fibre_map};

fn blue_share(colour: [f32; 3]) -> f32 {
    colour[2] / colour.iter().sum::<f32>()
}

#[test]
fn the_light_is_highest_at_noon_and_never_below_the_horizon() {
    let sky = Sky::default();
    let noon = sky.at(12.0).towards;
    for quarter in 0..=96u8 {
        let time = f32::from(quarter) / 4.0;
        let towards = sky.at(time).towards;
        assert!(towards.y > 0.0, "at {time} the light is below the horizon");
        assert!(
            (towards.length() - 1.0).abs() < 1e-5,
            "at {time}: {towards}"
        );
        if quarter != 48 {
            assert!(
                towards.y < noon.y,
                "at {time} the light is higher than at noon"
            );
        }
    }
    assert_eq!(sky.at(36.0), sky.at(12.0));
}

#[test]
fn the_night_key_is_dimmer_and_bluer_than_the_noon_key() {
    let sky = Sky::default();
    let (night, noon) = (sky.at(0.0), sky.at(12.0));
    let sum = |c: [f32; 3]| c.iter().sum::<f32>();
    assert!(sum(night.light) < sum(noon.light) / 4.0);
    assert!(blue_share(night.light) > blue_share(noon.light));
}

#[test]
fn between_two_keys_the_colours_blend() {
    let sky = Sky::default();
    let (dawn, noon, mid) = (sky.at(6.0), sky.at(12.0), sky.at(9.0));
    for channel in 0..3 {
        let half = f32::midpoint(dawn.light[channel], noon.light[channel]);
        assert!((mid.light[channel] - half).abs() < 1e-5, "{mid:?}");
    }
}

#[test]
fn the_fibre_map_is_the_same_bytes_for_the_same_recipe() {
    let first = fibre_map(Seed(7), 64);
    assert_eq!(first.size, (64, 64));
    assert_eq!(first.rgba.len(), 64 * 64 * 4);
    assert_eq!(first, fibre_map(Seed(7), 64));
    assert_ne!(first.rgba, fibre_map(Seed(8), 64).rgba);
}

#[test]
fn the_fibre_map_tiles_and_its_normals_face_out() {
    let map = fibre_map(Seed(3), 32);
    let pixel = |x: usize, y: usize| &map.rgba[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4];
    let step = |a: &[u8], b: &[u8]| a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
    let inner = (0..32)
        .map(|y| step(pixel(15, y), pixel(16, y)))
        .max()
        .unwrap();
    let seam = (0..32)
        .map(|y| step(pixel(31, y), pixel(0, y)))
        .max()
        .unwrap();
    assert!(
        seam <= inner.saturating_mul(2).max(8),
        "seam {seam}, inside {inner}"
    );
    assert!(
        map.rgba.chunks(4).all(|p| p[2] > 128),
        "every normal faces out"
    );
}
