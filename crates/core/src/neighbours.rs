//! Finding the agents near a given agent.
//!
//! This is the slow, obvious version: it compares every agent against every
//! other one. With 1000 agents that is a million comparisons per step.
//!
//! It is deliberately kept once the fast grid version exists, but only as a
//! checker — never as the thing we measure speed against. It is simple enough
//! to trust, so when the fast version disagrees with it, the fast version is
//! wrong.

use crate::{Agent, Params, Vector2D, toroidal_delta};

/// One agent seen from another agent's point of view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Neighbour {
    pub id: u64,
    /// Shortest path from the looking agent to this one, already accounting
    /// for the world wrapping around.
    pub offset: Vector2D,
    pub velocity: Vector2D,
}

/// Every agent within seeing distance of `agent`, not counting itself.
///
/// The result is sorted by id. That is not cosmetic: the steering rules add
/// these up, and adding decimals in a different order gives slightly different
/// answers. The distributed version gathers its neighbours from two places (its
/// own agents and copies from the next machine), so it would naturally end up
/// with a different order. Sorting makes both versions agree exactly.
pub fn find_neighbours(agent: &Agent, agents: &[Agent], params: &Params) -> Vec<Neighbour> {
    let radius_squared = params.perception_radius * params.perception_radius;
    let mut neighbours: Vec<Neighbour> = agents
        .iter()
        .filter(|other| other.id != agent.id)
        .filter_map(|other| {
            let offset = toroidal_delta(agent.position, other.position, params.world);
            if offset.len_sq() <= radius_squared {
                Some(Neighbour {
                    id: other.id,
                    offset,
                    velocity: other.velocity,
                })
            } else {
                None
            }
        })
        .collect();
    neighbours.sort_by_key(|neighbour| neighbour.id);
    neighbours
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vector2D;

    fn agent_at(id: u64, x: f64, y: f64) -> Agent {
        Agent {
            id,
            position: Vector2D::new(x, y),
            velocity: Vector2D::new(1.0, 0.0),
        }
    }

    fn small_world() -> Params {
        Params {
            world: Vector2D::new(100.0, 100.0),
            perception_radius: 10.0,
            ..Params::default()
        }
    }

    #[test]
    fn finds_agents_within_seeing_distance() {
        let params = small_world();
        let agents = vec![agent_at(0, 50.0, 50.0), agent_at(1, 55.0, 50.0)];
        let found = find_neighbours(&agents[0], &agents, &params);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, 1);
        assert_eq!(found[0].offset, Vector2D::new(5.0, 0.0));
    }

    #[test]
    fn ignores_agents_out_of_range() {
        let params = small_world();
        let agents = vec![agent_at(0, 50.0, 50.0), agent_at(1, 70.0, 50.0)];
        assert!(find_neighbours(&agents[0], &agents, &params).is_empty());
    }

    #[test]
    fn never_counts_the_agent_itself() {
        let params = small_world();
        let agents = vec![agent_at(0, 50.0, 50.0)];
        assert!(find_neighbours(&agents[0], &agents, &params).is_empty());
    }

    #[test]
    fn sees_across_the_edge_of_the_world() {
        // Two agents 4 apart, but on opposite sides of the map edge.
        let params = small_world();
        let agents = vec![agent_at(0, 98.0, 50.0), agent_at(1, 2.0, 50.0)];
        let found = find_neighbours(&agents[0], &agents, &params);
        assert_eq!(found.len(), 1, "should see through the wrap-around edge");
        assert_eq!(found[0].offset, Vector2D::new(4.0, 0.0));
    }

    #[test]
    fn results_come_back_sorted_by_id() {
        // Given out of order, so a missing sort would show up here. The order
        // is what keeps the sequential and distributed runs agreeing exactly.
        let params = small_world();
        let agents = vec![
            agent_at(7, 52.0, 50.0),
            agent_at(2, 53.0, 50.0),
            agent_at(9, 54.0, 50.0),
            agent_at(0, 50.0, 50.0),
        ];
        let me = agents[3];
        let found = find_neighbours(&me, &agents, &params);
        let ids: Vec<u64> = found.iter().map(|neighbour| neighbour.id).collect();
        assert_eq!(ids, vec![2, 7, 9]);
    }
}
