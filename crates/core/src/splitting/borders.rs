//! Sharing copies of agents across strip boundaries.
//!
//! An agent standing near the edge of its strip has neighbours belonging to the
//! process next door. It cannot see them, so it would steer on incomplete
//! information and the flock would show seams at the boundaries.
//!
//! The fix: before each step, every process sends its neighbours a copy of the
//! agents standing in a band along its edges. Those copies are read-only — a
//! process uses them to work out how its own agents should steer, and never
//! moves them. They are called *ghosts*, and the technique is known as ghost
//! cells.
//!
//! No MPI in this file. Deciding *which* agents to send is arithmetic; actually
//! sending them is the distributed runner's job. Keeping them apart means the
//! whole scheme can be tested inside a single process.

use crate::{Agent, Params, Partition};

/// Agents close enough to this strip's left edge that the process to the left
/// needs to see them.
///
/// The band is as wide as an agent can see. Any narrower and agents just over
/// the boundary would miss neighbours they should have found, which is exactly
/// the difference from the sequential run that must not exist.
pub fn agents_to_send_left(mine: &[Agent], partition: &Partition, params: &Params) -> Vec<Agent> {
    let band_ends = partition.left_edge() + params.perception_radius;
    mine.iter()
        .filter(|agent| agent.position.x < band_ends)
        .copied()
        .collect()
}

/// Agents close enough to this strip's right edge that the process to the right
/// needs to see them.
pub fn agents_to_send_right(mine: &[Agent], partition: &Partition, params: &Params) -> Vec<Agent> {
    let band_starts = partition.right_edge() - params.perception_radius;
    mine.iter()
        .filter(|agent| agent.position.x >= band_starts)
        .copied()
        .collect()
}

/// How many numbers one agent takes up when flattened for sending.
const NUMBERS_PER_AGENT: usize = 5;

/// Flattens agents into a plain list of numbers, ready to be sent.
///
/// MPI sends arrays of simple numbers, not Rust types. Flattening here rather
/// than in the distributed runner keeps `swarm-core` free of MPI while still
/// having one definition of the wire format that both sides share.
///
/// The id becomes an `f64`, which stores whole numbers exactly up to about
/// 9,000,000,000,000,000 — far beyond any swarm this will ever run.
pub fn encode_to_numbers(agents: &[Agent]) -> Vec<f64> {
    let mut numbers = Vec::with_capacity(agents.len() * NUMBERS_PER_AGENT);
    for agent in agents {
        numbers.push(agent.id as f64);
        numbers.push(agent.position.x);
        numbers.push(agent.position.y);
        numbers.push(agent.velocity.x);
        numbers.push(agent.velocity.y);
    }
    numbers
}

