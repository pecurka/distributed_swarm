//! The boids model, shared by the sequential and distributed runners.
//!
//! Nothing here knows about MPI or about how the work gets split up. That's the
//! point: both runners use this same code, so any difference in their results
//! comes from the distribution and nothing else.
//!
//! This file just lists the modules:
//!
//! - [`vector2d`]   the 2D vector type
//! - [`agent`]      a single boid
//! - [`params`]     settings for a run
//! - [`constants`]  default values for those settings
//! - [`geometry`]   distance maths for the wrap-around world
//! - [`swarm_init`] builds the starting swarm

pub mod agent;
pub mod constants;
pub mod geometry;
pub mod params;
pub mod swarm_init;
pub mod vector2d;

pub use agent::Agent;
pub use constants::*;
pub use geometry::{toroidal_delta, wrap};
pub use params::Params;
pub use swarm_init::lattice_swarm;
pub use vector2d::Vector2D;
