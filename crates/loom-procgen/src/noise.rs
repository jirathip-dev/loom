use glam::Vec2;
use loom_core::Seed;

#[derive(Clone, Copy, Debug)]
pub struct Noise {
    seed: u64,
    layer: u64,
}

impl Noise {
    pub const fn new(seed: Seed, layer: u64) -> Self {
        Self {
            seed: seed.0,
            layer,
        }
    }

    pub fn unit(self) -> f32 {
        self.lattice(0.0, 0.0)
    }

    fn lattice(self, x: f32, z: f32) -> f32 {
        let mut h = mix(self.seed ^ mix(self.layer));
        h = mix(h ^ u64::from((x + 0.0).to_bits()));
        h = mix(h ^ u64::from((z + 0.0).to_bits()).rotate_left(32));
        f32::from(u16::try_from(h >> 48).unwrap_or(0)) / 65_536.0
    }

    fn value(self, p: Vec2) -> f32 {
        let x = libm::floorf(p.x);
        let z = libm::floorf(p.y);
        let (fx, fz) = (fade(p.x - x), fade(p.y - z));
        let top = lerp(self.lattice(x, z), self.lattice(x + 1.0, z), fx);
        let bottom = lerp(self.lattice(x, z + 1.0), self.lattice(x + 1.0, z + 1.0), fx);
        lerp(top, bottom, fz)
    }

    pub fn fbm(self, p: Vec2, octaves: u32) -> f32 {
        let (mut sum, mut total, mut amp, mut at) = (0.0, 0.0, 1.0, p);
        for octave in 0..octaves {
            let layer = Self {
                layer: self.layer ^ (u64::from(octave) << 40),
                ..self
            };
            sum += amp * layer.value(at);
            total += amp;
            amp *= 0.5;
            at *= 2.0;
        }
        ((sum / total - 0.5) * 2.2 + 0.5).clamp(0.0, 1.0)
    }
}

const fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn fade(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn smoothstep(from: f32, to: f32, x: f32) -> f32 {
    fade(((x - from) / (to - from)).clamp(0.0, 1.0))
}
