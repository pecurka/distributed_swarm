//! How big the world should be for a run.

use crate::Vector2D;
use crate::world::constants::{DEFAULT_SWARM_SIZE, DEFAULT_WORLD};

/// The world size that keeps the swarm as crowded as the default setup.
///
/// The default is 1000 agents in a 1000x1000 world. Adding agents without
/// growing the world does not make the swarm bigger, it makes it denser: each
/// agent ends up with more neighbours to look at, so the work grows with the
/// square of the swarm size instead of in step with it. That also means
/// "bigger swarm" and "more crowded swarm" cannot be told apart, and they are
/// different questions.
///
/// Growing the world alongside the swarm keeps the number of neighbours each
/// agent has roughly fixed, so the swarm size measures how big the problem is
/// and nothing else. Because it is the *area* that has to grow in step with
/// the swarm, the side of a square world grows with the square root: ten times
/// the agents needs a world about 3.2 times wider.
pub fn world_for_constant_density(swarm_size: u64) -> Vector2D {
    let reference_area = DEFAULT_WORLD.x * DEFAULT_WORLD.y;
    let area = reference_area * swarm_size as f64 / DEFAULT_SWARM_SIZE as f64;
    let side = area.sqrt();
    Vector2D::new(side, side)
}

/// Decides which world a run should use.
///
/// Two ways to ask for one, because they answer different questions:
///
/// - `named_size` sets the side of a square world explicitly, for when a
///   particular size is wanted (`--world 4000` on the command line).
/// - `constant_density` picks the world from the swarm size so the swarm is
///   as crowded as the default, which is what makes swarm size mean "bigger
///   problem" rather than "more crowded". See [`world_for_constant_density`].
///
/// Asking for neither leaves the world at its default, which keeps the swarm
/// in a fixed 1000x1000 box however many agents are in it.
///
/// A named size wins if both are given, since naming a size is the more
/// specific request. Returns an explanation rather than a world when the
/// request does not make sense, so the caller can complain and stop instead
/// of quietly simulating something nobody asked for.
///
/// Reading the command line is left to each runner. This only makes the
/// decision, so the model never needs to know where the request came from.
pub fn choose_world(
    named_size: Option<f64>,
    constant_density: bool,
    swarm_size: u64,
    perception_radius: f64,
) -> Result<Vector2D, String> {
    let world = match named_size {
        Some(side) => {
            if !side.is_finite() || side <= 0.0 {
                return Err(format!("--world needs a size above zero, not {side}"));
            }
            Vector2D::new(side, side)
        }
        None if constant_density => world_for_constant_density(swarm_size),
        None => DEFAULT_WORLD,
    };

    // A world narrower than an agent can see leaves the whole simulation
    // inside a single grid cell, and every agent able to see every other one
    // through the wrap-around. Nothing about that is worth measuring, so say
    // so rather than produce a number.
    if world.x < perception_radius {
        return Err(format!(
            "a world of {:.1} is smaller than an agent can see ({perception_radius:.1}), \
             so every agent would see every other one",
            world.x
        ));
    }
    Ok(world)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Agents per unit of area. This is the thing being held still.
    fn density(swarm_size: u64, world: Vector2D) -> f64 {
        swarm_size as f64 / (world.x * world.y)
    }

    #[test]
    fn the_default_swarm_size_gives_back_the_default_world() {
        assert_eq!(
            world_for_constant_density(DEFAULT_SWARM_SIZE),
            DEFAULT_WORLD
        );
    }

    #[test]
    fn density_stays_the_same_however_big_the_swarm_gets() {
        // The whole point of the function. If this drifts, a "weak scaling"
        // measurement is quietly measuring crowding instead.
        let reference = density(DEFAULT_SWARM_SIZE, DEFAULT_WORLD);
        for swarm_size in [1, 500, 1_000, 16_000, 100_000, 1_000_000] {
            let world = world_for_constant_density(swarm_size);
            let got = density(swarm_size, world);
            assert!(
                (got - reference).abs() < 1e-12,
                "{swarm_size} agents gave density {got}, expected {reference}"
            );
        }
    }

    #[test]
    fn the_world_is_square() {
        let world = world_for_constant_density(123_456);
        assert_eq!(world.x, world.y);
    }

    #[test]
    fn ten_times_the_agents_needs_a_world_about_three_times_wider() {
        // Area grows with the swarm, so the side grows with the square root.
        let small = world_for_constant_density(1_000);
        let large = world_for_constant_density(10_000);
        assert!((large.x / small.x - 10f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn asking_for_nothing_leaves_the_world_at_its_default() {
        let world = choose_world(None, false, 1000, 50.0).unwrap();
        assert_eq!(world, DEFAULT_WORLD);
    }

    #[test]
    fn a_named_world_is_used_as_given() {
        let world = choose_world(Some(4000.0), false, 1_000_000, 50.0).unwrap();
        assert_eq!(world, Vector2D::new(4000.0, 4000.0));
    }

    #[test]
    fn constant_density_picks_the_world_from_the_swarm_size() {
        let world = choose_world(None, true, 10_000, 50.0).unwrap();
        assert_eq!(world, world_for_constant_density(10_000));
    }

    #[test]
    fn naming_a_world_beats_asking_for_constant_density() {
        // Both given, so the more specific request wins. Whichever rule is
        // chosen it must be predictable, or a sweep silently measures
        // something other than what the script asked for.
        let world = choose_world(Some(2000.0), true, 1_000_000, 50.0).unwrap();
        assert_eq!(world, Vector2D::new(2000.0, 2000.0));
    }

    #[test]
    fn nonsense_worlds_are_refused_rather_than_guessed_at() {
        for bad in [0.0, -5.0, f64::NAN, f64::INFINITY, 10.0] {
            assert!(
                choose_world(Some(bad), false, 1000, 50.0).is_err(),
                "a world of {bad} should have been refused"
            );
        }
    }

    #[test]
    fn a_million_agents_asks_for_a_world_of_about_thirty_two_thousand() {
        // The largest configuration the sweep runs.
        let world = world_for_constant_density(1_000_000);
        assert!(
            (world.x - 31_622.776_601_683_79).abs() < 1e-6,
            "got {}",
            world.x
        );
    }
}
