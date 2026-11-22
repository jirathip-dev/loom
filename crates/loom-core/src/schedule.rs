use bevy_ecs::query::{IterQueryData, QueryFilter};
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedule, SingleThreadedExecutor};
use bevy_ecs::system::{Query, ScheduleSystem};
use bevy_ecs::world::World;

use crate::{SimId, Tick};

/// One tick's systems, run once each on one thread in the order given.
pub struct Runner(Schedule);

impl Runner {
    /// A runner over `systems`, chained in the order they are listed.
    pub fn new<M>(systems: impl IntoScheduleConfigs<ScheduleSystem, M>) -> Self {
        let mut schedule = Schedule::default();
        schedule.set_executor(SingleThreadedExecutor::new());
        schedule.add_systems(systems.chain());
        Self(schedule)
    }

    /// Runs every system once, then advances the world's `Tick` by one.
    pub fn step(&mut self, world: &mut World) {
        self.0.run(world);
        let mut tick = world.get_resource_or_insert_with(Tick::default);
        let next = tick.next();
        *tick = next;
    }
}

/// The query's items sorted by `SimId`, the only order a system may visit entities in.
pub fn in_order<'a, 's, D: IterQueryData, F: QueryFilter>(
    query: &'a mut Query<'_, 's, (&SimId, D), F>,
) -> std::vec::IntoIter<(&'a SimId, D::Item<'a, 's>)> {
    let mut items: Vec<_> = query.iter_mut().collect();
    items.sort_by_key(|(id, _)| **id);
    items.into_iter()
}
