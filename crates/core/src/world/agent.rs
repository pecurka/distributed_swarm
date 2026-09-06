//! The thing being simulated.

use crate::Vector2D;

/// A single boid.
///
/// `id` identifies an agent no matter which node currently owns it. It also
/// fixes the order neighbours get added up in. That matters because floating
/// point addition gives slightly different answers depending on the order, so
/// without a fixed order the sequential and distributed runs would drift apart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Agent {
    pub id: u64,
    pub position: Vector2D,
    pub velocity: Vector2D,
}
