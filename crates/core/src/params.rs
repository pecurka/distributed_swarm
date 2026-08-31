//! Settings for a single run.

use crate::Vector2D;
use crate::constants::{
    DEFAULT_DT, DEFAULT_MAX_SPEED, DEFAULT_PERCEPTION_RADIUS, DEFAULT_WEIGHT_ALIGNMENT,
    DEFAULT_WEIGHT_COHESION, DEFAULT_WEIGHT_SEPARATION, DEFAULT_WORLD,
};

/// Simulation settings.
///
/// These stay the same across runs, so the only difference between the
/// sequential and distributed measurements is how the work is split up.
#[derive(Debug, Clone, Copy)]
pub struct Params {
    /// Size of the world. It wraps around, so this is also the wrap distance.
    pub world: Vector2D,
    /// How far an agent can see (`r`). Also the smallest the shared border
    /// between nodes is allowed to be.
    pub perception_radius: f64,
    pub weight_separation: f64,
    pub weight_alignment: f64,
    pub weight_cohesion: f64,
    pub max_speed: f64,
    pub dt: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            world: DEFAULT_WORLD,
            perception_radius: DEFAULT_PERCEPTION_RADIUS,
            weight_separation: DEFAULT_WEIGHT_SEPARATION,
            weight_alignment: DEFAULT_WEIGHT_ALIGNMENT,
            weight_cohesion: DEFAULT_WEIGHT_COHESION,
            max_speed: DEFAULT_MAX_SPEED,
            dt: DEFAULT_DT,
        }
    }
}
