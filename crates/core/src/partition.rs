//! Cutting the world into strips, one per process.
//!
//! Each process takes a vertical strip of the world and looks after the agents
//! standing in it. Nobody is in charge: every process works out its own strip
//! from its number, and they all agree because they all do the same sum.
//!
//! Strips rather than a chessboard of blocks, to begin with. A strip has
//! exactly two neighbours — one to the left, one to the right — and no corners,
//! which makes the border swapping much simpler to get right. Blocks come later.
//!
//! There is no MPI in here on purpose. Working out who owns what is arithmetic,
//! so it can be tested normally instead of only when launched across several
//! processes.

use crate::{Agent, Params, Vector2D};

/// One process's strip of the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Partition {
    /// Counting from 0.
    process_number: usize,
    process_count: usize,
    world_width: f64,
}

impl Partition {
    /// Works out the strip belonging to process number `index`.
    ///
    /// Every process runs this with its own number and the same total, so they
    /// all end up with the same picture of who owns what without having to
    /// agree on it by talking.
    pub fn for_process(process_number: usize, process_count: usize, params: &Params) -> Partition {
        assert!(process_count > 0, "there must be at least one strip");
        assert!(
            process_number < process_count,
            "strip {process_number} does not exist out of {process_count}"
        );
        Partition {
            process_number,
            process_count,
            world_width: params.world.x,
        }
    }

    pub fn width(&self) -> f64 {
        self.world_width / self.process_count as f64
    }

    pub fn left_edge(&self) -> f64 {
        self.process_number as f64 * self.width()
    }

    /// An agent standing exactly here belongs to the next strip along, not
    /// this one.
    pub fn right_edge(&self) -> f64 {
        self.left_edge() + self.width()
    }

    /// Only the left-to-right position matters — strips run the full height of
    /// the world.
    pub fn find_owning_process_for_position(
        position: Vector2D,
        process_count: usize,
        params: &Params,
    ) -> usize {
        let width = params.world.x / process_count as f64;
        // `min` guards the very edge of the world, where rounding could
        // otherwise produce a strip number that does not exist.
        ((position.x / width) as usize).min(process_count - 1)
    }

    pub fn contains_position(&self, position: Vector2D, params: &Params) -> bool {
        Partition::find_owning_process_for_position(position, self.process_count, params)
            == self.process_number
    }

    /// Ownership is worked out from where the agent is standing, not stored on
    /// the agent. Two processes can never disagree about it, because they both
    /// do the same sum — and it updates itself the moment the agent moves.
    pub fn owns_agent(&self, agent: &Agent, params: &Params) -> bool {
        self.contains_position(agent.position, params)
    }

    pub fn agents_inside(&self, agents: &[Agent], params: &Params) -> Vec<Agent> {
        agents
            .iter()
            .filter(|agent| self.owns_agent(agent, params))
            .copied()
            .collect()
    }

    /// Neighbours wrap: the first strip and the last are next to each other.
    pub fn left_neighbour(&self) -> usize {
        (self.process_number + self.process_count - 1) % self.process_count
    }

    pub fn right_neighbour(&self) -> usize {
        (self.process_number + 1) % self.process_count
    }

    pub fn process_number(&self) -> usize {
        self.process_number
    }

    pub fn process_count(&self) -> usize {
        self.process_count
    }

    /// Whether the strips are wide enough to be worth using.
    ///
    /// Once borders are being swapped, each process sends its neighbours a
    /// band as wide as an agent can see. If a strip were narrower than that,
    /// that band would reach past the next process into the one after it, and
    /// swapping with immediate neighbours alone would no longer be enough.
    ///
    /// So there is a hard limit on how many processes a given world can be split
    /// across, and it is worth failing loudly rather than quietly simulating
    /// something wrong.
    pub fn is_wide_enough(&self, params: &Params) -> bool {
        self.width() >= params.perception_radius
    }

