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
/// has, and also how much data crosses between machines, since the shared
/// border region has to be at least this wide.
///
/// Raised from 20 to 50 because at 20 the swarm barely flocked. The number of
/// neighbours an agent has is roughly:
///
/// ```text
///     agents / world area  x  area of the circle it can see
/// ```
///
/// With 1000 agents in a 1000x1000 world and a radius of 20, that came to about
/// 1.2 neighbours — most agents were flying alone with nothing to react to, and
/// the flocking measure only crept from 0.06 to 0.22. A radius of 50 gives
/// about 7.9 neighbours instead, which is enough for flocks to form at all.
/// That turned out to be necessary but not sufficient on its own — the steering
/// weights below had to come down as well before the swarm really flocked.
///
/// The trade-off is that this is also the setting that decides how much data
/// crosses between machines later, so a bigger radius means more communication.
/// That is a real cost, not a free improvement — but a simulation that does not
/// flock is not worth measuring.
pub const DEFAULT_PERCEPTION_RADIUS: f64 = 50.0;

// --- Steering weights -------------------------------------------------------
//
// How strongly each of the three rules pulls. Their sizes *relative to each
// other* give the flock its character. Their size *overall* decides how twitchy
// it is, and that turned out to matter far more than expected.
//
// These started at 1.5 / 1.0 / 1.0 and were divided by ten. At the original
// values an agent's velocity could swing by 3.5 in a single step, out of a
// maximum speed of 4 — agents were thrown about every step and never settled.
// The symptom was specific: even agents right next to each other did not agree
// on a direction.
//
// Measured after 600 steps with 1000 agents, keeping the 1.5 : 1 : 1 ratio and
// scaling all three together. "nearby" is how well an agent matches the agents
// it can actually see; "overall" is how well the whole swarm moves as one.
// Both run from 0 (no agreement) to 1 (perfect).
//
//     scale   biggest change per step   nearby   overall
//      1.00            3.50              0.43      0.17
//      0.50            1.75              0.89      0.87
//      0.25            0.88              0.94      0.91
//      0.10            0.35              0.95      0.94   <- chosen
//      0.05            0.18              0.99      0.98
//      0.02            0.07              0.95      0.26
//
// Anything between 0.05 and 0.25 flocks properly. 0.10 was picked as the middle
// of that range, comfortably clear of both edges.
//
// The last row is worth understanding rather than dismissing. Agents still
// agree with their neighbours, but the swarm as a whole does not line up: the
// steering is so gentle that lots of small tidy flocks form and never merge
// within 600 steps. A low overall number does not by itself mean the
// simulation is broken — always check the nearby number too.

/// How strongly agents push away from each other. Higher spreads the flock out,
/// lower packs it together.
pub const DEFAULT_WEIGHT_SEPARATION: f64 = 0.15;

/// How strongly agents match their neighbours' direction.
pub const DEFAULT_WEIGHT_ALIGNMENT: f64 = 0.10;

/// How strongly agents pull toward the middle of the group. This is what makes
/// flocks form — and what unbalances the machines later, since agents end up
/// bunched together instead of spread evenly.
pub const DEFAULT_WEIGHT_COHESION: f64 = 0.10;

/// Speed limit, applied after the three rules are combined.
pub const DEFAULT_MAX_SPEED: f64 = 4.0;

/// How much time one step covers.
pub const DEFAULT_TIMESTEP: f64 = 1.0;

/// The velocity every agent starts with.
pub const INITIAL_VELOCITY: Vector2D = Vector2D::new(1.0, 0.5);
