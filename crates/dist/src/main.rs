//! Distributed runner — a check that the toolchain works.
//!
//! It doesn't simulate anything yet. It just uses the three MPI features the
//! design needs, so a broken setup shows up now rather than later:
//!
//!   1. processes start and know which one they are
//!   2. sending data to a neighbour, the way ghost cells will
//!   3. waiting for everyone to catch up (a barrier)

mod constants;

use constants::{ROOT_RANK, UNKNOWN_PROCESSOR};
use mpi::collective::SystemOperation;
use mpi::traits::*;

fn main() {
    let universe = mpi::initialize().expect("MPI failed to initialise");
    let world = universe.world();
    let rank = world.rank();
    let size = world.size();

    let name = mpi::environment::processor_name().unwrap_or_else(|_| UNKNOWN_PROCESSOR.into());
    println!("rank {rank} of {size} on {name}");

    world.barrier();

    // Send to the next rank, receive from the previous one. This is the same
    // neighbour pattern the ghost exchange will use, so if it works here it
    // works there.
    let next_rank = (rank + 1) % size;
    let previous_rank = (rank + size - 1) % size;

    let payload = rank;
    let received: i32 = mpi::request::scope(|scope| {
        let send_request = world
            .process_at_rank(next_rank)
            .immediate_send(scope, &payload);
        let (message, _status) = world.process_at_rank(previous_rank).receive::<i32>();
        send_request.wait();
        message
    });

    assert_eq!(
        received, previous_rank,
        "rank {rank} expected {previous_rank} from its ring predecessor, got {received}"
    );

    // Add up a value from every rank. The timing harness will use this to
    // collect per-rank measurements.
    let mut sum: i32 = 0;
    world.all_reduce_into(&rank, &mut sum, SystemOperation::sum());
    let expected = (size - 1) * size / 2;
    assert_eq!(sum, expected, "all_reduce gave the wrong total");

    world.barrier();

    if rank == ROOT_RANK {
        println!();
        println!("ring exchange   ok ({size} ranks)");
        println!("all_reduce      ok (sum of ranks = {sum})");
        println!("barrier         ok");
        println!("MPI toolchain verified.");
    }
}
