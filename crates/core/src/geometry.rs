//! Distance and position maths for a wrap-around world.
//!
//! The world is a torus: go off the right edge and you come back on the left.
//! Both distance and position have to account for that, so it lives in one
//! place. The steering rules and the node partitioning both need to agree on
//! what counts as close.

use crate::Vector2D;

/// Shortest path from one point to another in a wrap-around world.
///
/// Two agents on opposite edges are really neighbours, so plain subtraction
/// gives the wrong answer because it measures the long way round.
pub fn toroidal_delta(from: Vector2D, to: Vector2D, world: Vector2D) -> Vector2D {
    let mut delta = to - from;
    if delta.x > world.x * 0.5 {
        delta.x -= world.x;
    } else if delta.x < -world.x * 0.5 {
        delta.x += world.x;
    }
    if delta.y > world.y * 0.5 {
        delta.y -= world.y;
    } else if delta.y < -world.y * 0.5 {
        delta.y += world.y;
    }
    delta
}

/// Bring a position back inside the world, wrapping at the edges.
pub fn wrap(position: Vector2D, world: Vector2D) -> Vector2D {
    Vector2D::new(
        position.x.rem_euclid(world.x),
        position.y.rem_euclid(world.y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toroidal_delta_wraps_across_the_seam() {
        let world = Vector2D::new(100.0, 100.0);
        // Two agents 4 apart, but on opposite sides of the x=0 edge.
        let from = Vector2D::new(98.0, 50.0);
        let to = Vector2D::new(2.0, 50.0);
        let delta = toroidal_delta(from, to, world);
        assert_eq!(delta.x, 4.0, "should wrap around, not go the long way");
        assert_eq!(delta.y, 0.0);
    }

    #[test]
    fn toroidal_delta_matches_plain_difference_away_from_the_seam() {
        let world = Vector2D::new(100.0, 100.0);
        let delta = toroidal_delta(Vector2D::new(40.0, 40.0), Vector2D::new(45.0, 48.0), world);
        assert_eq!(delta, Vector2D::new(5.0, 8.0));
    }

    #[test]
    fn wrap_brings_positions_back_into_the_domain() {
        let world = Vector2D::new(100.0, 100.0);
        assert_eq!(
            wrap(Vector2D::new(105.0, -5.0), world),
            Vector2D::new(5.0, 95.0)
        );
    }

    #[test]
    fn toroidal_delta_wraps_across_the_y_seam() {
        // Mirror of the x-axis test. The y branch is separate code, so a
        // copy-paste slip there would pass every other test in this file.
        let world = Vector2D::new(100.0, 100.0);
        let delta = toroidal_delta(Vector2D::new(50.0, 98.0), Vector2D::new(50.0, 2.0), world);
        assert_eq!(delta.x, 0.0);
        assert_eq!(delta.y, 4.0, "should wrap around, not go the long way");
    }

    #[test]
    fn toroidal_delta_uses_each_axis_own_size() {
        // A non-square world catches a branch that reads world.x when it
        // should read world.y.
        let world = Vector2D::new(100.0, 40.0);
        let delta = toroidal_delta(Vector2D::new(50.0, 38.0), Vector2D::new(50.0, 2.0), world);
        assert_eq!(delta.y, 4.0);
    }

    #[test]
    fn toroidal_delta_flips_sign_when_arguments_swap() {
        let world = Vector2D::new(100.0, 100.0);
        let from = Vector2D::new(10.0, 20.0);
        let to = Vector2D::new(30.0, 45.0);
        let forward = toroidal_delta(from, to, world);
        let backward = toroidal_delta(to, from, world);
        assert_eq!(forward.x, -backward.x);
        assert_eq!(forward.y, -backward.y);
    }

    #[test]
    fn toroidal_delta_never_exceeds_half_the_world() {
        // The whole point of wrapping: no two points are further apart than
        // half the world on any axis.
        let world = Vector2D::new(100.0, 60.0);
        for offset_x in [0.0, 1.0, 25.0, 50.0, 75.0, 99.0] {
            for offset_y in [0.0, 5.0, 30.0, 59.0] {
                let delta = toroidal_delta(
                    Vector2D::ZERO,
                    Vector2D::new(offset_x, offset_y),
                    world,
                );
                assert!(delta.x.abs() <= world.x * 0.5, "x too far: {delta:?}");
                assert!(delta.y.abs() <= world.y * 0.5, "y too far: {delta:?}");
            }
        }
    }

    #[test]
    fn wrap_maps_the_far_edge_back_to_zero() {
        // A position landing exactly on the edge belongs at the start, not
        // outside the world.
        let world = Vector2D::new(100.0, 100.0);
        assert_eq!(wrap(Vector2D::new(100.0, 100.0), world), Vector2D::ZERO);
    }

    #[test]
    fn wrap_handles_positions_several_worlds_away() {
        let world = Vector2D::new(100.0, 100.0);
        assert_eq!(
            wrap(Vector2D::new(250.0, -150.0), world),
            Vector2D::new(50.0, 50.0)
        );
    }

    #[test]
    fn wrap_leaves_positions_already_inside_alone() {
        let world = Vector2D::new(100.0, 100.0);
        let position = Vector2D::new(12.5, 87.5);
        assert_eq!(wrap(position, world), position);
    }
}
