//! The three rules that make a flock.
//!
//! Each rule looks at an agent's neighbours and returns a direction to steer
//! in. Nobody tells the agents to form a flock — flocking is what happens when
//! every agent follows these three rules at once.
//!
//! Each rule returns a direction of length 1 (or zero, if it has nothing to
//! say). That keeps the three comparable, so the weights in `Params` decide how
//! much each one matters rather than the rules accidentally out-shouting each
//! other because of how their maths happens to scale.

use crate::{Neighbour, Params, Vector2D};

/// How small a leftover has to be, compared with the parts that made it, before
/// we treat it as nothing.
const CANCELLED_OUT: f64 = 1e-9;

/// Turns a summed-up steering vector into a direction of length 1.
///
/// `combined_size` is how big the individual parts were before they were added
/// together. If the total is tiny compared with them, they cancelled each other
/// out — which is what happens when an agent sits exactly in the middle of a
/// symmetric group. What is left is rounding error, not a direction. Stretching
/// that to length 1 would turn numerical dust into a full-strength push in a
/// meaningless direction, so return nothing instead.
fn direction_of(total: Vector2D, combined_size: f64) -> Vector2D {
    if total.len() <= CANCELLED_OUT * combined_size {
        Vector2D::ZERO
    } else {
        total.normalised()
    }
}

/// Steer away from anyone too close.
///
/// Only neighbours nearer than `separation_radius` count. This rule is
/// deliberately short-range while the other two use the full perception radius.
/// If it reached as far as they do, an agent would push away from everything it
/// can see, which always beats the pull toward the group: the swarm ends up
/// evenly spaced instead of flocked.
///
/// Among those close neighbours, closer ones push harder — each contributes a
/// push of size 1 / distance.
pub fn separation(neighbours: &[Neighbour], separation_radius: f64) -> Vector2D {
    let too_close = separation_radius * separation_radius;
    let mut push = Vector2D::ZERO;
    let mut combined_size = 0.0;
    for neighbour in neighbours {
        let distance_squared = neighbour.offset.len_sq();
        if distance_squared > 0.0 && distance_squared <= too_close {
            // Away from them, divided by distance squared: direction over
            // distance, so the push shrinks as they get further away.
            let contribution = neighbour.offset * (-1.0 / distance_squared);
            push = push + contribution;
            combined_size += contribution.len();
        }
    }
    direction_of(push, combined_size)
}

/// Steer to match the direction the neighbours are heading.
pub fn alignment(neighbours: &[Neighbour], own_velocity: Vector2D) -> Vector2D {
    if neighbours.is_empty() {
        return Vector2D::ZERO;
    }
    let mut total = Vector2D::ZERO;
    for neighbour in neighbours {
        total = total + neighbour.velocity;
    }
    let average = total * (1.0 / neighbours.len() as f64);
    direction_of(average - own_velocity, average.len() + own_velocity.len())
}

/// Steer toward the middle of the group.
///
/// Uses the offsets rather than the neighbours' absolute positions, which
/// matters in a world that wraps around: averaging raw positions of agents on
/// opposite edges would point at the middle of the map instead of at the group.
pub fn cohesion(neighbours: &[Neighbour]) -> Vector2D {
    if neighbours.is_empty() {
        return Vector2D::ZERO;
    }
    let mut total = Vector2D::ZERO;
    let mut combined_size = 0.0;
    for neighbour in neighbours {
        total = total + neighbour.offset;
        combined_size += neighbour.offset.len();
    }
    let neighbour_count = neighbours.len() as f64;
    direction_of(total * (1.0 / neighbour_count), combined_size / neighbour_count)
}

/// All three rules blended with the weights from `Params`.
///
/// An agent with no neighbours gets zero from all three and simply carries on
/// in the direction it was already going.
pub fn steer(neighbours: &[Neighbour], own_velocity: Vector2D, params: &Params) -> Vector2D {
    separation(neighbours, params.separation_radius) * params.weight_separation
        + alignment(neighbours, own_velocity) * params.weight_alignment
        + cohesion(neighbours) * params.weight_cohesion
}

#[cfg(test)]
mod tests {
    use super::*;

    fn neighbour_at(id: u64, offset: Vector2D, velocity: Vector2D) -> Neighbour {
        Neighbour {
            id,
            offset,
            velocity,
        }
    }

    #[test]
    fn separation_points_away_from_a_close_neighbour() {
        // Neighbour is to the right, so we should be pushed left.
        let neighbours = vec![neighbour_at(1, Vector2D::new(2.0, 0.0), Vector2D::ZERO)];
        assert_eq!(separation(&neighbours, 50.0), Vector2D::new(-1.0, 0.0));
    }

    #[test]
    fn separation_is_dominated_by_the_nearest_neighbour() {
        // One neighbour very close on the right, one far away on the left.
        // The close one should win, pushing us left.
        let neighbours = vec![
            neighbour_at(1, Vector2D::new(1.0, 0.0), Vector2D::ZERO),
            neighbour_at(2, Vector2D::new(-15.0, 0.0), Vector2D::ZERO),
        ];
        assert!(separation(&neighbours, 50.0).x < 0.0);
    }

    #[test]
    fn alignment_points_toward_the_neighbours_direction() {
        // We are going right, the neighbour is going up, so we should be
        // steered upward.
        let neighbours = vec![neighbour_at(1, Vector2D::new(3.0, 0.0), Vector2D::new(0.0, 2.0))];
        let steer_direction = alignment(&neighbours, Vector2D::new(2.0, 0.0));
        assert!(steer_direction.y > 0.0);
        assert!(steer_direction.x < 0.0, "should also stop going right so hard");
    }

    #[test]
    fn cohesion_points_toward_the_middle_of_the_group() {
        // Both neighbours are up and to the right.
        let neighbours = vec![
            neighbour_at(1, Vector2D::new(10.0, 10.0), Vector2D::ZERO),
            neighbour_at(2, Vector2D::new(6.0, 2.0), Vector2D::ZERO),
        ];
        let steer_direction = cohesion(&neighbours);
        assert!(steer_direction.x > 0.0);
        assert!(steer_direction.y > 0.0);
    }

    #[test]
    fn every_rule_is_silent_with_no_neighbours() {
        let params = Params::default();
        let own_velocity = Vector2D::new(1.0, 1.0);
        assert_eq!(separation(&[], 50.0), Vector2D::ZERO);
        assert_eq!(alignment(&[], own_velocity), Vector2D::ZERO);
        assert_eq!(cohesion(&[]), Vector2D::ZERO);
        assert_eq!(steer(&[], own_velocity, &params), Vector2D::ZERO);
    }

    #[test]
    fn rules_return_directions_of_length_one() {
        // The weights in Params decide how much each rule matters, so the rules
        // themselves must not vary in strength.
        let neighbours = vec![neighbour_at(1, Vector2D::new(4.0, 3.0), Vector2D::new(0.0, 1.0))];
        for direction in [
            separation(&neighbours, 50.0),
            alignment(&neighbours, Vector2D::new(1.0, 0.0)),
            cohesion(&neighbours),
        ] {
            assert!((direction.len() - 1.0).abs() < 1e-9, "got {direction:?}");
        }
    }
}
