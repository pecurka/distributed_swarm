//! The boids model, shared by the sequential and distributed runners.
//!
//! Nothing here knows about MPI. That's the point: both runners use this same
//! code, so any difference in their results comes from the distribution and
//! nothing else.
//!
//! The modules are grouped by what they are for:
//!
//! - [`world`]      the space and what is in it
//! - [`neighbours`] who is near whom
//! - [`behaviour`]  the three rules, and one step of the simulation
//! - [`splitting`]  dividing the world between processes
//! - [`output`]     measuring and showing
//!
//! Everything is re-exported below, so callers can say `swarm_core::Agent`
//! without needing to know which folder it lives in.

pub mod behaviour;
pub mod neighbours;
pub mod output;
pub mod splitting;
pub mod world;

pub use behaviour::simulation::{run, step, step_slowly, step_with_ghosts};
pub use behaviour::steering::{alignment, cohesion, separation, steer};
pub use neighbours::brute_force::{Neighbour, find_neighbours};
pub use neighbours::grid::Grid;
pub use output::metrics::{
    average_neighbour_count, local_alignment, neighbour_counts, polarisation,
};
pub use output::recording::Recorder;
pub use output::report::{configuration_report, progress_heading, progress_line};
pub use splitting::borders::{
    agents_to_send_left, agents_to_send_right, decode_from_numbers, encode_to_numbers,
};
pub use splitting::partition::Partition;
pub use world::agent::Agent;
pub use world::constants::*;
pub use world::geometry::{toroidal_delta, wrap};
pub use world::params::Params;
pub use world::swarm_init::{lattice_swarm, scattered_swarm};
pub use world::vector2d::Vector2D;
