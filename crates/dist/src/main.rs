//! Distributed runner.
//!
//! The world is cut into vertical strips, one per process. Each process looks
//! after the agents standing in its own strip and simulates only those.
//!
//!     mpirun -n 4 ./target/release/swarm-dist [agents] [steps]
//!
//! Not finished. Agents near a strip's edge have neighbours belonging to the
//! process next door and cannot see them yet, so they steer wrongly and the
//! flock shows seams at the boundaries. Swapping copies of edge agents between
//! neighbours is the next piece of work.

mod constants;

use constants::ROOT_RANK;
use mpi::collective::SystemOperation;
use mpi::traits::*;
use swarm_core::{
    DEFAULT_SWARM_SIZE, Params, Partition, configuration_report, scattered_swarm, step,
};

/// Where the swarm size sits on the command line. Index 0 is the program itself.
const ARG_SWARM_SIZE: usize = 1;
/// Where the number of steps sits on the command line.
const ARG_STEPS: usize = 2;
/// How many steps to run when the command line doesn't say.
const DEFAULT_STEPS: u64 = 600;
/// How often to print a progress line.
const REPORT_EVERY: u64 = 100;

fn main() {
    let universe = mpi::initialize().expect("MPI failed to initialise");
    let world = universe.world();
    let rank = world.rank();
    let process_count = world.size();

    let params = Params::default();
    let swarm_size = numeric_argument(ARG_SWARM_SIZE).unwrap_or(DEFAULT_SWARM_SIZE);
    let steps = numeric_argument(ARG_STEPS).unwrap_or(DEFAULT_STEPS);

    let partition = Partition::for_process(rank as usize, process_count as usize, &params);

    // A strip narrower than an agent can see cannot work once borders are being
    // swapped, so stop now rather than quietly simulating something wrong.
    if !partition.is_wide_enough(&params) {
        if rank == ROOT_RANK {
            eprintln!(
                "error: {process_count} processes makes each strip {:.1} wide, \
                 but an agent can see {:.1}.\n       \
                 This world can be split across at most {} processes.",
                partition.width(),
                params.perception_radius,
                Partition::most_strips_that_fit(&params)
            );
        }
        return;
    }

    // Every process builds the whole swarm and then keeps only its own agents.
    // Wasteful, but it guarantees all processes start from identical state,
    // which is what makes the comparison with the sequential run meaningful.
    // Building only the local part comes later, once it is worth the care.
    let everyone = scattered_swarm(swarm_size, &params);
    let mut mine = partition.agents_inside(&everyone, &params);

    if rank == ROOT_RANK {
        println!(
            "{}",
            configuration_report("distributed", everyone.len(), steps, &params)
        );
        println!("  processes         {process_count}");
        println!("  strip width       {:.1}", partition.width());
        println!();
        println!("  step   agents per process (min / average / max)   strayed");
    }

    report_spread(&world, rank, 0, &mine, &partition, &params, swarm_size);

    for current_step in 1..=steps {
        // The same step function the sequential runner uses. It is simply
        // given fewer agents.
        mine = step(&mine, &params);

        if current_step % REPORT_EVERY == 0 || current_step == steps {
            report_spread(
                &world,
                rank,
                current_step,
                &mine,
                &partition,
                &params,
                swarm_size,
            );
        }
    }
}

/// Collects how many agents each process is holding and prints one line.
///
/// The gap between the busiest and the quietest process is the load imbalance —
/// the thing that decides how much time is wasted waiting at a barrier. It
/// starts out even and gets worse as the agents bunch into flocks.
///
/// Also checks nothing has been lost: the totals across all processes must
/// still add up to the swarm we started with.
fn report_spread(
    world: &mpi::topology::SimpleCommunicator,
    rank: i32,
    step_number: u64,
    agents: &[swarm_core::Agent],
    partition: &Partition,
    params: &Params,
    swarm_size: u64,
) {
    // Agents that have walked out of the strip that still owns them. Nothing
    // hands them over yet, so this only ever grows — and while it is above
    // zero, the counts above describe where agents started, not where they are.
    let strayed = agents
        .iter()
        .filter(|agent| !partition.owns_agent(agent, params))
        .count() as i32;
    let mut strayed_total = 0;
    world.all_reduce_into(&strayed, &mut strayed_total, SystemOperation::sum());

    let mine = agents.len() as i32;
    let mut total = 0;
    let mut smallest = 0;
    let mut largest = 0;
    world.all_reduce_into(&mine, &mut total, SystemOperation::sum());
    world.all_reduce_into(&mine, &mut smallest, SystemOperation::min());
    world.all_reduce_into(&mine, &mut largest, SystemOperation::max());

    if rank != ROOT_RANK {
        return;
    }

    assert_eq!(
        total as u64, swarm_size,
        "agents went missing at step {step_number}: {total} of {swarm_size}"
    );

    let average = total as f64 / world.size() as f64;
    let imbalance = largest as f64 / average;
    println!(
        "  {step_number:>4}   {smallest:>5} / {average:>7.1} / {largest:>5}   \
         {strayed_total:>5}   (busiest {imbalance:.2}x average)"
    );
}

/// Reads one number from a fixed position on the command line.
fn numeric_argument(position: usize) -> Option<u64> {
    std::env::args().nth(position)?.parse().ok()
}
