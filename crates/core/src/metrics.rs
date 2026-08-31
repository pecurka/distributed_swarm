//! Numbers that describe the state of the whole swarm.

use crate::{Agent, Params, Vector2D, find_neighbours};

/// How flocked the swarm is, from 0 to 1.
///
/// 0 means everyone is heading in random directions and the directions cancel
/// out. 1 means everyone is flying in formation.
///
/// It works by pointing every agent's direction at length 1, averaging them,
/// and measuring how long the average is. Directions that disagree cancel;
/// directions that agree add up.
///
/// This is how you tell the simulation is working without staring at
/// coordinates. It is also the fallback way of comparing the sequential and
/// distributed runs, if matching them number-for-number turns out not to work.
pub fn polarisation(agents: &[Agent]) -> f64 {
    if agents.is_empty() {
        return 0.0;
    }
    let mut total = Vector2D::ZERO;
    for agent in agents {
        total = total + agent.velocity.normalised();
    }
    (total * (1.0 / agents.len() as f64)).len()
}

/// How well each agent agrees with the agents it can actually see, 0 to 1.
///
/// This is usually the number to read, not [`polarisation`]. A swarm normally
/// settles into several separate flocks, each tidy inside but heading its own
/// way — that is correct behaviour, and it drags the overall number down while
/// leaving this one high.
///
/// High here with low overall means several flocks. Low on both means something
/// is wrong.
pub fn local_alignment(agents: &[Agent], params: &Params) -> f64 {
    let mut total = 0.0;
    let mut counted = 0;
    for agent in agents {
        let neighbours = find_neighbours(agent, agents, params);
        if neighbours.is_empty() {
            continue;
        }
        let mut sum = Vector2D::ZERO;
        for neighbour in &neighbours {
            sum = sum + neighbour.velocity.normalised();
        }
        total += (sum * (1.0 / neighbours.len() as f64)).len();
        counted += 1;
    }
    if counted == 0 {
        0.0
    } else {
        total / counted as f64
    }
}

/// How many neighbours each agent has.
///
/// Alignment measures whether agents point the same way. This measures whether
/// they are actually together, which is a different question — a swarm spread
/// evenly across the world can be perfectly aligned and still not be a flock.
/// Both numbers are needed to say the simulation is behaving.
pub fn neighbour_counts(agents: &[Agent], params: &Params) -> Vec<usize> {
    agents
        .iter()
        .map(|agent| find_neighbours(agent, agents, params).len())
        .collect()
}

/// Average number of neighbours per agent. Rises sharply when a swarm clumps.
pub fn average_neighbour_count(agents: &[Agent], params: &Params) -> f64 {
    if agents.is_empty() {
        return 0.0;
    }
    let counts = neighbour_counts(agents, params);
    counts.iter().sum::<usize>() as f64 / counts.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Params;

    fn agent_going(id: u64, velocity: Vector2D) -> Agent {
        Agent {
            id,
            position: Vector2D::ZERO,
            velocity,
        }
    }

    #[test]
    fn everyone_heading_the_same_way_scores_one() {
        // Different speeds, same direction — only direction should count.
        let agents = vec![
            agent_going(0, Vector2D::new(1.0, 0.0)),
            agent_going(1, Vector2D::new(4.0, 0.0)),
        ];
        assert!((polarisation(&agents) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn opposite_directions_cancel_to_zero() {
        let agents = vec![
            agent_going(0, Vector2D::new(1.0, 0.0)),
            agent_going(1, Vector2D::new(-1.0, 0.0)),
        ];
        assert!(polarisation(&agents) < 1e-12);
    }

    #[test]
    fn four_directions_at_right_angles_cancel_to_zero() {
        let agents = vec![
            agent_going(0, Vector2D::new(1.0, 0.0)),
            agent_going(1, Vector2D::new(-1.0, 0.0)),
            agent_going(2, Vector2D::new(0.0, 1.0)),
            agent_going(3, Vector2D::new(0.0, -1.0)),
        ];
        assert!(polarisation(&agents) < 1e-12);
    }

    #[test]
    fn an_empty_swarm_scores_zero() {
        assert_eq!(polarisation(&[]), 0.0);
    }

    #[test]
    fn local_alignment_is_one_when_close_agents_agree() {
        let params = Params {
            world: Vector2D::new(100.0, 100.0),
            perception_radius: 20.0,
            ..Params::default()
        };
        let agents = vec![
            Agent { id: 0, position: Vector2D::new(50.0, 50.0), velocity: Vector2D::new(1.0, 0.0) },
            Agent { id: 1, position: Vector2D::new(55.0, 50.0), velocity: Vector2D::new(3.0, 0.0) },
        ];
        assert!((local_alignment(&agents, &params) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_alignment_ignores_agents_with_nobody_near_them() {
        // A lone agent has nothing to agree or disagree with, so it must not
        // drag the average down.
        let params = Params {
            world: Vector2D::new(1000.0, 1000.0),
            perception_radius: 20.0,
            ..Params::default()
        };
        let agents = vec![
            Agent { id: 0, position: Vector2D::new(50.0, 50.0), velocity: Vector2D::new(1.0, 0.0) },
            Agent { id: 1, position: Vector2D::new(55.0, 50.0), velocity: Vector2D::new(1.0, 0.0) },
            Agent { id: 2, position: Vector2D::new(900.0, 900.0), velocity: Vector2D::new(0.0, 1.0) },
        ];
        assert!((local_alignment(&agents, &params) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn neighbour_counts_are_reported_per_agent() {
        let params = Params {
            world: Vector2D::new(1000.0, 1000.0),
            perception_radius: 20.0,
            ..Params::default()
        };
        let agents = vec![
            Agent { id: 0, position: Vector2D::new(50.0, 50.0), velocity: Vector2D::ZERO },
            Agent { id: 1, position: Vector2D::new(55.0, 50.0), velocity: Vector2D::ZERO },
            Agent { id: 2, position: Vector2D::new(900.0, 900.0), velocity: Vector2D::ZERO },
        ];
        assert_eq!(neighbour_counts(&agents, &params), vec![1, 1, 0]);
        assert!((average_neighbour_count(&agents, &params) - 2.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn an_empty_swarm_has_no_neighbours() {
        let params = Params::default();
        assert_eq!(average_neighbour_count(&[], &params), 0.0);
        assert_eq!(local_alignment(&[], &params), 0.0);
    }
}
