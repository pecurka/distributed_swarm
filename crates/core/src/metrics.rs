//! Numbers that describe the state of the whole swarm.

use crate::{Agent, Vector2D};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
