use std::time::Duration;

const MOST_A_FRAME: u32 = 8;

/// Turns real time into whole simulation ticks of a fixed length.
#[derive(Clone, Copy, Debug)]
pub struct Pacer {
    tick: Duration,
    owed: Duration,
}

impl Pacer {
    #[must_use]
    pub fn new(ticks_per_second: u32) -> Self {
        Self {
            tick: (Duration::from_secs(1) / ticks_per_second.max(1)).max(Duration::from_nanos(1)),
            owed: Duration::ZERO,
        }
    }

    /// The whole ticks due after `dt` seconds at `speed` times real time, at most 8; time beyond that is dropped.
    pub fn add(&mut self, dt: f32, speed: u32) -> u32 {
        self.owed += Duration::try_from_secs_f32(dt).unwrap_or_default() * speed;
        let mut ticks = 0;
        while ticks < MOST_A_FRAME && self.owed >= self.tick {
            self.owed -= self.tick;
            ticks += 1;
        }
        if self.owed >= self.tick {
            let left = self.owed.as_nanos() % self.tick.as_nanos();
            self.owed = Duration::from_nanos(u64::try_from(left).unwrap_or(0));
        }
        ticks
    }

    /// How far the clock is into the next tick, 0 to 1.
    #[must_use]
    pub fn alpha(&self) -> f32 {
        (self.owed.as_secs_f32() / self.tick.as_secs_f32()).clamp(0.0, 1.0)
    }
}
