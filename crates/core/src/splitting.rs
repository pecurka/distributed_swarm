//! Dividing the world between processes.
//!
//! No MPI in here. Working out who owns what is arithmetic, so it can be tested
//! normally rather than only when launched across several processes — which
//! also means the distributed algorithm can be run and checked inside a single
//! process, against the sequential result.
//!
//! - [`partition`] which strip of the world belongs to which process
//! - [`borders`]   sharing copies of agents across strip boundaries

pub mod borders;
pub mod partition;
