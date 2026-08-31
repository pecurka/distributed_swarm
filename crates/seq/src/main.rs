//! Sequential runner.
//!
//! Builds a swarm, runs it for a while, and reports how flocked it became.

use swarm_core::{DEFAULT_SWARM_SIZE, Params, polarisation, scattered_swarm, step};

/// Where the swarm size sits on the command line. Index 0 is the program itself.
const ARG_SWARM_SIZE: usize = 1;
/// Where the number of steps sits on the command line.
const ARG_STEPS: usize = 2;

/// How many steps to run when the command line doesn't say.
const DEFAULT_STEPS: u64 = 500;
/// How often to print a progress line.
const REPORT_EVERY: u64 = 50;

fn main() {
    let params = Params::default();
    let swarm_size = numeric_argument(ARG_SWARM_SIZE).unwrap_or(DEFAULT_SWARM_SIZE);
    let steps = numeric_argument(ARG_STEPS).unwrap_or(DEFAULT_STEPS);

    let mut agents = scattered_swarm(swarm_size, &params);

    println!("distributed_swarm — sequential baseline");
    println!("  agents            {}", agents.len());
    println!("  steps             {steps}");
    println!(
        "  world             {} x {} (wraps around)",
        params.world.x, params.world.y
    );
    println!("  perception radius {}", params.perception_radius);
    println!(
        "  weights           separation {} alignment {} cohesion {}",
        params.weight_separation, params.weight_alignment, params.weight_cohesion
    );
    println!("  max speed         {}", params.max_speed);
    println!("  timestep          {}", params.timestep);
    println!();
    println!("  step   flocking");
    println!("  {:>4}   {:.3}", 0, polarisation(&agents));

    for current_step in 1..=steps {
        agents = step(&agents, &params);
        if current_step % REPORT_EVERY == 0 || current_step == steps {
            println!("  {current_step:>4}   {:.3}", polarisation(&agents));
        }
    }
}

/// Reads one number from the command line, if it is there and makes sense.
fn numeric_argument(position: usize) -> Option<u64> {
    std::env::args().nth(position)?.parse().ok()
}
