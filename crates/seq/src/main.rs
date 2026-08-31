//! Sequential runner.
//!
//! Builds a swarm, runs it, and reports how flocked it became. With `--dump` it
//! also saves positions to a file so the run can be drawn afterwards.
//!
//!     swarm-seq [agents] [steps] [--dump FILE] [--every N]

use std::path::PathBuf;

use swarm_core::{
    DEFAULT_SWARM_SIZE, Params, Recorder, configuration_report, local_alignment, polarisation,
    progress_heading, progress_line, scattered_swarm, step,
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
            .record(0, &agents)
            .expect("could not record step 0");
    }

    for current_step in 1..=steps {
        agents = step(&agents, &params);

        if let Some(recorder) = recorder.as_mut() {
            recorder
                .record(current_step, &agents)
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

    if let Some(recorder) = recorder {
        recorder.finish().expect("could not finish the recording");
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