/// Rebuilds agents from the numbers that arrived.
pub fn decode_from_numbers(numbers: &[f64]) -> Vec<Agent> {
    assert!(
        numbers.len().is_multiple_of(NUMBERS_PER_AGENT),
        "received {} numbers, which is not a whole number of agents",
        numbers.len()
    );
    numbers
        .chunks(NUMBERS_PER_AGENT)
        .map(|chunk| Agent {
            id: chunk[0] as u64,
            position: crate::Vector2D::new(chunk[1], chunk[2]),
            velocity: crate::Vector2D::new(chunk[3], chunk[4]),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Vector2D, scattered_swarm, step, step_with_ghosts};

    /// Runs one step the way the processes would, but all inside this one
    /// process: split the swarm, work out what each strip sends its
    /// neighbours, step each strip with the copies it received, then put the
    /// results back together.
    ///
    /// This is why the border logic lives in `swarm-core` rather than in the
    /// distributed runner. The whole scheme can be checked against the
    /// sequential result with an ordinary test, no `mpirun` involved.
    fn one_step_split_across(
        process_count: usize,
        agents: &[Agent],
        params: &Params,
    ) -> Vec<Agent> {
        let strips: Vec<Partition> = (0..process_count)
            .map(|number| Partition::for_process(number, process_count, params))
            .collect();
        let owned: Vec<Vec<Agent>> = strips
            .iter()
            .map(|strip| strip.agents_inside(agents, params))
            .collect();

        let mut result = Vec::new();
        for (number, strip) in strips.iter().enumerate() {
            let mut ghosts = Vec::new();
            if process_count > 1 {
                // What the process on the left sends across its right edge,
                // and what the process on the right sends across its left edge.
                ghosts.extend(agents_to_send_right(
                    &owned[strip.left_neighbour()],
                    &strips[strip.left_neighbour()],
                    params,
                ));
                ghosts.extend(agents_to_send_left(
                    &owned[strip.right_neighbour()],
                    &strips[strip.right_neighbour()],
                    params,
                ));
            }
            result.extend(step_with_ghosts(&owned[number], &ghosts, params));
        }
        result.sort_by_key(|agent| agent.id);
        result
    }

    #[test]
    fn splitting_the_work_gives_exactly_the_same_answer() {
        // The point of the whole exercise. Splitting the world between
        // processes must not change the simulation at all — not even in the
        // last decimal place.
        let params = Params::default();
        let agents = scattered_swarm(1000, &params);
        let sequential = step(&agents, &params);

        for process_count in [1usize, 2, 3, 4, 8, 20] {
            let split = one_step_split_across(process_count, &agents, &params);
            assert_eq!(
                split, sequential,
                "splitting across {process_count} processes changed the result"
            );
        }
    }

    #[test]
    fn without_the_border_copies_the_answer_is_wrong() {
        // Confirms the copies are doing something. Step each strip with no
        // ghosts at all and the agents near the boundaries steer differently.
        let params = Params::default();
        let agents = scattered_swarm(1000, &params);
        let sequential = step(&agents, &params);

        let mut blind = Vec::new();
        for number in 0..4 {
            let strip = Partition::for_process(number, 4, &params);
            let mine = strip.agents_inside(&agents, &params);
            blind.extend(step_with_ghosts(&mine, &[], &params));
        }
        blind.sort_by_key(|agent| agent.id);
        assert_ne!(blind, sequential, "the border copies changed nothing");
    }

    #[test]
    fn the_band_is_as_wide_as_an_agent_can_see() {
        let params = Params::default();
        let strip = Partition::for_process(1, 4, &params); // 250..500
        let agents = vec![
            Agent {
                id: 0,
                position: Vector2D::new(251.0, 0.0),
                velocity: Vector2D::ZERO,
            },
            Agent {
                id: 1,
                position: Vector2D::new(299.0, 0.0),
                velocity: Vector2D::ZERO,
            },
            Agent {
                id: 2,
                position: Vector2D::new(301.0, 0.0),
                velocity: Vector2D::ZERO,
            },
            Agent {
                id: 3,
                position: Vector2D::new(499.0, 0.0),
                velocity: Vector2D::ZERO,
            },
        ];
        // Seeing distance is 50, so the left band is 250..300.
        let left: Vec<u64> = agents_to_send_left(&agents, &strip, &params)
            .iter()
            .map(|agent| agent.id)
            .collect();
        assert_eq!(left, vec![0, 1]);
        // and the right band is 450..500.
        let right: Vec<u64> = agents_to_send_right(&agents, &strip, &params)
            .iter()
            .map(|agent| agent.id)
            .collect();
        assert_eq!(right, vec![3]);
    }

    #[test]
    fn agents_survive_being_flattened_and_rebuilt() {
        let params = Params::default();
        let agents = scattered_swarm(50, &params);
        assert_eq!(decode_from_numbers(&encode_to_numbers(&agents)), agents);
    }

    #[test]
    fn flattening_nothing_gives_nothing() {
        assert!(encode_to_numbers(&[]).is_empty());
        assert!(decode_from_numbers(&[]).is_empty());
    }
}
