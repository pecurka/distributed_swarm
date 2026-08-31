//! Moving the whole swarm forward one step.

use crate::{Agent, Params, find_neighbours, steer, wrap};

/// Work out where every agent goes next.
///
/// Every agent's new state is worked out from the *old* positions of all the
/// others, and only then does anything get written. Never one agent at a time:
/// if agent 1 moved before agent 2 looked at it, agent 2 would see the new
/// position instead of the old one. The distributed version cannot work that
/// way, so if this one did, the two would disagree for reasons that have
/// nothing to do with distribution.
///
/// Returning a fresh list rather than editing in place is what enforces that.
pub fn step(agents: &[Agent], params: &Params) -> Vec<Agent> {
    agents
        .iter()
        .map(|agent| {
            let neighbours = find_neighbours(agent, agents, params);
            let acceleration = steer(&neighbours, agent.velocity, params);

            let velocity =
                (agent.velocity + acceleration * params.timestep).clamped_to(params.max_speed);
            let position = wrap(agent.position + velocity * params.timestep, params.world);

            Agent {
                id: agent.id,
                position,
                velocity,
            }
        })
        .collect()
}

/// Run several steps in a row.
pub fn run(agents: &[Agent], steps: u64, params: &Params) -> Vec<Agent> {
    let mut current = agents.to_vec();
    for _ in 0..steps {
        current = step(&current, params);
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Vector2D, average_neighbour_count, lattice_swarm, local_alignment, neighbour_counts,
        polarisation, scattered_swarm,
    };

    #[test]
    fn a_lone_agent_carries_on_in_a_straight_line() {
        // No neighbours means all three rules say nothing, so it should coast.
        let params = Params::default();
        let agents = vec![Agent {
            id: 0,
            position: Vector2D::new(10.0, 10.0),
            velocity: Vector2D::new(2.0, 1.0),
        }];
        let next = step(&agents, &params);
        assert_eq!(next[0].velocity, Vector2D::new(2.0, 1.0));
        assert_eq!(next[0].position, Vector2D::new(12.0, 11.0));
    }

    #[test]
    fn agents_never_exceed_the_speed_limit() {
        let params = Params::default();
        let agents = run(&scattered_swarm(200, &params), 50, &params);
        for agent in &agents {
            assert!(
                agent.velocity.len() <= params.max_speed + 1e-9,
                "agent {} went too fast: {}",
                agent.id,
                agent.velocity.len()
            );
        }
    }

    #[test]
    fn agents_stay_inside_the_world() {
        let params = Params::default();
        let agents = run(&scattered_swarm(200, &params), 50, &params);
        for agent in &agents {
            assert!(agent.position.x >= 0.0 && agent.position.x < params.world.x);
            assert!(agent.position.y >= 0.0 && agent.position.y < params.world.y);
        }
    }

    #[test]
    fn no_agent_is_created_lost_or_renamed() {
        let params = Params::default();
        let start = scattered_swarm(100, &params);
        let end = run(&start, 20, &params);
        assert_eq!(end.len(), start.len());
        let ids: Vec<u64> = end.iter().map(|agent| agent.id).collect();
        let original: Vec<u64> = start.iter().map(|agent| agent.id).collect();
        assert_eq!(ids, original);
    }

    #[test]
    fn running_twice_gives_exactly_the_same_answer() {
        // Everything downstream depends on this: without it there is no way to
        // tell a real difference from floating point noise.
        let params = Params::default();
        let start = scattered_swarm(100, &params);
        assert_eq!(run(&start, 30, &params), run(&start, 30, &params));
    }

    #[test]
    fn every_agent_is_updated_from_the_same_old_state() {
        // Two agents mirrored around a point, moving in opposite directions.
        // Their situations are identical, so their new velocities must be exact
        // mirrors. If one were updated before the other looked at it, the
        // second would see a moved agent and the mirror would break.
        let params = Params {
            world: Vector2D::new(100.0, 100.0),
            perception_radius: 20.0,
            ..Params::default()
        };
        let agents = vec![
            Agent {
                id: 0,
                position: Vector2D::new(45.0, 50.0),
                velocity: Vector2D::new(1.0, 0.0),
            },
            Agent {
                id: 1,
                position: Vector2D::new(55.0, 50.0),
                velocity: Vector2D::new(-1.0, 0.0),
            },
        ];
        let next = step(&agents, &params);
        assert!((next[0].velocity.x + next[1].velocity.x).abs() < 1e-12);
        assert!((next[0].velocity.y - next[1].velocity.y).abs() < 1e-12);
    }

    #[test]
    fn a_perfectly_even_lattice_never_moves_off_course() {
        // 1024 = 32x32, so this lattice is perfectly regular and everyone
        // starts moving the same way. Every rule should cancel to exactly
        // nothing and the swarm should coast forever.
        //
        // This catches a subtle bug: when the parts of a rule cancel out, what
        // is left is rounding error. Stretching that to full length would turn
        // noise into a real push, and the lattice would slowly fall apart.
        let params = Params {
            world: Vector2D::new(400.0, 400.0),
            ..Params::default()
        };
        let start = lattice_swarm(1024, &params);
        let end = run(&start, 100, &params);
        for (after, before) in end.iter().zip(start.iter()) {
            assert_eq!(
                after.velocity, before.velocity,
                "agent {} was nudged off course",
                before.id
            );
        }
        assert!((polarisation(&end) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn the_swarm_clumps_together_rather_than_spreading_out() {
        // Catches a bug that alignment alone cannot see: if the "push away"
        // rule reaches as far as an agent can see, the swarm settles into an
        // evenly spaced grid. Every agent points the same way, so the alignment
        // measure reads near-perfect, but nothing has flocked — they are spread
        // out, not together.
        //
        // Measured: a real flock goes from 6.3 neighbours each to about 48.
        // The evenly spaced version drops to 5.4.
        let params = Params::default();
        let start = scattered_swarm(800, &params);
        let end = run(&start, 600, &params);

        let before = average_neighbour_count(&start, &params);
        let after = average_neighbour_count(&end, &params);
        assert!(
            after > before * 3.0,
            "swarm did not clump together: {before:.1} neighbours each -> {after:.1}"
        );
    }

    #[test]
    fn agents_do_not_all_end_up_with_the_same_number_of_neighbours() {
        // The other half of the same problem. In an evenly spaced grid every
        // agent has almost exactly the same number of neighbours. In a real
        // flock some agents sit in a crowd and others in open space, so the
        // counts vary a lot.
        //
        // Measured: about 0.47 for a real flock, 0.17 for the evenly spaced
        // version.
        let params = Params::default();
        let end = run(&scattered_swarm(800, &params), 600, &params);

        let counts: Vec<f64> = neighbour_counts(&end, &params)
            .iter()
            .map(|count| *count as f64)
            .collect();
        let average = counts.iter().sum::<f64>() / counts.len() as f64;
        let spread = (counts.iter().map(|c| (c - average).powi(2)).sum::<f64>()
            / counts.len() as f64)
            .sqrt()
            / average;

        assert!(
            spread > 0.30,
            "agents ended up evenly spaced: variation {spread:.2}"
        );
    }

    #[test]
    fn agents_end_up_agreeing_with_the_agents_around_them() {
        // Several separate flocks normally form, each heading its own way, so
        // the overall alignment number stays low. What must be high is how well
        // each agent agrees with the ones it can actually see.
        let params = Params::default();
        let start = scattered_swarm(800, &params);
        let end = run(&start, 600, &params);
        let before = local_alignment(&start, &params);
        let after = local_alignment(&end, &params);
        assert!(
            after > 0.9 && after > before,
            "agents never agreed with their neighbours: {before:.3} -> {after:.3}"
        );
    }

    #[test]
    fn a_dense_swarm_becomes_more_flocked_over_time() {
        // The whole point of the model. Needs a crowded world: with agents too
        // far apart to see each other, nothing can happen.
        let params = Params {
            world: Vector2D::new(200.0, 200.0),
            ..Params::default()
        };
        let start = scattered_swarm(500, &params);
        let end = run(&start, 300, &params);
        let before = polarisation(&start);
        let after = polarisation(&end);
        assert!(
            after > before + 0.3,
            "swarm did not flock: {before:.3} -> {after:.3}"
        );
    }
}
