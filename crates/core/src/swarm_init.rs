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

/// Scatters agents across the world, each heading a different way.
///
/// The lattice above is too tidy to flock: a perfectly regular grid where
/// everyone moves identically is balanced, so every rule cancels out and
/// nothing ever happens. This spreads them unevenly instead, which is what a
/// real starting state looks like.
///
/// It is random-looking but not actually random. The numbers come from mixing
/// up the agent's id, so the same swarm size always produces exactly the same
/// swarm, on any machine. Both runners have to start from identical state or
/// comparing them measures the starting conditions.
pub fn scattered_swarm(swarm_size: u64, params: &Params) -> Vec<Agent> {
    (0..swarm_size)
        .map(|id| {
            let mut seed = id;
            let x = params.world.x * random_fraction(&mut seed);
            let y = params.world.y * random_fraction(&mut seed);
            let heading = std::f64::consts::TAU * random_fraction(&mut seed);
            let speed = params.max_speed * (0.5 + 0.5 * random_fraction(&mut seed));
            Agent {
                id,
                position: Vector2D::new(x, y),
                velocity: Vector2D::new(heading.cos(), heading.sin()) * speed,
            }
        })
        .collect()
}

/// Turns a counter into a number between 0 and 1 that looks random.
///
/// This is a well-known bit-mixing routine (SplitMix64). We use our own rather
/// than a library so the numbers are guaranteed to be identical everywhere the
/// code runs, which is what makes runs repeatable.
fn random_fraction(state: &mut u64) -> f64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut mixed = *state;
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    mixed ^= mixed >> 31;
    // Keep the top 53 bits — that is as much precision as an f64 can hold.
    (mixed >> 11) as f64 / (1u64 << 53) as f64
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
    fn scattered_swarm_stays_inside_the_world() {
        let params = Params::default();
        for agent in scattered_swarm(200, &params) {
            assert!(agent.position.x >= 0.0 && agent.position.x < params.world.x);
            assert!(agent.position.y >= 0.0 && agent.position.y < params.world.y);
            assert!(agent.velocity.len() <= params.max_speed + 1e-9);
        }
    }

    #[test]
    fn scattered_swarm_is_repeatable() {
        let params = Params::default();
        assert_eq!(scattered_swarm(50, &params), scattered_swarm(50, &params));
    }

    #[test]
    fn scattered_swarm_points_agents_in_different_directions() {
        // The whole point of scattering: if they all started aligned there
        // would be nothing for the rules to do.
        let params = Params::default();
        let agents = scattered_swarm(100, &params);
        let first = agents[0].velocity.normalised();
        assert!(
            agents
                .iter()
                .any(|agent| (agent.velocity.normalised() - first).len() > 0.5),
            "every agent ended up heading the same way"
        );
    }

    #[test]
    fn lattice_swarm_is_repeatable() {
        // Both runs must start from identical state, so building the swarm
        // twice has to give exactly the same agents.
        let params = Params::default();
        assert_eq!(lattice_swarm(50, &params), lattice_swarm(50, &params));
    }
}
