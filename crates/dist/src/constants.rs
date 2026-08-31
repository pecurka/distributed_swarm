//! Settings for the distributed runner.
//!
//! Kept out of `swarm-core` on purpose. That crate knows nothing about MPI,
//! which is what lets the sequential runner share the model code without
//! pulling MPI in as a dependency.

/// The rank that prints output.
///
/// MPI doesn't treat rank 0 as special — this is just our convention, so that
/// per-run output happens once instead of once per process. It gets a name
/// because it will show up often, and a bare `0` there looks like an index.
pub const ROOT_RANK: i32 = 0;

/// Used when MPI can't tell us the machine name.
pub const UNKNOWN_PROCESSOR: &str = "unknown";
