use bevy_ecs::component::Component;
use bevy_ecs::resource::Resource;
use serde::{Deserialize, Serialize};

#[derive(
    Resource, Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
/// One step of sim time; the world's current tick is a resource.
pub struct Tick(pub u64);

/// The run's seed, from which every random stream is derived.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seed(pub u64);

#[derive(
    Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
/// A sim entity's stable id, kept across save and load.
pub struct SimId(pub u64);

/// The counter that hands out `SimId`s; part of the saved state.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextId(pub SimId);

/// An XXH3-128 hash of canonical state bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StateHash(pub u128);

impl Tick {
    /// The tick after this one.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl NextId {
    /// Hands out the next id and advances the counter.
    pub fn issue(&mut self) -> SimId {
        let id = self.0;
        self.0 = SimId(id.0 + 1);
        id
    }
}

impl StateHash {
    /// The hash of these bytes.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(xxhash_rust::xxh3::xxh3_128(bytes))
    }
}
