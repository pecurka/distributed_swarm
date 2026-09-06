//! Working out which agents are near which.
//!
//! Two ways of answering the same question:
//!
//! - [`brute_force`] compares every agent against every other one. Slow, but
//!   simple enough to trust.
//! - [`grid`] divides the world into squares so each agent only checks a
//!   handful of others.
//!
//! They are a pair on purpose. The fast one is checked against the slow one and
//! must give bit-identical answers, so when they disagree the fast one is
//! wrong. The slow one is never what speed is measured against.

pub mod brute_force;
pub mod grid;
