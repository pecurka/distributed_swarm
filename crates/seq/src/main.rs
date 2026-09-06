//! Sequential runner.
//!
//! Builds a swarm, runs it, and reports how flocked it became. With `--dump` it
//! also saves positions to a file so the run can be drawn afterwards.
//!
//!     swarm-seq [agents] [steps] [--dump FILE] [--every N]

use std::path::PathBuf;
use std::time::Instant;

use swarm_core::{
    DEFAULT_SWARM_SIZE, Params, Recorder, RunResult, Timings, append_result, configuration_report,
    local_alignment, polarisation, progress_heading, progress_line, scattered_swarm,
    state_fingerprint, step,
};

/// Where the swarm size sits on the command line. Index 0 is the program itself.
const ARG_SWARM_SIZE: usize = 1;
/// Where the number of steps sits on the command line.
const ARG_STEPS: usize = 2;

/// How many steps to run when the command line doesn't say.
const DEFAULT_STEPS: u64 = 600;
/// How often to print a progress line.
const REPORT_EVERY: u64 = 50;
/// How many steps to skip between saved snapshots, when saving.
const DEFAULT_RECORD_EVERY: u64 = 5;

fn main() {
    let params = Params::default();
    let swarm_size = numeric_argument(ARG_SWARM_SIZE).unwrap_or(DEFAULT_SWARM_SIZE);
    let steps = numeric_argument(ARG_STEPS).unwrap_or(DEFAULT_STEPS);
    let record_every = flag_value("--every")
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_RECORD_EVERY);

    let mut agents = scattered_swarm(swarm_size, &params);

    // Only opened if asked for, so ordinary runs pay nothing for it.
    let mut recorder = flag_value("--dump").map(PathBuf::from).map(|path| {
        let recorder = Recorder::create(&path, record_every, &params)
            .unwrap_or_else(|error| panic!("could not write to {}: {error}", path.display()));
        println!("recording to {} every {record_every} steps", path.display());
        recorder
    });

    println!(
        "{}",
        configuration_report("sequential baseline", agents.len(), steps, &params)
    );
    println!();

    // Two numbers, not one. See `progress_line` for why reading only the
    // overall figure is misleading.
    println!("{}", progress_heading());
    println!(
        "{}",
        progress_line(0, local_alignment(&agents, &params), polarisation(&agents))
    );

    if let Some(recorder) = recorder.as_mut() {
        recorder
            .record(0, &agents, 0)
            .expect("could not record step 0");
    }

    // Only the simulation is timed. The progress lines and any recording sit
    // outside it: this number is the baseline every distributed speedup is
    // divided by, so anything that is not the simulation must stay out of it.
    let mut simulating = std::time::Duration::ZERO;
    let started_run = Instant::now();

    for current_step in 1..=steps {
        let started_step = Instant::now();
        agents = step(&agents, &params);
        simulating += started_step.elapsed();

        if let Some(recorder) = recorder.as_mut() {
            recorder
                .record(current_step, &agents, 0)
                .expect("could not record a step");
        }
        if current_step % REPORT_EVERY == 0 || current_step == steps {
            // `local_alignment` searches for every agent's neighbours, so it
            // costs about as much as a simulation step. Fine every 50 steps
            // now, but it must stay outside anything we time later.
            println!(
                "{}",
                progress_line(
                    current_step,
                    local_alignment(&agents, &params),
                    polarisation(&agents)
                )
            );
        }
    }

    let wall_clock = started_run.elapsed();

    if let Some(recorder) = recorder {
        recorder.finish().expect("could not finish the recording");
    }

    // The baseline every distributed run is measured against. Note this
    // includes the flocking numbers printed along the way, which cost about as
    // much as a step each — so record without `--dump` and compare like with
    // like.
    println!();
    println!(
        "  simulating        {:.3}s   ({:.3}ms per step)",
        simulating.as_secs_f64(),
        1000.0 * simulating.as_secs_f64() / steps.max(1) as f64
    );
    println!(
        "  wall clock        {:.3}s   (including progress lines and recording)",
        wall_clock.as_secs_f64()
    );

    // One number summarising the exact final state. The distributed runner
    // prints the same thing, so the two can be compared directly.
    let fingerprint = state_fingerprint(&agents);
    println!();
    println!("  fingerprint       {fingerprint:016x}");

    if let Some(path) = flag_value("--results") {
        append_result(
            std::path::Path::new(&path),
            &RunResult {
                runner: "sequential",
                processes: 1,
                agents: agents.len() as u64,
                steps,
                simulating,
                wall_clock,
                // A single process has nobody to wait for or talk to.
                timings: Timings {
                    computing: simulating,
                    ..Timings::default()
                },
                measured_phases: false,
                smallest_process_load: agents.len(),
                largest_process_load: agents.len(),
                fingerprint,
            },
        )
        .unwrap_or_else(|error| panic!("could not write results to {path}: {error}"));
    }
}

/// Reads one number from a fixed position on the command line.
fn numeric_argument(position: usize) -> Option<u64> {
    std::env::args().nth(position)?.parse().ok()
}

/// Reads the value that follows a named flag, as in `--dump run.csv`.
fn flag_value(name: &str) -> Option<String> {
    let arguments: Vec<String> = std::env::args().collect();
    let position = arguments.iter().position(|argument| argument == name)?;
    arguments.get(position + 1).cloned()
}
