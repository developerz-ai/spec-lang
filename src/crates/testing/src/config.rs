//! How much the test engine samples. The defaults are fixed and printed in
//! every report; they are choices, not measured optima.

/// Counts and the base seed of one run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Base seed; each check derives its own from it and its name.
    pub seed: u64,
    /// Runs per property test.
    pub runs: u32,
    /// Traces per law.
    pub traces: u32,
    /// Steps per trace.
    pub steps: u32,
    /// At most this many random calls build each trace's start world.
    pub prefix: u32,
    /// Worker threads for the runs of one law or property. Results never
    /// depend on it: runs are merged by number, not by thread.
    pub jobs: usize,
}

impl Default for Config {
    fn default() -> Self {
        let jobs = std::thread::available_parallelism().map_or(1, usize::from);
        Config { seed: 0, runs: 100, traces: 200, steps: 40, prefix: 10, jobs }
    }
}
