//! Splitting the world into squares so agents only compare against nearby ones.
//!
//! The slow search compares every agent against every other one. With 1000
//! agents that is a million comparisons every step, and almost all of them
//! answer "too far away".
//!
//! Instead, divide the world into squares at least as wide as an agent can see.
//! Then any agent close enough to matter must be in the same square or one of
//! the eight around it, so each agent only checks a handful of others rather
//! than all of them.
//!
//! The squares are also how the work gets split between machines later, so the
//! same structure does both jobs.

use crate::{Agent, Neighbour, Params, toroidal_delta};

/// The world divided into squares, with each agent filed into one.
pub struct Grid {
    columns: usize,
    rows: usize,
    cell_width: f64,
    cell_height: f64,
    /// For each square, the positions in the agent list of the agents in it.
    /// Storing positions rather than copies keeps this cheap to build.
    cells: Vec<Vec<usize>>,
}

impl Grid {
    /// Files every agent into a square.
    ///
    /// Rebuilt every step, because the agents have moved. That costs one pass
    /// over the list, which is nothing next to what it saves.
    pub fn build(agents: &[Agent], params: &Params) -> Grid {
        // Squares must be at least as wide as an agent can see, or agents close
        // enough to matter could sit two squares away and be missed. Rounding
        // the count down makes each square that little bit bigger than the
        // seeing distance, never smaller.
        let columns = ((params.world.x / params.perception_radius).floor() as usize).max(1);
        let rows = ((params.world.y / params.perception_radius).floor() as usize).max(1);
        let cell_width = params.world.x / columns as f64;
        let cell_height = params.world.y / rows as f64;

        let mut cells = vec![Vec::new(); columns * rows];
        for (position_in_list, agent) in agents.iter().enumerate() {
            let column = ((agent.position.x / cell_width) as usize).min(columns - 1);
            let row = ((agent.position.y / cell_height) as usize).min(rows - 1);
            cells[row * columns + column].push(position_in_list);
        }

        Grid {
            columns,
            rows,
            cell_width,
            cell_height,
            cells,
        }
    }

    /// Every agent within seeing distance of `agent`, not counting itself.
    ///
    /// Gives exactly the same answer as the slow search, in the same order.
    /// That is the point: the fast version has to be indistinguishable from the
    /// obvious one, or none of the measurements mean anything.
    pub fn neighbours_of(
        &self,
        agent: &Agent,
        agents: &[Agent],
        params: &Params,
    ) -> Vec<Neighbour> {
        let radius_squared = params.perception_radius * params.perception_radius;
        let column = ((agent.position.x / self.cell_width) as usize).min(self.columns - 1);
        let row = ((agent.position.y / self.cell_height) as usize).min(self.rows - 1);

        let mut neighbours = Vec::new();
        for cell in self.surrounding_cells(column, row) {
            for &position_in_list in &self.cells[cell] {
                let other = &agents[position_in_list];
                if other.id == agent.id {
                    continue;
                }
                let offset = toroidal_delta(agent.position, other.position, params.world);
                if offset.len_sq() <= radius_squared {
                    neighbours.push(Neighbour {
                        id: other.id,
                        offset,
                        velocity: other.velocity,
                    });
                }
            }
        }
        // Same fixed order as the slow search: adding decimals in a different
        // order gives slightly different answers.
        neighbours.sort_by_key(|neighbour| neighbour.id);
        neighbours
    }

    /// The squares to search: this one and the eight around it, wrapping at the
    /// edges of the world.
    ///
    /// Returns each square once. That matters when the world is only one or two
    /// squares across — wrapping would otherwise land on the same square more
    /// than once and every agent in it would be counted twice.
    fn surrounding_cells(&self, column: usize, row: usize) -> Vec<usize> {
        let mut found: Vec<usize> = Vec::with_capacity(9);
        for row_offset in [self.rows - 1, 0, 1] {
            for column_offset in [self.columns - 1, 0, 1] {
                let wrapped_row = (row + row_offset) % self.rows;
                let wrapped_column = (column + column_offset) % self.columns;
                let cell = wrapped_row * self.columns + wrapped_column;
                if !found.contains(&cell) {
                    found.push(cell);
                }
            }
        }
        found
    }

