//! Printing what a run was set up to do, and how it is getting on.
//!
//! Both runners use these so their output looks exactly the same. That is not
//! tidiness: proving the sequential and distributed versions behave identically
//! is the first research question, and the easiest way to check is to run both
//! and compare the output line by line. That only works if the formatting
//! cannot drift apart.

use crate::Params;

/// The settings a run was given, as lines of text.
///
/// Worth printing on every run. When a result looks surprising six months from
/// now, this is what tells you which settings produced it.
pub fn configuration_report(label: &str, swarm_size: usize, steps: u64, params: &Params) -> String {
    let mut lines = String::new();
    lines.push_str(&format!("distributed_swarm — {label}\n"));
    lines.push_str(&format!("  agents            {swarm_size}\n"));
    lines.push_str(&format!("  steps             {steps}\n"));
    lines.push_str(&format!(
        "  world             {} x {} (wraps around)\n",
        params.world.x, params.world.y
    ));
    lines.push_str(&format!(
        "  perception radius {}\n",
        params.perception_radius
    ));
    lines.push_str(&format!(
        "  separation radius {}\n",
        params.separation_radius
    ));
    lines.push_str(&format!(
        "  weights           separation {} alignment {} cohesion {}\n",
        params.weight_separation, params.weight_alignment, params.weight_cohesion
    ));
    lines.push_str(&format!("  max speed         {}\n", params.max_speed));
    lines.push_str(&format!("  timestep          {}", params.timestep));
    lines
}

/// Column headings for the progress lines.
pub fn progress_heading() -> &'static str {
    "  step   nearby  overall"
}

/// One progress line.
///
/// Two numbers, not one. `nearby` is how well each agent agrees with the agents
/// it can see; `overall` is how well the whole swarm moves as one. A healthy
/// swarm normally has a high `nearby` and a middling `overall`, because several
/// separate flocks form and each heads its own way. Watching `overall` alone
/// will tell you the simulation is broken when it is fine, and fine when it is
/// broken.
pub fn progress_line(step: u64, nearby: f64, overall: f64) -> String {
    format!("  {step:>4}   {nearby:>6.3}  {overall:>7.3}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_names_every_setting_that_changes_the_result() {
        let params = Params::default();
        let report = configuration_report("test", 500, 100, &params);
        for expected in [
            "agents            500",
            "steps             100",
            "perception radius",
            "separation radius",
            "weights",
            "max speed",
            "timestep",
        ] {
            assert!(report.contains(expected), "report is missing {expected:?}");
        }
    }

    #[test]
    fn the_report_shows_the_separation_radius() {
        // Left out originally, and it is the setting that decides whether the
        // swarm flocks at all — so a run's output could not tell you which
        // behaviour you were looking at.
        let params = Params {
            separation_radius: 12.5,
            ..Params::default()
        };
        assert!(configuration_report("test", 1, 1, &params).contains("12.5"));
    }

    #[test]
    fn progress_lines_stay_lined_up_as_the_step_number_grows() {
        // The two runners' output gets compared line by line, so the columns
        // have to stay put. The step number is padded to four characters, so
        // everything up to step 9999 lines up exactly.
        let first = progress_line(0, 0.5, 0.25);
        let later = progress_line(6000, 0.5, 0.25);
        assert_eq!(first.len(), later.len());
        assert!(first.contains("0.500"));
        assert!(first.contains("0.250"));
    }

    #[test]
    fn a_five_digit_step_widens_the_line_by_exactly_one() {
        // Past 9999 the column has to give. Worth knowing rather than being
        // surprised by it in a long run.
        let inside = progress_line(9999, 0.5, 0.25);
        let past = progress_line(10000, 0.5, 0.25);
        assert_eq!(past.len(), inside.len() + 1);
    }
}
