//! Measuring where a step's time actually goes.
//!
//! A step is split into four parts, and keeping them apart is the whole point:
//!
//! - **computing** — the simulation work itself
//! - **waiting for others** — time lost because some processes had more work
//!   than others
//! - **communicating** — moving data between processes
//! - **finishing together** — the barrier that closes the step
//!
//! Separating the first two from the third is harder than it looks. If you
//! simply time the communication, a process that finishes early sits inside its
//! receive call waiting for a slower neighbour to send — and that waiting gets
//! counted as communication. Communication then looks expensive and uneven load
//! looks free, so the conclusion about what actually limits scaling comes out
//! backwards.
//!
//! The fix is an extra barrier after computing and before communicating. It
//! soaks up the waiting, so what is left in the communication measurement is
//! really data movement. That barrier costs a little real speed, so runs that
//! measure overall speed leave it out and runs that measure the breakdown put
//! it in — and the thesis says which is which.

use std::time::Duration;

/// How long each part of a run took, added up over all its steps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Timings {
    pub computing: Duration,
    pub waiting_for_others: Duration,
    pub communicating: Duration,
    pub finishing_together: Duration,
}

impl Timings {
    pub fn total(&self) -> Duration {
        self.computing + self.waiting_for_others + self.communicating + self.finishing_together
    }

    /// The share of the total that was not simulation work.
    ///
    /// This is the price of splitting the job up. When it approaches 1, adding
    /// processes has stopped helping.
    pub fn overhead_share(&self) -> f64 {
        let total = self.total().as_secs_f64();
        if total == 0.0 {
            return 0.0;
        }
        1.0 - self.computing.as_secs_f64() / total
    }

    /// Adds another set of timings onto this one.
    pub fn add(&mut self, other: &Timings) {
        self.computing += other.computing;
        self.waiting_for_others += other.waiting_for_others;
        self.communicating += other.communicating;
        self.finishing_together += other.finishing_together;
    }
}

/// The four numbers as a readable block.
///
/// When these are the slowest process's figure for each part, the "added up"
/// row can come out larger than the run actually took. That is not a mistake:
/// a different process can be the slowest at computing than at communicating,
/// so the four worst cases never all happen to the same process. Read the parts
/// for where the time goes, and the wall clock for how long it took.
pub fn timing_report(timings: &Timings, steps: u64) -> String {
    let total = timings.total();
    let share = |part: Duration| {
        if total.is_zero() {
            0.0
        } else {
            100.0 * part.as_secs_f64() / total.as_secs_f64()
        }
    };
    let milliseconds_per_step = |part: Duration| 1000.0 * part.as_secs_f64() / steps.max(1) as f64;

    let mut report = String::new();
    report.push_str("  where the time went      total      per step     share\n");
    for (name, part) in [
        ("computing", timings.computing),
        ("waiting for others", timings.waiting_for_others),
        ("communicating", timings.communicating),
        ("finishing together", timings.finishing_together),
    ] {
        report.push_str(&format!(
            "  {name:<20} {:>8.3}s   {:>8.3}ms   {:>5.1}%\n",
            part.as_secs_f64(),
            milliseconds_per_step(part),
            share(part)
        ));
    }
    report.push_str(&format!(
        "  {:<20} {:>8.3}s   {:>8.3}ms\n",
        "added up",
        total.as_secs_f64(),
        milliseconds_per_step(total)
    ));
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn milliseconds(count: u64) -> Duration {
        Duration::from_millis(count)
    }

    #[test]
    fn the_total_is_the_four_parts_added_up() {
        let timings = Timings {
            computing: milliseconds(100),
            waiting_for_others: milliseconds(20),
            communicating: milliseconds(30),
            finishing_together: milliseconds(50),
        };
        assert_eq!(timings.total(), milliseconds(200));
    }

    #[test]
    fn overhead_share_is_everything_that_was_not_simulation_work() {
        let timings = Timings {
            computing: milliseconds(75),
            waiting_for_others: milliseconds(10),
            communicating: milliseconds(10),
            finishing_together: milliseconds(5),
        };
        assert!((timings.overhead_share() - 0.25).abs() < 1e-12);
    }

    #[test]
    fn a_run_that_did_nothing_has_no_overhead() {
        // Guards against dividing by zero before anything has been timed.
        assert_eq!(Timings::default().overhead_share(), 0.0);
    }

    #[test]
    fn adding_timings_adds_each_part_separately() {
        let mut running = Timings {
            computing: milliseconds(10),
            ..Timings::default()
        };
        running.add(&Timings {
            computing: milliseconds(5),
            communicating: milliseconds(3),
            ..Timings::default()
        });
        assert_eq!(running.computing, milliseconds(15));
        assert_eq!(running.communicating, milliseconds(3));
    }

    #[test]
    fn the_report_names_every_part_and_survives_zero_steps() {
        let report = timing_report(&Timings::default(), 0);
        for expected in ["computing", "waiting for others", "communicating", "total"] {
            assert!(report.contains(expected), "missing {expected:?}");
        }
    }
}
