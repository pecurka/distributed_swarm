//! Settings for the simulation.
//!
//! Everything here is a knob you can turn. They live together so a run's setup
//! is readable in one place, and so the sequential and distributed runners
//! can't drift apart — they have to start from the same settings for the
//! comparison between them to mean anything.
//!
//! These stay fixed across runs, so the only thing that changes between
//! measurements is how the work is spread out.

use crate::Vector2D;

/// How many agents to create when a run doesn't say.
pub const DEFAULT_SWARM_SIZE: u64 = 1000;

/// Size of the world. It wraps around, so this is also the wrap distance.
pub const DEFAULT_WORLD: Vector2D = Vector2D::new(1000.0, 1000.0);

/// How far an agent can see (`r`).
///
/// The most important knob in the project. It sets how many neighbours an agent
/// has, and also how much data crosses between nodes, since the shared border
/// region has to be at least this wide.
pub const DEFAULT_PERCEPTION_RADIUS: f64 = 20.0;

/// How strongly agents push away from each other. Higher spreads the flock out,
/// lower packs it together.
pub const DEFAULT_WEIGHT_SEPARATION: f64 = 1.5;

/// How strongly agents match their neighbours' direction.
pub const DEFAULT_WEIGHT_ALIGNMENT: f64 = 1.0;

/// How strongly agents pull toward the middle of the group. This is what makes
/// flocks form — and what unbalances the nodes, since agents end up bunched up.
pub const DEFAULT_WEIGHT_COHESION: f64 = 1.0;

/// Speed limit, applied after the three rules are combined.
pub const DEFAULT_MAX_SPEED: f64 = 4.0;

/// How much time one step covers.
pub const DEFAULT_TIMESTEP: f64 = 1.0;

/// The velocity every agent starts with.
pub const INITIAL_VELOCITY: Vector2D = Vector2D::new(1.0, 0.5);
