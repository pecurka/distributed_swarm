//! Measuring what happened and showing it.
//!
//! - [`metrics`]   numbers describing the swarm as a whole
//! - [`report`]    printing a run's settings and progress
//! - [`recording`] saving positions to a file so a run can be drawn

pub mod metrics;
pub mod recording;
pub mod report;