    pub fn most_strips_that_fit(params: &Params) -> usize {
        ((params.world.x / params.perception_radius).floor() as usize).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scattered_swarm;

    #[test]
    fn strips_divide_the_world_evenly() {
        let params = Params::default();
        let first = Partition::for_process(0, 4, &params);
        let last = Partition::for_process(3, 4, &params);
        assert_eq!(first.width(), 250.0);
        assert_eq!(first.left_edge(), 0.0);
        assert_eq!(first.right_edge(), 250.0);
        assert_eq!(last.left_edge(), 750.0);
        assert_eq!(last.right_edge(), 1000.0);
    }

    #[test]
    fn every_agent_belongs_to_exactly_one_strip() {
        // The invariant everything after this depends on. An agent owned twice
        // gets simulated twice; an agent owned by nobody disappears.
        let params = Params::default();
        let agents = scattered_swarm(1000, &params);
        for count in [1usize, 2, 3, 4, 7, 10] {
            let strips: Vec<Partition> = (0..count)
                .map(|index| Partition::for_process(index, count, &params))
                .collect();
            let mut owners = 0;
            for strip in &strips {
                owners += strip.agents_inside(&agents, &params).len();
            }
            assert_eq!(
                owners,
                agents.len(),
                "{count} strips owned {owners} of {} agents",
                agents.len()
            );
        }
    }

    #[test]
    fn an_agent_on_a_boundary_belongs_to_the_strip_on_its_right() {
        // Somebody has to own the exact boundary. Whichever rule is chosen, both
        // processes must apply the same one or the agent is owned twice or not
        // at all.
        let params = Params::default();
        let on_the_line = Vector2D::new(250.0, 500.0);
        assert_eq!(
            Partition::find_owning_process_for_position(on_the_line, 4, &params),
            1
        );
        assert!(!Partition::for_process(0, 4, &params).contains_position(on_the_line, &params));
        assert!(Partition::for_process(1, 4, &params).contains_position(on_the_line, &params));
    }

    #[test]
    fn the_far_edge_of_the_world_belongs_to_the_last_strip() {
        // Positions are always wrapped below the world width, but rounding at
        // the very edge must not invent a strip that does not exist.
        let params = Params::default();
        let almost_the_edge = Vector2D::new(999.9999, 0.0);
        assert_eq!(
            Partition::find_owning_process_for_position(almost_the_edge, 4, &params),
            3
        );
    }

    #[test]
    fn owning_an_agent_is_the_same_question_as_containing_its_position() {
        // The two must never disagree, which is why one calls the other rather
        // than repeating the sum.
        let params = Params::default();
        let agents = scattered_swarm(200, &params);
        for count in [1usize, 3, 4] {
            for number in 0..count {
                let strip = Partition::for_process(number, count, &params);
                for agent in &agents {
                    assert_eq!(
                        strip.owns_agent(agent, &params),
                        strip.contains_position(agent.position, &params)
                    );
                }
            }
        }
    }

    #[test]
    fn ownership_follows_an_agent_that_moves() {
        // Ownership is not stored anywhere, so moving an agent across a
        // boundary changes who owns it with nothing to update.
        let params = Params::default();
        let first = Partition::for_process(0, 4, &params);
        let mut agent = Agent {
            id: 0,
            position: Vector2D::new(100.0, 500.0),
            velocity: Vector2D::ZERO,
        };
        assert!(first.owns_agent(&agent, &params));
        agent.position = Vector2D::new(600.0, 500.0);
        assert!(!first.owns_agent(&agent, &params));
    }

    #[test]
    fn neighbours_wrap_around_the_world() {
        // The world is a loop, so the first strip and the last are neighbours.
        let params = Params::default();
        let first = Partition::for_process(0, 4, &params);
        let last = Partition::for_process(3, 4, &params);
        assert_eq!(first.left_neighbour(), 3);
        assert_eq!(first.right_neighbour(), 1);
        assert_eq!(last.right_neighbour(), 0);
    }

    #[test]
    fn a_single_strip_is_its_own_neighbour() {
        let params = Params::default();
        let only = Partition::for_process(0, 1, &params);
        assert_eq!(only.left_neighbour(), 0);
        assert_eq!(only.right_neighbour(), 0);
        assert_eq!(
            only.agents_inside(&scattered_swarm(50, &params), &params)
                .len(),
            50
        );
    }

    #[test]
    fn strips_narrower_than_an_agent_can_see_are_rejected() {
        // With a 1000-wide world and a seeing distance of 50, 20 strips is the
        // most that works. Beyond that a process's border band would reach past
        // its neighbour.
        let params = Params::default();
        assert_eq!(Partition::most_strips_that_fit(&params), 20);
        assert!(Partition::for_process(0, 20, &params).is_wide_enough(&params));
        assert!(!Partition::for_process(0, 21, &params).is_wide_enough(&params));
    }
}
