//! Writing one row per run to a results file.
//!
//! A sweep runs the same configuration several times and many configurations in
//! turn, so results have to accumulate somewhere a script can read back. Each
//! run appends a row; nothing is ever overwritten.
//!
//! Every row carries the fingerprint of the swarm it ended with. That is the
//! evidence that a fast result and a slow one were the same simulation — a
//! timing is meaningless if the run it came from was computing something else.

use std::fs::OpenOptions;
use std::io::{Result, Write};
use std::path::Path;
use std::time::Duration;

use crate::Timings;

/// Everything worth recording about one run.
pub struct RunResult<'a> {
    /// `"sequential"` or `"distributed"`.
    pub runner: &'a str,
    pub processes: usize,
    pub agents: u64,
    pub steps: u64,
    /// The simulation and nothing else. This is the number speedup is worked
    /// out from.
    pub simulating: Duration,
    /// Everything the program did, including progress lines and recording.
    pub wall_clock: Duration,
    /// Where the time went. All zero for the sequential runner, which has no
    /// communicating or waiting to do.
    pub timings: Timings,
    /// Whether the extra barrier was in, which makes the breakdown meaningful
    /// but costs a little speed.
    pub measured_phases: bool,
    /// Agents held by the emptiest and busiest process at the end. Equal for a
    /// sequential run.
    pub smallest_process_load: usize,
    pub largest_process_load: usize,
    /// How uneven the load was, averaged over the whole run, and at its worst.
    ///
    /// The busiest process divided by the average, so 1.0 is perfectly even.
    /// Measured all the way through rather than only at the end: a run that
    /// stays balanced until the last moment and one that goes bad immediately
    /// look identical at the finish line, and they cost completely different
    /// amounts.
    pub average_imbalance: f64,
    pub worst_imbalance: f64,
    /// Identifies the exact final state, so rows can be checked against each
    /// other.
    pub fingerprint: u64,
}

const HEADER: &str = "runner,processes,agents,steps,simulating_seconds,wall_clock_seconds,\
computing_seconds,waiting_seconds,communicating_seconds,finishing_seconds,measured_phases,\
smallest_process_load,largest_process_load,average_imbalance,worst_imbalance,fingerprint";

/// Adds one row, writing the header first if the file is new.
pub fn append_result(path: &Path, result: &RunResult) -> Result<()> {
    let is_new = !path.exists();
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    if is_new {
        writeln!(file, "{HEADER}")?;
    }
    writeln!(file, "{}", format_row(result))?;
    Ok(())
}

/// One row, without writing it anywhere. Split out so it can be tested.
pub fn format_row(result: &RunResult) -> String {
    format!(
        "{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{},{:.4},{:.4},{:016x}",
        result.runner,
        result.processes,
        result.agents,
        result.steps,
        result.simulating.as_secs_f64(),
        result.wall_clock.as_secs_f64(),
        result.timings.computing.as_secs_f64(),
        result.timings.waiting_for_others.as_secs_f64(),
        result.timings.communicating.as_secs_f64(),
        result.timings.finishing_together.as_secs_f64(),
        result.measured_phases,
        result.smallest_process_load,
        result.largest_process_load,
        result.average_imbalance,
        result.worst_imbalance,
        result.fingerprint
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn example() -> RunResult<'static> {
        RunResult {
            runner: "distributed",
            processes: 4,
            agents: 1000,
            steps: 300,
            simulating: Duration::from_millis(1500),
            wall_clock: Duration::from_millis(1600),
            timings: Timings {
                computing: Duration::from_millis(1000),
                waiting_for_others: Duration::from_millis(400),
                communicating: Duration::from_millis(80),
                finishing_together: Duration::from_millis(20),
            },
            measured_phases: true,
            smallest_process_load: 180,
            largest_process_load: 340,
            average_imbalance: 1.42,
            worst_imbalance: 2.11,
            fingerprint: 0x475a_1e24_4ee7_5967,
        }
    }

    #[test]
    fn a_row_has_one_field_for_every_column() {
        let columns = HEADER.split(',').count();
        let fields = format_row(&example()).split(',').count();
        assert_eq!(fields, columns, "row and header do not line up");
    }

    #[test]
    fn the_fingerprint_is_written_the_same_way_the_runners_print_it() {
        assert!(format_row(&example()).contains("475a1e244ee75967"));
    }

    #[test]
    fn the_header_is_written_once_and_rows_accumulate() {
        let path = std::env::temp_dir().join("swarm-results-test.csv");
        fs::remove_file(&path).ok();

        append_result(&path, &example()).unwrap();
        append_result(&path, &example()).unwrap();

        let written = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = written.lines().collect();
        assert_eq!(lines.len(), 3, "one header and two rows");
        assert_eq!(lines[0], HEADER);
        assert_eq!(lines[1], lines[2]);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn timings_are_written_with_enough_precision_to_be_useful() {
        // Steps take under a millisecond at small swarm sizes, so rounding to
        // milliseconds would throw the measurement away.
        let mut result = example();
        result.simulating = Duration::from_micros(1234);
        assert!(format_row(&result).contains("0.001234"));
    }
}
