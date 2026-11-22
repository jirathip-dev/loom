use std::collections::BTreeMap;

use bevy_ecs::resource::Resource;
use rand_pcg::Pcg64;
use rand_pcg::rand_core::Rng;
use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Seed;

/// The name a system draws its random numbers under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StreamName(pub &'static str);

/// Every random stream of a run, one per name, created from the seed on first use.
#[derive(Resource, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Streams {
    seed: Seed,
    streams: BTreeMap<String, Stream>,
}

/// One PCG stream; sim code draws through these methods only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stream(Pcg64);

impl Serialize for Stream {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::to_string(&self.0)
            .map_err(S::Error::custom)?
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Stream {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        serde_json::from_str(&text)
            .map(Self)
            .map_err(D::Error::custom)
    }
}

impl Streams {
    /// No streams yet; each is made from `seed` and its name when first asked for.
    #[must_use]
    pub const fn new(seed: Seed) -> Self {
        Self {
            seed,
            streams: BTreeMap::new(),
        }
    }

    /// The stream of `name`, created on first use.
    pub fn of(&mut self, name: StreamName) -> &mut Stream {
        let seed = self.seed;
        self.streams
            .entry(name.0.to_owned())
            .or_insert_with(|| Stream::new(seed, name))
    }
}

impl Stream {
    fn new(seed: Seed, name: StreamName) -> Self {
        let selector = xxhash_rust::xxh3::xxh3_128(name.0.as_bytes());
        Self(Pcg64::new(u128::from(seed.0), selector))
    }

    /// The next 64 random bits.
    pub fn bits(&mut self) -> u64 {
        self.0.next_u64()
    }

    /// A uniform number in `0..n`, without modulo bias; `0` when `n` is `0`.
    pub fn below(&mut self, n: u32) -> u32 {
        let n = u64::from(n);
        let Some(floor) = n.wrapping_neg().checked_rem(n) else {
            return 0;
        };
        loop {
            let x = self.bits();
            if x >= floor {
                return u32::try_from(x % n).unwrap_or(0);
            }
        }
    }

    /// A uniform number in `[0, 1)` with 23 bits of precision.
    pub fn unit(&mut self) -> f32 {
        let mantissa = u32::try_from(self.bits() >> 41).unwrap_or(0);
        f32::from_bits(0x3F80_0000 | mantissa) - 1.0
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f32) -> bool {
        self.unit() < p
    }
}
