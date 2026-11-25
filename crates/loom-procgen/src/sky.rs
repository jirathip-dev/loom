use std::f32::consts::TAU;

use glam::Vec3;
use serde::{Deserialize, Serialize};

const DAY: f32 = 24.0;
const NOON_HEIGHT: f32 = 1.2;
const LOWEST_HEIGHT: f32 = 0.15;

/// A day's light as keys over its 24 hours, blended between them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sky {
    pub keys: Vec<SkyKey>,
}

/// The light's, the sky's and the ambient colour at one hour of the day, 0 to 24.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkyKey {
    pub at: f32,
    pub light: [f32; 3],
    pub sky: [f32; 3],
    pub ambient: [f32; 3],
}

/// The direction towards the one light and the colours at one time of day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyNow {
    pub towards: Vec3,
    pub light: [f32; 3],
    pub sky: [f32; 3],
    pub ambient: [f32; 3],
}

impl Default for Sky {
    fn default() -> Self {
        let key = |at, light, sky, ambient| SkyKey {
            at,
            light,
            sky,
            ambient,
        };
        Self {
            keys: vec![
                key(
                    0.0,
                    [0.08, 0.1, 0.2],
                    [0.01, 0.015, 0.04],
                    [0.02, 0.025, 0.05],
                ),
                key(6.0, [1.2, 0.6, 0.35], [0.6, 0.4, 0.35], [0.15, 0.12, 0.12]),
                key(12.0, [3.0, 2.9, 2.7], [0.4, 0.6, 0.9], [0.25, 0.28, 0.32]),
                key(18.0, [1.2, 0.5, 0.3], [0.55, 0.35, 0.35], [0.14, 0.1, 0.11]),
            ],
        }
    }
}

impl Sky {
    /// The light and colours at `time` hours, blended between the nearest keys either side.
    #[must_use]
    pub fn at(&self, time: f32) -> SkyNow {
        let hour = time.rem_euclid(DAY);
        let turn = hour / DAY * TAU;
        let height = LOWEST_HEIGHT + (NOON_HEIGHT - LOWEST_HEIGHT) * (1.0 - libm::cosf(turn)) / 2.0;
        let flat = libm::cosf(height);
        let towards = Vec3::new(
            libm::sinf(turn) * flat,
            libm::sinf(height),
            libm::cosf(turn) * flat,
        );
        let mut before: Option<(f32, &SkyKey)> = None;
        let mut after: Option<(f32, &SkyKey)> = None;
        for key in &self.keys {
            let back = (hour - key.at).rem_euclid(DAY);
            let ahead = (key.at - hour).rem_euclid(DAY);
            if before.is_none_or(|(least, _)| back < least) {
                before = Some((back, key));
            }
            if after.is_none_or(|(least, _)| ahead < least) {
                after = Some((ahead, key));
            }
        }
        let (Some((back, from)), Some((ahead, to))) = (before, after) else {
            return SkyNow {
                towards,
                light: [0.0; 3],
                sky: [0.0; 3],
                ambient: [0.0; 3],
            };
        };
        let t = if back + ahead > 0.0 {
            back / (back + ahead)
        } else {
            0.0
        };
        let mix = |a: [f32; 3], b: [f32; 3]| Vec3::from(a).lerp(Vec3::from(b), t).to_array();
        SkyNow {
            towards,
            light: mix(from.light, to.light),
            sky: mix(from.sky, to.sky),
            ambient: mix(from.ambient, to.ambient),
        }
    }
}
