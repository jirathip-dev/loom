use bevy_ecs::component::Component;
use bevy_ecs::resource::Resource;
use bevy_ecs::world::World;
use loom_core::{NextId, Registry, SectionName, Seed, SimId, State, Streams, Tick};
use serde::{Deserialize, Serialize};

#[derive(Component, Clone, Copy, Serialize, Deserialize)]
struct Pos(f32, f32);

#[derive(Component, Clone, Copy, Serialize, Deserialize)]
struct Walker;

#[derive(Resource, Serialize, Deserialize)]
struct Clock {
    day: u32,
}

#[derive(Resource, Serialize, Deserialize)]
struct Weather(String);

fn registry() -> Registry {
    Registry::default()
        .component::<Pos>("Pos")
        .component::<Walker>("Walker")
        .resource::<Clock>("Clock")
        .resource::<Weather>("Weather")
}

fn world() -> World {
    let mut world = World::new();
    world.insert_resource(Tick(9));
    world.insert_resource(NextId(SimId(4)));
    world.insert_resource(Streams::new(Seed(3)));
    world
}

fn forward() -> World {
    let mut world = world();
    world.insert_resource(Clock { day: 2 });
    world.insert_resource(Weather("rain".to_owned()));
    world.spawn((SimId(1), Walker, Pos(1.5, 2.0)));
    world.spawn((SimId(2), Pos(0.25, 8.0)));
    world.spawn((SimId(3), Walker, Pos(-4.0, 0.0)));
    world
}

fn backward() -> World {
    let mut world = world();
    world.insert_resource(Weather("rain".to_owned()));
    world.insert_resource(Clock { day: 2 });
    let three = world.spawn(Pos(-4.0, 0.0)).id();
    world.spawn((Pos(0.25, 8.0), SimId(2)));
    let one = world.spawn(Pos(1.5, 2.0)).id();
    world.entity_mut(one).insert((Walker, SimId(1)));
    world.entity_mut(three).insert((SimId(3), Walker));
    world
}

#[test]
fn equal_state_built_in_another_order_hashes_equal() -> Result<(), Box<dyn std::error::Error>> {
    let registry = registry();
    let a = registry.capture(&mut forward())?.hash()?;
    let b = registry.capture(&mut backward())?.hash()?;
    assert_eq!(a, b);
    assert_eq!(a.tick, Tick(9));
    let names: Vec<&str> = a.sections.keys().map(|name| name.0.as_str()).collect();
    assert_eq!(names, ["Clock", "Pos", "Streams", "Walker", "Weather"]);
    Ok(())
}

#[test]
fn one_changed_bit_changes_the_hash_and_names_its_section() -> Result<(), Box<dyn std::error::Error>>
{
    let registry = registry();
    let before = registry.capture(&mut forward())?.hash()?;

    let mut changed = forward();
    let mut query = changed.query::<(&SimId, &mut Pos)>();
    for (id, mut pos) in query.iter_mut(&mut changed) {
        if *id == SimId(2) {
            pos.0 = f32::from_bits(pos.0.to_bits() ^ 1);
        }
    }
    let after = registry.capture(&mut changed)?.hash()?;

    assert_ne!(before.total, after.total);
    let differ: Vec<&SectionName> = before
        .sections
        .iter()
        .filter(|(name, hash)| after.sections.get(*name) != Some(*hash))
        .map(|(name, _)| name)
        .collect();
    assert_eq!(differ, [&SectionName("Pos".to_owned())]);
    Ok(())
}

#[test]
fn a_state_saved_as_json_restores_to_the_same_hash() -> Result<(), Box<dyn std::error::Error>> {
    let registry = registry();
    let mut original = forward();
    original
        .resource_mut::<Streams>()
        .of(loom_core::StreamName("drift"))
        .bits();
    let state = registry.capture(&mut original)?;
    let saved: State = serde_json::from_str(&serde_json::to_string(&state)?)?;
    let restored = registry.capture(&mut registry.restore(&saved)?)?;
    assert_eq!(restored, state);
    assert_eq!(restored.hash()?, state.hash()?);
    Ok(())
}

#[derive(Resource)]
struct Ground;

#[test]
fn a_derived_resource_is_neither_saved_nor_hashed_and_is_known_as_derived()
-> Result<(), Box<dyn std::error::Error>> {
    let registry = registry().derived::<Ground>("Ground");
    let mut with = forward();
    with.insert_resource(Ground);

    assert_eq!(registry.unregistered(&with), 0);
    assert_eq!(self::registry().unregistered(&with), 1);
    let state = registry.capture(&mut with)?;
    assert_eq!(state, registry.capture(&mut forward())?);
    assert!(
        !state
            .resources
            .contains_key(&SectionName("Ground".to_owned()))
    );
    assert_eq!(state.hash()?, registry.capture(&mut forward())?.hash()?);
    assert!(registry.is_derived("Ground"));
    assert!(!registry.is_derived("Clock"));
    Ok(())
}
