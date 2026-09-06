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

use std::path::PathBuf;
use std::time::Instant;

use constants::ROOT_RANK;
use mpi::collective::SystemOperation;
use mpi::traits::*;
use swarm_core::{
    Agent, DEFAULT_SWARM_SIZE, Params, Partition, Recorder, Timings, agents_to_send_left,
    agents_to_send_right, configuration_report, decode_from_numbers, encode_to_numbers,
    scattered_swarm, sort_agents_by_destination, state_fingerprint, step_with_ghosts,
    timing_report,
};

/// Where the swarm size sits on the command line. Index 0 is the program itself.
const ARG_SWARM_SIZE: usize = 1;
/// Where the number of steps sits on the command line.
const ARG_STEPS: usize = 2;
/// How many steps to run when the command line doesn't say.
const DEFAULT_STEPS: u64 = 600;
/// How often to print a progress line.
const REPORT_EVERY: u64 = 100;
/// How many steps to skip between saved snapshots, when saving.
const DEFAULT_RECORD_EVERY: u64 = 5;

fn main() {
    let universe = mpi::initialize().expect("MPI failed to initialise");
    let world = universe.world();
    let rank = world.rank();
    let process_count = world.size();

    let params = Params::default();
    let swarm_size = numeric_argument(ARG_SWARM_SIZE).unwrap_or(DEFAULT_SWARM_SIZE);
    let steps = numeric_argument(ARG_STEPS).unwrap_or(DEFAULT_STEPS);
    let record_every = flag_value("--every")
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_RECORD_EVERY);

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

    // Runs that measure overall speed leave the extra barrier out; runs that
    // measure where the time goes put it in. See `swarm_core::output::timing`.
    let measure_phases = std::env::args().any(|argument| argument == "--measure-phases");

    // Recording is off unless asked for, so measurement runs pay nothing.
    // Only the root process writes: it gathers everyone's agents so the file
    // holds one whole swarm per step, the same shape the sequential runner
    // produces.
    let mut recorder = flag_value("--dump").map(PathBuf::from).map(|path| {
        let recorder = Recorder::create(&path, record_every, &params)
            .unwrap_or_else(|error| panic!("could not write to {}: {error}", path.display()));
        if rank == ROOT_RANK {
            println!("recording to {} every {record_every} steps", path.display());
        }
        recorder
    });
    record_everyones_agents(&world, rank, recorder.as_mut(), 0, &mine);

    // The copies our agents will look at in the first step. After that, each
    // step ends by preparing the copies the next one needs.
    let mut ghosts = swap_border_agents(&world, &partition, &mine, &params);
    let mut timings = Timings::default();
    let started_run = Instant::now();

    for current_step in 1..=steps {
        // 1. The simulation work itself.
        let started = Instant::now();
        mine = step_with_ghosts(&mine, &ghosts, &params);
        timings.computing += started.elapsed();

        // 2. Wait for everyone to finish computing. This is what stops time
        //    lost to uneven load from being counted as communication.
        if measure_phases {
            let started = Instant::now();
            world.barrier();
            timings.waiting_for_others += started.elapsed();
        }

        // 3. Hand over anyone who crossed a border, then prepare the copies for
        //    the next step.
        let started = Instant::now();
        mine = hand_over_agents(&world, &partition, mine, &params);
        ghosts = swap_border_agents(&world, &partition, &mine, &params);
        timings.communicating += started.elapsed();

        // 4. Nobody starts the next step until everyone has finished this one.
        let started = Instant::now();
        world.barrier();
        timings.finishing_together += started.elapsed();

        record_everyones_agents(&world, rank, recorder.as_mut(), current_step, &mine);

        if current_step.is_multiple_of(REPORT_EVERY) || current_step == steps {
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

    if let Some(recorder) = recorder {
        recorder.finish().expect("could not finish the recording");
    }

    let wall_clock = started_run.elapsed();
    let slowest = slowest_across_processes(&world, &timings);

    // The slowest process's own total. This is the number to compare against
    // the sequential runner's "simulating": both cover the simulation and
    // nothing else, so progress lines and recording cannot flatter either side.
    let my_total = timings.total().as_secs_f64();
    let mut simulating = 0.0;
    world.all_reduce_into(&my_total, &mut simulating, SystemOperation::max());
    if rank == ROOT_RANK {
        println!();
        println!(
            "  simulating        {:.3}s   ({:.3}ms per step)",
            simulating,
            1000.0 * simulating / steps.max(1) as f64
        );
        println!(
            "  wall clock        {:.3}s   (including progress lines and recording)",
            wall_clock.as_secs_f64()
        );
        if !measure_phases {
            println!("  (pass --measure-phases to see where the time went)");
        }
        println!();
        print!("{}", timing_report(&slowest, steps));
        if measure_phases {
            println!(
                "  (each part is the slowest process's figure, so they add up to\n   \
                 more than the wall clock — different processes are slowest at\n   \
                 different parts)"
            );
        }
    }

    report_fingerprint(&world, rank, &mine);
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

/// Gathers everyone's agents onto the root process so it can write one file.
///
/// Each group is written with the number of the process that owns it, so a
/// drawing can colour agents by owner — which makes hand-overs at the borders,
/// and strips filling up unevenly, visible at a glance.
///
/// Costs a round of messages, so it only happens on steps that are being
/// recorded, and only when recording was asked for at all.
fn record_everyones_agents(
    world: &mpi::topology::SimpleCommunicator,
    rank: i32,
    recorder: Option<&mut Recorder>,
    step_number: u64,
    mine: &[Agent],
) {
    let Some(recorder) = recorder else {
        return;
    };
    if !recorder.is_recording_step(step_number) {
        return;
    }

    if rank != ROOT_RANK {
        world
            .process_at_rank(ROOT_RANK)
            .send(&encode_to_numbers(mine)[..]);
        return;
    }

    recorder
        .record(step_number, mine, ROOT_RANK as usize)
        .expect("could not record a step");
    for other in 0..world.size() {
        if other == ROOT_RANK {
            continue;
        }
        let (numbers, _status) = world.process_at_rank(other).receive_vec::<f64>();
        recorder
            .record(step_number, &decode_from_numbers(&numbers), other as usize)
            .expect("could not record a step");
    }
}

/// The slowest process's time for each part.
///
/// Every process has its own four numbers. What matters is the slowest, because
/// at every barrier everyone else is waiting for it — the run only goes as fast
/// as its slowest part.
fn slowest_across_processes(
    world: &mpi::topology::SimpleCommunicator,
    timings: &Timings,
) -> Timings {
    let slowest = |part: std::time::Duration| {
        let mine = part.as_secs_f64();
        let mut worst = 0.0;
        world.all_reduce_into(&mine, &mut worst, SystemOperation::max());
        std::time::Duration::from_secs_f64(worst)
    };
    Timings {
        computing: slowest(timings.computing),
        waiting_for_others: slowest(timings.waiting_for_others),
        communicating: slowest(timings.communicating),
        finishing_together: slowest(timings.finishing_together),
    }
}

/// Collects every process's agents onto the root process and prints one number
/// summarising the exact final state.
///
/// The sequential runner prints the same number. If they match, splitting the
/// work across processes changed nothing at all — which is the first thing this
/// project set out to show.
fn report_fingerprint(world: &mpi::topology::SimpleCommunicator, rank: i32, mine: &[Agent]) {
    let mut everyone: Vec<Agent> = Vec::new();
    if rank == ROOT_RANK {
        everyone.extend_from_slice(mine);
        for other in 0..world.size() {
            if other == ROOT_RANK {
                continue;
            }
            let (numbers, _status) = world.process_at_rank(other).receive_vec::<f64>();
            everyone.extend(decode_from_numbers(&numbers));
        }
        println!();
        println!("  fingerprint       {:016x}", state_fingerprint(&everyone));
    } else {
        let numbers = encode_to_numbers(mine);
        world.process_at_rank(ROOT_RANK).send(&numbers[..]);
    }
}

/// Passes agents that walked out of our strip to whoever owns where they now
/// stand, and takes in the ones that walked into ours.
///
/// Happens after the step, once positions are settled — and well apart from the
/// border copies, which are made before the step and thrown away after it. An
/// agent is therefore never both copied and handed over in the same step, which
/// would leave two processes each believing they owned it.
fn hand_over_agents(
    world: &mpi::topology::SimpleCommunicator,
    partition: &Partition,
    mine: Vec<Agent>,
    params: &Params,
) -> Vec<Agent> {
    let sorted = sort_agents_by_destination(&mine, partition, params);
    if partition.process_count() == 1 {
        return sorted.staying;
    }

    let left = partition.left_neighbour() as i32;
    let right = partition.right_neighbour() as i32;
    let going_left = encode_to_numbers(&sorted.going_left);
    let going_right = encode_to_numbers(&sorted.going_right);

    let mut kept = sorted.staying;
    for (destination, source, outgoing) in [(left, right, &going_left), (right, left, &going_right)]
    {
        let arrived: Vec<f64> = mpi::request::scope(|scope| {
            let sent = world
                .process_at_rank(destination)
                .immediate_send(scope, &outgoing[..]);
            let (numbers, _status) = world.process_at_rank(source).receive_vec::<f64>();
            sent.wait();
            numbers
        });
        kept.extend(decode_from_numbers(&arrived));
    }
    kept
}

/// Sends our edge agents to the two processes next door and collects theirs.
///
/// Two rounds. First everyone sends leftwards and receives from the right, then
/// everyone sends rightwards and receives from the left. Doing it in rounds is
/// what keeps two processes from both waiting on each other.
///
/// The sends are non-blocking: a plain send would sit waiting for the other
/// side to be ready to receive, and if every process did that at the same
/// moment nothing would ever move.
fn swap_border_agents(
    world: &mpi::topology::SimpleCommunicator,
    partition: &Partition,
    mine: &[Agent],
    params: &Params,
) -> Vec<Agent> {
    // On its own, a process is its own neighbour and already sees everything.
    if partition.process_count() == 1 {
        return Vec::new();
    }

    let left = partition.left_neighbour() as i32;
    let right = partition.right_neighbour() as i32;

    let going_left = encode_to_numbers(&agents_to_send_left(mine, partition, params));
    let going_right = encode_to_numbers(&agents_to_send_right(mine, partition, params));

    let mut ghosts = Vec::new();
    for (destination, source, outgoing) in [(left, right, &going_left), (right, left, &going_right)]
    {
        let arrived: Vec<f64> = mpi::request::scope(|scope| {
            let sent = world
                .process_at_rank(destination)
                .immediate_send(scope, &outgoing[..]);
            let (numbers, _status) = world.process_at_rank(source).receive_vec::<f64>();
            sent.wait();
            numbers
        });
        ghosts.extend(decode_from_numbers(&arrived));
    }
    ghosts
}

/// Reads the value that follows a named flag, as in `--dump run.csv`.
fn flag_value(name: &str) -> Option<String> {
    let arguments: Vec<String> = std::env::args().collect();
    let position = arguments.iter().position(|argument| argument == name)?;
    arguments.get(position + 1).cloned()
}

/// Reads one number from a fixed position on the command line.
fn numeric_argument(position: usize) -> Option<u64> {
    std::env::args().nth(position)?.parse().ok()
}
