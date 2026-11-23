use std::any::TypeId;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use bevy_ecs::component::{Component, ComponentInfo};
use bevy_ecs::entity::Entity;
use bevy_ecs::entity_disabling::DefaultQueryFilters;
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use xxhash_rust::xxh3::Xxh3;

use crate::{CanonicalError, NextId, SimId, StateHash, Streams, Tick, canonical};

/// The name a component or resource is saved and hashed under.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SectionName(pub String);

/// The whole saved state of a world: what save, dump and the hash all read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub tick: Tick,
    pub next_id: SimId,
    pub streams: Streams,
    pub resources: BTreeMap<SectionName, Value>,
    pub entities: BTreeMap<SimId, BTreeMap<SectionName, Value>>,
}

/// The state hash at a tick: the total and one hash per section.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashReport {
    pub tick: Tick,
    pub total: StateHash,
    pub sections: BTreeMap<SectionName, StateHash>,
}

/// Why a world and a `State` could not be turned into each other.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    Missing(&'static str),
    DuplicateId(SimId),
    Unknown(SectionName),
    Value {
        section: SectionName,
        message: String,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "the world has no {name} resource"),
            Self::DuplicateId(id) => write!(f, "two entities share SimId {}", id.0),
            Self::Unknown(name) => write!(f, "no registered section is named {}", name.0),
            Self::Value { section, message } => write!(f, "section {}: {message}", section.0),
        }
    }
}

impl std::error::Error for RegistryError {}

type Read<K> = fn(&World, K) -> Option<serde_json::Result<Value>>;
type Write<K> = fn(&mut World, K, Value) -> serde_json::Result<()>;

struct Entry<K> {
    type_id: TypeId,
    read: Read<K>,
    write: Write<K>,
}

/// Every component and resource that is sim state, each by name with typed read and write.
#[derive(Default)]
pub struct Registry {
    components: BTreeMap<&'static str, Entry<Entity>>,
    resources: BTreeMap<&'static str, Entry<()>>,
    derived: BTreeMap<&'static str, TypeId>,
}

impl Registry {
    /// Registers component `T` under `name`.
    #[must_use]
    pub fn component<T: Component + Serialize + DeserializeOwned>(
        mut self,
        name: &'static str,
    ) -> Self {
        self.components.insert(
            name,
            Entry {
                type_id: TypeId::of::<T>(),
                read: |world, entity| world.get::<T>(entity).map(serde_json::to_value),
                write: |world, entity, value| {
                    world
                        .entity_mut(entity)
                        .insert(serde_json::from_value::<T>(value)?);
                    Ok(())
                },
            },
        );
        self
    }

    /// Registers resource `T` under `name`.
    #[must_use]
    pub fn resource<T: Resource + Serialize + DeserializeOwned>(
        mut self,
        name: &'static str,
    ) -> Self {
        self.resources.insert(
            name,
            Entry {
                type_id: TypeId::of::<T>(),
                read: |world, ()| world.get_resource::<T>().map(serde_json::to_value),
                write: |world, (), value| {
                    world.insert_resource(serde_json::from_value::<T>(value)?);
                    Ok(())
                },
            },
        );
        self
    }

    /// Registers resource `T` under `name` as derived: made again from the state, never saved or hashed.
    #[must_use]
    pub fn derived<T: Resource>(mut self, name: &'static str) -> Self {
        self.derived.insert(name, TypeId::of::<T>());
        self
    }

    /// Whether `name` is registered as derived.
    #[must_use]
    pub fn is_derived(&self, name: &str) -> bool {
        self.derived.contains_key(name)
    }

