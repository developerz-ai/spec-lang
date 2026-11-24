//! Numbered runs of one check spread over threads, each with its own
//! machine, merged by number: the result never depends on the threads.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

/// What one run gave: a failure, if any.
pub(crate) struct RunResult<F> {
    pub(crate) failure: Option<F>,
}

/// The lowest-numbered failure.
pub(crate) struct Merged<F> {
    pub(crate) first: Option<(u32, F)>,
}

/// Run `work(0..n)` on `jobs` threads; runs above a known failure are skipped.
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
    for (i, slot) in slots.into_iter().enumerate() {
        // Runs are handed out in order and only runs above a failure are
        // skipped, so every run below the first failure finished.
        let Some(r) = slot else { break };
        if let Some(f) = r.failure {
            return Merged { first: Some((u32::try_from(i).unwrap_or(u32::MAX), f)) };
        }
    }
    Merged { first: None }
}
