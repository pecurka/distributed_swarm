//! Builds the starting swarm.
//!
//! Both runs have to start from exactly the same state, or comparing them
//! measures the starting conditions instead of the distribution. So nothing
//! here is random.

use crate::constants::INITIAL_VELOCITY;
use crate::{Agent, Params, Vector2D};

/// Places agents on an evenly spaced grid, all moving the same way.
///
/// A placeholder until seeded random placement lands. Being repeatable matters
/// more than being realistic right now.
pub fn lattice_swarm(swarm_size: u64, params: &Params) -> Vec<Agent> {
    let columns = (swarm_size as f64).sqrt().ceil() as u64;
    let spacing_x = params.world.x / columns as f64;
    let spacing_y = params.world.y / columns as f64;
    (0..swarm_size)
        .map(|id| Agent {
            id,
            position: Vector2D::new(
                (id % columns) as f64 * spacing_x,
                (id / columns) as f64 * spacing_y,
            ),
            velocity: INITIAL_VELOCITY,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lattice_swarm_gives_unique_ids_inside_the_world() {
        let params = Params::default();
        let agents = lattice_swarm(100, &params);
        assert_eq!(agents.len(), 100);
        for (index, agent) in agents.iter().enumerate() {
            assert_eq!(agent.id, index as u64);
            assert!(agent.position.x >= 0.0 && agent.position.x < params.world.x);
            assert!(agent.position.y >= 0.0 && agent.position.y < params.world.y);
        }
    }

    #[test]
    fn lattice_swarm_handles_counts_that_are_not_a_perfect_square() {
        // 10 agents on a 4-wide grid leaves the last row partly empty.
        let params = Params::default();
        let agents = lattice_swarm(10, &params);
        assert_eq!(agents.len(), 10);
        for agent in &agents {
            assert!(agent.position.x >= 0.0 && agent.position.x < params.world.x);
            assert!(agent.position.y >= 0.0 && agent.position.y < params.world.y);
        }
    }

    #[test]
    fn lattice_swarm_of_zero_is_empty() {
        // With no agents the column count is zero, so the spacing maths would
        // divide by zero if it ever ran. It doesn't, because the loop is empty
        // — this test makes sure a future rewrite keeps it that way.
        let params = Params::default();
        assert!(lattice_swarm(0, &params).is_empty());
    }

    #[test]
    fn lattice_swarm_of_one_places_it_at_the_origin() {
        let params = Params::default();
        let agents = lattice_swarm(1, &params);
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].position, Vector2D::ZERO);
    }

    #[test]
    fn lattice_swarm_gives_every_agent_the_same_starting_velocity() {
        let params = Params::default();
        for agent in lattice_swarm(20, &params) {
            assert_eq!(agent.velocity, INITIAL_VELOCITY);
        }
    }

    #[test]
    fn lattice_swarm_is_repeatable() {
        // Both runs must start from identical state, so building the swarm
        // twice has to give exactly the same agents.
        let params = Params::default();
        assert_eq!(lattice_swarm(50, &params), lattice_swarm(50, &params));
    }
}