    /// The names of the registered components, in name order.
    pub fn components(&self) -> impl Iterator<Item = &'static str> {
        self.components.keys().copied()
    }

    /// How many component or resource types in the world's sim state are neither registered nor core.
    #[must_use]
    pub fn unregistered(&self, world: &World) -> usize {
        let known: BTreeSet<TypeId> = self
            .components
            .values()
            .map(|entry| entry.type_id)
            .chain(self.resources.values().map(|entry| entry.type_id))
            .chain(self.derived.values().copied())
            .chain([
                TypeId::of::<SimId>(),
                TypeId::of::<Tick>(),
                TypeId::of::<NextId>(),
                TypeId::of::<Streams>(),
                TypeId::of::<DefaultQueryFilters>(),
                TypeId::of::<Schedules>(),
            ])
            .collect();
        let components = world.components();
        let Some(sim_id) = components.get_id(TypeId::of::<SimId>()) else {
            return 0;
        };
        let parts: BTreeSet<_> = world
            .archetypes()
            .iter()
            .filter(|archetype| archetype.contains(sim_id))
            .flat_map(|archetype| archetype.components().iter().copied())
            .chain(world.iter_resources().map(|(id, _, _)| id))
            .collect();
        parts
            .into_iter()
            .filter(|id| {
                components
                    .get_info(*id)
                    .and_then(ComponentInfo::type_id)
                    .is_some_and(|type_id| !known.contains(&type_id))
            })
            .count()
    }

    /// Reads the world's state through the registry, entities in `SimId` order.
    pub fn capture(&self, world: &mut World) -> Result<State, RegistryError> {
        let tick = *world
            .get_resource::<Tick>()
            .ok_or(RegistryError::Missing("Tick"))?;
        let next_id = world
            .get_resource::<NextId>()
            .ok_or(RegistryError::Missing("NextId"))?
            .0;
        let streams = world
            .get_resource::<Streams>()
            .ok_or(RegistryError::Missing("Streams"))?
            .clone();
        let mut resources = BTreeMap::new();
        for (name, entry) in &self.resources {
            if let Some(value) = (entry.read)(world, ()) {
                resources.insert(section(name), value.map_err(|e| invalid(name, &e))?);
            }
        }
        let mut ids = BTreeMap::new();
        for (entity, id) in world.query::<(Entity, &SimId)>().iter(world) {
            if ids.insert(*id, entity).is_some() {
                return Err(RegistryError::DuplicateId(*id));
            }
        }
        let mut entities = BTreeMap::new();
        for (id, entity) in ids {
            let mut parts = BTreeMap::new();
            for (name, entry) in &self.components {
                if let Some(value) = (entry.read)(world, entity) {
                    parts.insert(section(name), value.map_err(|e| invalid(name, &e))?);
                }
            }
            entities.insert(id, parts);
        }
        Ok(State {
            tick,
            next_id,
            streams,
            resources,
            entities,
        })
    }

    /// Builds a new world holding exactly `state`.
    pub fn restore(&self, state: &State) -> Result<World, RegistryError> {
        let mut world = World::new();
        world.insert_resource(state.tick);
        world.insert_resource(NextId(state.next_id));
        world.insert_resource(state.streams.clone());
        for (name, value) in &state.resources {
            let entry = lookup(&self.resources, name)?;
            (entry.write)(&mut world, (), value.clone()).map_err(|e| invalid(&name.0, &e))?;
        }
        for (id, parts) in &state.entities {
            let entity = world.spawn(*id).id();
            for (name, value) in parts {
                let entry = lookup(&self.components, name)?;
                (entry.write)(&mut world, entity, value.clone())
                    .map_err(|e| invalid(&name.0, &e))?;
            }
        }
        Ok(world)
    }
}

impl State {
    /// XXH3-128 over the canonical bytes of the whole state, and per section.
    pub fn hash(&self) -> Result<HashReport, CanonicalError> {
        let mut sections: BTreeMap<SectionName, Xxh3> = BTreeMap::new();
        let mut feed = |name: &SectionName, part: &[u8]| {
            sections.entry(name.clone()).or_default().update(part);
        };
        feed(&section("Streams"), &bytes(&self.streams)?);
        for (name, value) in &self.resources {
            feed(name, &bytes(value)?);
        }
        for (id, parts) in &self.entities {
            for (name, value) in parts {
                feed(name, &bytes(&(id, value))?);
            }
        }
        Ok(HashReport {
            tick: self.tick,
            total: StateHash::of(&bytes(self)?),
            sections: sections
                .into_iter()
                .map(|(name, hasher)| (name, StateHash(hasher.digest128())))
                .collect(),
        })
    }
}

fn bytes<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, CanonicalError> {
    let mut out = Vec::new();
    canonical(value, &mut out)?;
    Ok(out)
}

fn section(name: &str) -> SectionName {
    SectionName(name.to_owned())
}

fn invalid(name: &str, error: &serde_json::Error) -> RegistryError {
    RegistryError::Value {
        section: section(name),
        message: error.to_string(),
    }
}

fn lookup<'a, K>(
    entries: &'a BTreeMap<&'static str, Entry<K>>,
    name: &SectionName,
) -> Result<&'a Entry<K>, RegistryError> {
    entries
        .get(name.0.as_str())
        .ok_or_else(|| RegistryError::Unknown(name.clone()))
}
