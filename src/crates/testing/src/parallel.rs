//! One world per worker thread (decision 006): numbered runs of one check
//! spread over threads, results merged by run number, never by thread.
//!
//! Each run is a pure function of its number (its seed comes from the base
//! seed, the check's name and the number), so the merged result is the one
//! a single thread would give: the failure with the smallest number, and a
//! tally over every run up to it. Runs numbered above a known failure are
//! skipped, which is the parallel form of stopping at the first failure.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

/// What one run gave: a count to add up (traces that started lawful, ...)
/// and a failure, if any.
pub(crate) struct RunResult<F> {
    pub(crate) tally: u32,
    pub(crate) failure: Option<F>,
}

/// The merged outcome: the lowest-numbered failure, and the tally of runs
/// `0..=` that failure (or of all runs).
pub(crate) struct Merged<F> {
    pub(crate) first: Option<(u32, F)>,
    pub(crate) tally: u32,
}

/// Run `work(0..n)` on `jobs` threads.
pub(crate) fn first_failure<F: Send>(n: u32, jobs: usize, work: impl Fn(u32) -> RunResult<F> + Sync) -> Merged<F> {
    let next = AtomicU32::new(0);
    let bound = AtomicU32::new(u32::MAX);
    let slots: Mutex<Vec<Option<RunResult<F>>>> = Mutex::new((0..n).map(|_| None).collect());
    let worker = || {
        loop {
            let i = next.fetch_add(1, Ordering::Relaxed);
            if i >= n || i > bound.load(Ordering::Relaxed) {
                break;
            }
            let r = work(i);
            if r.failure.is_some() {
                bound.fetch_min(i, Ordering::Relaxed);
            }
            if let Ok(mut s) = slots.lock() {
                s[i as usize] = Some(r);
            }
        }
    };
    let jobs = jobs.clamp(1, n.max(1) as usize);
    if jobs == 1 {
        worker();
    } else {
        std::thread::scope(|s| {
            for _ in 0..jobs {
                s.spawn(worker);
            }
        });
    }
    let slots = slots.into_inner().unwrap_or_else(|e| e.into_inner());
    let mut tally = 0;
    for (i, slot) in slots.into_iter().enumerate() {
        // Every run below the first failure was taken and finished: runs
        // are handed out in order and only runs above a failure are skipped.
        let Some(r) = slot else { break };
        tally += r.tally;
        if let Some(f) = r.failure {
            return Merged { first: Some((u32::try_from(i).unwrap_or(u32::MAX), f)), tally };
        }
    }
    Merged { first: None, tally }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lowest_failure_wins_whatever_the_threads() {
        for jobs in [1, 2, 8] {
            let m = first_failure(100, jobs, |i| RunResult { tally: 1, failure: (i % 37 == 36).then_some(i) });
            assert_eq!(m.first, Some((36, 36)));
            assert_eq!(m.tally, 37);
        }
        let m = first_failure(10, 4, |_| RunResult::<()> { tally: 2, failure: None });
        assert!(m.first.is_none());
        assert_eq!(m.tally, 20);
    }
}
