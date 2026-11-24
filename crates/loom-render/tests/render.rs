use glam::camera::rh::{proj::directx, view::look_at_mat4};
use loom_render::{Eye, Gpu, Instance, Light, Mat4, Material, Scene, Vec3, Vertex};
use std::f32::consts::PI;

const SIZE: (u32, u32) = (1920, 1080);
const SKY: [f32; 3] = [0.4, 0.6, 0.9];
const RADIUS: f32 = 1.0;
const EYE: Vec3 = Vec3::new(0.0, 1.0, 6.0);
const LEFT_BALL: Vec3 = Vec3::new(-1.3, 1.0, 0.0);
const RIGHT_BALL: Vec3 = Vec3::new(1.3, 1.0, 0.0);

fn gpu() -> Option<Gpu> {
    match Gpu::offscreen(SIZE) {
        Ok(gpu) => Some(gpu),
        Err(e) => {
            assert_ne!(
                std::env::var("LOOM_GPU").as_deref(),
                Ok("required"),
                "LOOM_GPU=required, but: {e}"
            );
            eprintln!("skipped: {e}; set LOOM_GPU=required to fail instead");
            None
        }
    }
}

fn vertex(at: Vec3, normal: Vec3, uv: [f32; 2]) -> Vertex {
    Vertex {
        at: at.to_array(),
        normal: normal.to_array(),
        uv,
        colour: [255; 4],
    }
}

fn ratio(n: u32, d: u32) -> f32 {
    f32::from(u16::try_from(n).unwrap()) / f32::from(u16::try_from(d).unwrap())
}

fn ball() -> (Vec<Vertex>, Vec<u32>) {
    let (stacks, slices) = (48u32, 96u32);
    let mut vertices = Vec::new();
    for i in 0..=stacks {
        for j in 0..=slices {
            let (u, v) = (ratio(j, slices), ratio(i, stacks));
            let (theta, phi) = (v * PI, u * 2.0 * PI);
            let n = Vec3::new(
                theta.sin() * phi.cos(),
                theta.cos(),
                theta.sin() * phi.sin(),
            );
            vertices.push(vertex(n * RADIUS, n, [u * 4.0, v * 2.0]));
        }
    }
    let mut indices = Vec::new();
    for i in 0..stacks {
        for j in 0..slices {
            let a = i * (slices + 1) + j;
            let (b, d) = (a + slices + 1, a + 1);
            let c = b + 1;
            indices.extend([a, d, c, a, c, b]);
        }
    }
    (vertices, indices)
}

fn plane() -> (Vec<Vertex>, Vec<u32>) {
    let corners = [(-10.0, -10.0), (10.0, -10.0), (10.0, 10.0), (-10.0, 10.0)];
    let vertices = corners
        .iter()
        .map(|&(x, z)| vertex(Vec3::new(x, 0.0, z), Vec3::Y, [x, z]))
        .collect();
    (vertices, vec![0, 3, 2, 0, 2, 1])
}

fn eye() -> Eye {
    Eye {
        view: look_at_mat4(EYE, Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        proj: directx::perspective(PI / 4.0, 16.0 / 9.0, 0.1, 100.0),
    }
}

fn render(gpu: &mut Gpu, sheen: [f32; 3]) -> Vec<u8> {
    let fibre = gpu.texture(&[128, 128, 255, 255], (1, 1));
    let (vertices, indices) = ball();
    let ball = gpu.mesh(&vertices, &indices);
    let (vertices, indices) = plane();
    let ground = gpu.mesh(&vertices, &indices);
    let cloth = gpu.material(&Material {
        colour: [0.6, 0.15, 0.1],
        sheen,
        roughness: 0.8,
        fuzz: 0.4,
        fibre,
    });
    let floor = gpu.material(&Material {
        colour: [0.4, 0.4, 0.35],
        sheen: [0.0; 3],
        roughness: 0.9,
        fuzz: 0.5,
        fibre,
    });
    let light = Light {
        towards: Vec3::new(-1.0, 1.0, 0.3),
        colour: [3.0; 3],
    };
    let mut scene = Scene::new(eye(), light, SKY, [0.1, 0.1, 0.12]);
    let at = |centre| Instance {
        place: Mat4::from_translation(centre),
        tint: [1.0; 4],
    };
    scene.draw(ball, cloth, &[at(LEFT_BALL), at(RIGHT_BALL)]);
    scene.draw(ground, floor, &[at(Vec3::ZERO)]);
    let stats = gpu.render(&scene).unwrap();
    assert_eq!((stats.draws, stats.triangles), (2, 2 * 48 * 96 * 2 + 2));
    gpu.pixels().unwrap()
}

fn brightness(image: &[u8], world: Vec3) -> u32 {
    let clip = eye().proj * eye().view * world.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    let x = index(f32::midpoint(ndc.x, 1.0) * ratio(SIZE.0, 1));
    let y = index(f32::midpoint(1.0, -ndc.y) * ratio(SIZE.1, 1));
    pixel(image, x, y)[..3].iter().map(|&c| u32::from(c)).sum()
}

fn index(coord: f32) -> usize {
    (1..=u16::MAX)
        .take_while(|&i| f32::from(i) <= coord)
        .count()
}

fn pixel(image: &[u8], x: usize, y: usize) -> &[u8] {
    let at = (y * usize::try_from(SIZE.0).unwrap() + x) * 4;
    &image[at..at + 4]
}

fn on_left_ball(normal: Vec3) -> Vec3 {
    LEFT_BALL + normal.normalize() * RADIUS
}

#[test]
fn the_sunward_side_is_brighter_than_the_far_side() {
    let Some(mut gpu) = gpu() else { return };
    let image = render(&mut gpu, [0.8; 3]);
    let sunward = brightness(&image, on_left_ball(Vec3::new(-0.6, 0.0, 0.8)));
    let far = brightness(&image, on_left_ball(Vec3::new(0.6, 0.0, 0.8)));
    assert!(sunward > far + 30, "sunward {sunward}, far {far}");
}

#[test]
fn sheen_brightens_the_rim_at_a_grazing_angle() {
    let Some(mut gpu) = gpu() else { return };
    let toward_eye = (EYE - LEFT_BALL).normalize();
    let side = toward_eye.cross(Vec3::Y).normalize();
    let rim = on_left_ball(side * 0.95 + toward_eye * 0.3);
    let plain = brightness(&render(&mut gpu, [0.0; 3]), rim);
    let sheen = brightness(&render(&mut gpu, [1.0; 3]), rim);
    assert!(sheen > plain + 30, "with sheen {sheen}, without {plain}");
}

#[test]
fn the_background_is_the_sky_colour() {
    let Some(mut gpu) = gpu() else { return };
    let image = render(&mut gpu, [0.8; 3]);
    let srgb = |linear: f32| {
        let encoded = if linear <= 0.003_130_8 {
            linear * 12.92
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        };
        encoded * 255.0
    };
    for (x, y) in [(10, 10), (960, 20), (1900, 60)] {
        let got = pixel(&image, x, y);
        for (channel, linear) in SKY.iter().enumerate() {
            let diff = (f32::from(got[channel]) - srgb(*linear)).abs();
            assert!(diff <= 1.5, "({x}, {y}) is {got:?}, sky is {SKY:?}");
        }
    }
}

#[test]
fn two_renders_of_one_scene_are_byte_identical() {
    let Some(mut gpu) = gpu() else { return };
    let first = render(&mut gpu, [0.8; 3]);
    let second = render(&mut gpu, [0.8; 3]);
    assert_eq!(first.len(), 1920 * 1080 * 4);
    assert!(first == second, "the two renders differ");
}
