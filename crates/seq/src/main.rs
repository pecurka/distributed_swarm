//! Sequential runner.
//!
//! Right now it just builds the starting swarm and prints the setup. The
//! neighbour grid and the steering rules come next.

use swarm_core::{DEFAULT_SWARM_SIZE, Params, lattice_swarm};

/// Where the swarm size sits on the command line. Index 0 is the program itself.
const ARG_SWARM_SIZE: usize = 1;

fn main() {
    let params = Params::default();
    let swarm_size: u64 = std::env::args()
        .nth(ARG_SWARM_SIZE)
        .and_then(|argument| argument.parse().ok())
        .unwrap_or(DEFAULT_SWARM_SIZE);

    let agents = lattice_swarm(swarm_size, &params);

    println!("distributed_swarm — sequential baseline");
    println!("  agents            {}", agents.len());
    println!(
        "  world             {} x {} (toroidal)",
        params.world.x, params.world.y
    );
    println!("  perception radius {}", params.perception_radius);
    println!(
        "  weights           sep {} align {} coh {}",
        params.weight_separation, params.weight_alignment, params.weight_cohesion
    );
    println!("  max speed         {}", params.max_speed);
    println!("  dt                {}", params.dt);
    println!();
    println!("first agent: {:?}", agents[0]);
    println!("last  agent: {:?}", agents[agents.len() - 1]);
}
