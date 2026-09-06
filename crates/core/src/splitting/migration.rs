//! Handing agents over when they walk into the next strip.
//!
//! Agents move, so an agent that started in one strip will sooner or later
//! stand in another. Ownership has to move with it, or the process that started
//! with it keeps simulating an agent that is nowhere near it any more.
//!
//! This happens *after* the step, once the new positions are known, and it is
//! kept well apart from the border copies. Those copies are made *before* the
//! step, are read-only, and are thrown away at the end of it — they never
//! change who owns anything. Keeping the two apart is what stops an agent being
//! both copied and handed over in the same step, which would leave two
//! processes each thinking they own it.
//!
//! No MPI here either. Deciding who an agent should go to is arithmetic.

use crate::{Agent, Params, Partition};

/// This process's agents after a step, sorted by where they now belong.
#[derive(Debug, Clone, PartialEq)]
pub struct Destinations {
    /// Still inside this strip.
    pub staying: Vec<Agent>,
    /// Now belong to the process on the left.
    pub going_left: Vec<Agent>,
    /// Now belong to the process on the right.
    pub going_right: Vec<Agent>,
}

/// Works out which agents have walked out of this strip, and which way.
///
/// An agent can only reach a strip immediately next door in one step: it moves
/// at most `max_speed` per step, and a strip is never narrower than an agent
/// can see, which is far larger. If that ever stopped being true an agent could
/// skip a whole strip and be handed to the wrong process, so it is checked
/// rather than assumed.
pub fn sort_agents_by_destination(
    mine: &[Agent],
    partition: &Partition,
    params: &Params,
) -> Destinations {
    let mut destinations = Destinations {
        staying: Vec::new(),
        going_left: Vec::new(),
        going_right: Vec::new(),
    };

    for agent in mine {
        if partition.owns_agent(agent, params) {
            destinations.staying.push(*agent);
            continue;
        }
        let new_owner = Partition::find_owning_process_for_position(
            agent.position,
            partition.process_count(),
            params,
        );
        if new_owner == partition.right_neighbour() {
            destinations.going_right.push(*agent);
        } else if new_owner == partition.left_neighbour() {
            destinations.going_left.push(*agent);
        } else {
            panic!(
                "agent {} jumped from strip {} to strip {new_owner} in one step, \
                 skipping a strip entirely. Agents can only reach a strip next \
                 door — check that max speed is far smaller than the strip width.",
                agent.id,
                partition.process_number()
            );
        }
    }
    destinations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Vector2D, agents_to_send_left, agents_to_send_right, run, scattered_swarm, step_with_ghosts,
    };

    /// Runs the whole distributed algorithm inside this one process: split the
    /// swarm, swap border copies, step, hand over anyone who crossed, repeat.
    ///
    /// This is the check that matters. If it matches the sequential run step
    /// after step, the scheme is right — and it needs no `mpirun` to say so.
    fn run_split_across(
        process_count: usize,
        agents: &[Agent],
        steps: u64,
        params: &Params,
    ) -> Vec<Agent> {
        let strips: Vec<Partition> = (0..process_count)
            .map(|number| Partition::for_process(number, process_count, params))
            .collect();
        let mut owned: Vec<Vec<Agent>> = strips
            .iter()
            .map(|strip| strip.agents_inside(agents, params))
            .collect();

        for _ in 0..steps {
            // Copies of the agents along each border, made before anything moves.
            let mut stepped: Vec<Vec<Agent>> = Vec::with_capacity(process_count);
            for (number, strip) in strips.iter().enumerate() {
                let mut ghosts = Vec::new();
                if process_count > 1 {
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
                stepped.push(step_with_ghosts(&owned[number], &ghosts, params));
            }

            // Everyone has moved. Now hand over whoever crossed a border.
            let sorted: Vec<Destinations> = stepped
                .iter()
                .zip(strips.iter())
                .map(|(agents, strip)| sort_agents_by_destination(agents, strip, params))
                .collect();

            owned = sorted.iter().map(|s| s.staying.clone()).collect();
            for (number, strip) in strips.iter().enumerate() {
                owned[strip.left_neighbour()].extend(sorted[number].going_left.iter().copied());
                owned[strip.right_neighbour()].extend(sorted[number].going_right.iter().copied());
            }
        }

        let mut result: Vec<Agent> = owned.into_iter().flatten().collect();
        result.sort_by_key(|agent| agent.id);
        result
    }

    #[test]
    fn splitting_the_work_matches_the_sequential_run_step_after_step() {
        // The whole point of the project: splitting the world between processes
        // must not change the simulation at all, however long it runs.
        let params = Params::default();
        let agents = scattered_swarm(600, &params);
        let sequential = run(&agents, 200, &params);

        for process_count in [1usize, 2, 3, 4, 8] {
            let split = run_split_across(process_count, &agents, 200, &params);
            assert_eq!(
                split, sequential,
                "splitting across {process_count} processes changed the result"
            );
        }
    }

    #[test]
    fn no_agent_is_lost_or_duplicated_while_being_handed_over() {
        let params = Params::default();
        let agents = scattered_swarm(600, &params);
        for process_count in [2usize, 4, 8] {
            let after = run_split_across(process_count, &agents, 200, &params);
            assert_eq!(after.len(), agents.len(), "{process_count} processes");
            let ids: Vec<u64> = after.iter().map(|agent| agent.id).collect();
            let mut unique = ids.clone();
            unique.dedup();
            assert_eq!(ids, unique, "an agent was handed to two processes at once");
        }
    }

    #[test]
    fn agents_that_stayed_put_are_not_handed_anywhere() {
        let params = Params::default();
        let strip = Partition::for_process(1, 4, &params); // 250..500
        let agents = vec![Agent {
            id: 0,
            position: Vector2D::new(300.0, 100.0),
            velocity: Vector2D::ZERO,
        }];
        let sorted = sort_agents_by_destination(&agents, &strip, &params);
        assert_eq!(sorted.staying.len(), 1);
        assert!(sorted.going_left.is_empty());
        assert!(sorted.going_right.is_empty());
    }

    #[test]
    fn agents_go_to_the_side_they_walked_out_of() {
        let params = Params::default();
        let strip = Partition::for_process(1, 4, &params); // 250..500
        let agents = vec![
            Agent {
                id: 0,
                position: Vector2D::new(249.0, 0.0),
                velocity: Vector2D::ZERO,
            },
            Agent {
                id: 1,
                position: Vector2D::new(501.0, 0.0),
                velocity: Vector2D::ZERO,
            },
        ];
        let sorted = sort_agents_by_destination(&agents, &strip, &params);
        assert!(sorted.staying.is_empty());
        assert_eq!(sorted.going_left.len(), 1);
        assert_eq!(sorted.going_left[0].id, 0);
        assert_eq!(sorted.going_right.len(), 1);
        assert_eq!(sorted.going_right[0].id, 1);
    }

    #[test]
    fn handing_over_wraps_around_the_world() {
        // The first strip's left neighbour is the last one, right across the
        // world's edge.
        let params = Params::default();
        let first = Partition::for_process(0, 4, &params); // 0..250
        let agents = vec![Agent {
            id: 0,
            position: Vector2D::new(999.0, 0.0),
            velocity: Vector2D::ZERO,
        }];
        let sorted = sort_agents_by_destination(&agents, &first, &params);
        assert_eq!(sorted.going_left.len(), 1);
    }

    #[test]
    fn on_its_own_a_process_hands_nothing_over() {
        let params = Params::default();
        let only = Partition::for_process(0, 1, &params);
        let agents = scattered_swarm(50, &params);
        let sorted = sort_agents_by_destination(&agents, &only, &params);
        assert_eq!(sorted.staying.len(), 50);
        assert!(sorted.going_left.is_empty());
        assert!(sorted.going_right.is_empty());
    }
}
