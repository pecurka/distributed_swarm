//! Writing swarm positions to a file so they can be drawn later.
//!
//! Drawing never happens inside the simulation. The simulation writes a file,
//! and a separate program turns that into pictures. Three reasons:
//!
//! - drawing is slow, and doing it inside the part we time would mean timing
//!   the drawing instead of the simulation
//! - the model stays free of graphics libraries, so it still runs on a machine
//!   with no screen
//! - the distributed version can use exactly the same format, one file per
//!   machine, merged afterwards
//!
//! Recording is off unless asked for. Runs whose speed we care about write
//! nothing but their own timings.

use std::fs::File;
use std::io::{BufWriter, Result, Write};
use std::path::Path;

use crate::{Agent, Params};

/// Writes agent positions to a CSV file, every so many steps.
pub struct Recorder {
    /// Buffered on purpose: without this, every single line would go to the
    /// disk separately, which is the slowest possible way to do it.
    writer: BufWriter<File>,
    every: u64,
}

impl Recorder {
    /// Starts a new recording. `every` is how many steps to skip between
    /// snapshots — an animation looks smooth long before you save every one.
    pub fn create(path: &Path, every: u64, params: &Params) -> Result<Self> {
        let mut writer = BufWriter::new(File::create(path)?);
        // The drawing program needs to know how big the world is, and it
        // cannot tell from the positions alone.
        writeln!(writer, "# world {} {}", params.world.x, params.world.y)?;
        writeln!(writer, "step,id,x,y,velocity_x,velocity_y")?;
        Ok(Self {
            writer,
            every: every.max(1),
        })
    }

    /// Saves a snapshot, if this is a step we are saving.
    pub fn record(&mut self, step_number: u64, agents: &[Agent]) -> Result<()> {
        if !step_number.is_multiple_of(self.every) {
            return Ok(());
        }
        for agent in agents {
            writeln!(
                self.writer,
                "{},{},{:.3},{:.3},{:.3},{:.3}",
                step_number,
                agent.id,
                agent.position.x,
                agent.position.y,
                agent.velocity.x,
                agent.velocity.y
            )?;
        }
        Ok(())
    }

    /// Makes sure everything still sitting in the buffer reaches the disk.
    pub fn finish(mut self) -> Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scattered_swarm;
    use std::fs;

    fn temporary_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("swarm-recording-test-{name}.csv"))
    }

    #[test]
    fn writes_a_header_the_drawing_program_can_read() {
        let params = Params::default();
        let path = temporary_path("header");
        let recorder = Recorder::create(&path, 1, &params).expect("could not create");
        recorder.finish().expect("could not finish");

        let written = fs::read_to_string(&path).expect("could not read back");
        let mut lines = written.lines();
        assert_eq!(
            lines.next().unwrap(),
            format!("# world {} {}", params.world.x, params.world.y)
        );
        assert_eq!(lines.next().unwrap(), "step,id,x,y,velocity_x,velocity_y");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn writes_one_row_per_agent() {
        let params = Params::default();
        let agents = scattered_swarm(5, &params);
        let path = temporary_path("rows");

        let mut recorder = Recorder::create(&path, 1, &params).unwrap();
        recorder.record(0, &agents).unwrap();
        recorder.record(1, &agents).unwrap();
        recorder.finish().unwrap();

        let written = fs::read_to_string(&path).unwrap();
        let rows = written
            .lines()
            .filter(|line| line.starts_with(char::is_numeric));
        assert_eq!(rows.count(), 10, "two steps of five agents");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn skips_the_steps_it_was_told_to_skip() {
        let params = Params::default();
        let agents = scattered_swarm(2, &params);
        let path = temporary_path("skip");

        let mut recorder = Recorder::create(&path, 3, &params).unwrap();
        for step_number in 0..9 {
            recorder.record(step_number, &agents).unwrap();
        }
        recorder.finish().unwrap();

        let written = fs::read_to_string(&path).unwrap();
        let steps: Vec<&str> = written
            .lines()
            .filter(|line| line.starts_with(char::is_numeric))
            .map(|line| line.split(',').next().unwrap())
            .collect();
        // Steps 0, 3 and 6, two agents each.
        assert_eq!(steps, vec!["0", "0", "3", "3", "6", "6"]);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn an_interval_of_zero_does_not_divide_by_zero() {
        let params = Params::default();
        let path = temporary_path("zero");
        let mut recorder = Recorder::create(&path, 0, &params).unwrap();
        recorder.record(1, &scattered_swarm(1, &params)).unwrap();
        recorder.finish().unwrap();
        fs::remove_file(&path).ok();
    }
}