    /// How many squares the world was divided into, across and down.
    pub fn shape(&self) -> (usize, usize) {
        (self.columns, self.rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Vector2D, find_neighbours, run, scattered_swarm, step, step_slowly};

    /// Worlds worth checking, including awkward ones: a world only one square
    /// across, one exactly two squares across, and a world that is not square.
    fn awkward_worlds() -> Vec<Params> {
        vec![
            Params::default(),
            Params {
                world: Vector2D::new(60.0, 60.0),
                perception_radius: 50.0,
                ..Params::default()
            },
            Params {
                world: Vector2D::new(110.0, 110.0),
                perception_radius: 50.0,
                ..Params::default()
            },
            Params {
                world: Vector2D::new(300.0, 120.0),
                perception_radius: 25.0,
                ..Params::default()
            },
            Params {
                world: Vector2D::new(1000.0, 1000.0),
                perception_radius: 8.0,
                ..Params::default()
            },
        ]
    }

    #[test]
    fn the_grid_finds_exactly_what_the_slow_search_finds() {
        // The whole point. The slow search is simple enough to trust, so if
        // these disagree, the grid is wrong.
        for params in awkward_worlds() {
            let agents = scattered_swarm(400, &params);
            let grid = Grid::build(&agents, &params);
            for agent in &agents {
                assert_eq!(
                    grid.neighbours_of(agent, &agents, &params),
                    find_neighbours(agent, &agents, &params),
                    "disagreed for agent {} in a {} x {} world",
                    agent.id,
                    params.world.x,
                    params.world.y
                );
            }
        }
    }

    #[test]
    fn the_grid_finds_neighbours_across_the_edge_of_the_world() {
        // Agents on opposite edges are really neighbours, and they sit in
        // squares at opposite ends of the grid.
        let params = Params {
            world: Vector2D::new(200.0, 200.0),
            perception_radius: 20.0,
            ..Params::default()
        };
        let agents = vec![
            Agent {
                id: 0,
                position: Vector2D::new(1.0, 100.0),
                velocity: Vector2D::new(1.0, 0.0),
            },
            Agent {
                id: 1,
                position: Vector2D::new(199.0, 100.0),
                velocity: Vector2D::new(1.0, 0.0),
            },
        ];
        let grid = Grid::build(&agents, &params);
        let found = grid.neighbours_of(&agents[0], &agents, &params);
        assert_eq!(found.len(), 1, "should see through the wrap-around edge");
        assert_eq!(found[0].id, 1);
    }

    #[test]
    fn a_world_only_one_square_across_does_not_count_agents_twice() {
        // With one square, all nine directions wrap onto the same square. Every
        // agent would be counted nine times if the squares were not deduplicated.
        let params = Params {
            world: Vector2D::new(60.0, 60.0),
            perception_radius: 50.0,
            ..Params::default()
        };
        let agents = scattered_swarm(30, &params);
        let grid = Grid::build(&agents, &params);
        assert_eq!(grid.shape(), (1, 1));
        for agent in &agents {
            let found = grid.neighbours_of(agent, &agents, &params);
            let mut ids: Vec<u64> = found.iter().map(|neighbour| neighbour.id).collect();
            let before = ids.len();
            ids.dedup();
            assert_eq!(before, ids.len(), "agent {} was counted twice", agent.id);
        }
    }

    #[test]
    fn squares_are_never_smaller_than_an_agent_can_see() {
        // If a square were narrower than the seeing distance, an agent close
        // enough to matter could sit two squares away and be missed.
        for params in awkward_worlds() {
            let grid = Grid::build(&[], &params);
            let (columns, rows) = grid.shape();
            assert!(params.world.x / columns as f64 >= params.perception_radius - 1e-9);
            assert!(params.world.y / rows as f64 >= params.perception_radius - 1e-9);
        }
    }

    #[test]
    fn the_fast_and_slow_simulations_stay_identical_step_after_step() {
        // Agreeing once is not enough. Any difference at all, even in the last
        // decimal place, would grow over time and make the two versions drift
        // apart — which is exactly the failure the distributed version must
        // avoid, so it has to hold here first.
        let params = Params::default();
        let mut fast = scattered_swarm(300, &params);
        let mut slow = fast.clone();
        for step_number in 0..200 {
            fast = step(&fast, &params);
            slow = step_slowly(&slow, &params);
            assert_eq!(fast, slow, "drifted apart at step {step_number}");
        }
    }

    #[test]
    fn the_grid_gives_the_same_answer_every_time() {
        let params = Params::default();
        let start = scattered_swarm(200, &params);
        assert_eq!(run(&start, 50, &params), run(&start, 50, &params));
    }
}
