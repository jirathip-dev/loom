use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Query, ResMut};
use bevy_ecs::world::World;
use loom_core::{Runner, SimId, Tick, in_order};

#[derive(Resource, Default)]
struct Seen(Vec<String>);

fn first(mut seen: ResMut<Seen>) {
    seen.0.push("first".to_owned());
}

fn second(mut seen: ResMut<Seen>) {
    seen.0.push("second".to_owned());
}

fn visit(mut ids: Query<(&SimId, ())>, mut seen: ResMut<Seen>) {
    for (id, ()) in in_order(&mut ids) {
        seen.0.push(id.0.to_string());
    }
}

#[test]
fn a_step_runs_each_system_once_in_order_then_advances_the_tick() {
    let mut world = World::new();
    world.init_resource::<Seen>();
    for id in [5, 1, 3] {
        world.spawn(SimId(id));
    }
    let mut runner = Runner::new((second, visit, first));
    runner.step(&mut world);
    runner.step(&mut world);
    let seen = &world.resource::<Seen>().0;
    assert_eq!(
        seen,
        &[
            "second", "1", "3", "5", "first", "second", "1", "3", "5", "first"
        ]
    );
    assert_eq!(*world.resource::<Tick>(), Tick(2));
}
