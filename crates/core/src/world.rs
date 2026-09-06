//! The space the simulation happens in, and what is in it.
//!
//! - [`vector2d`]   positions and velocities
//! - [`geometry`]   distances in a world that wraps around
//! - [`agent`]      a single boid
//! - [`params`]     the settings for a run
//! - [`constants`]  the default values for those settings
//! - [`swarm_init`] building the swarm to start with

pub mod agent;
pub mod constants;
pub mod geometry;
pub mod params;
pub mod swarm_init;
pub mod vector2d;
