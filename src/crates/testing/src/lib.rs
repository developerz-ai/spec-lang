//! The test engine: every `@cases` row, `test` and `law` of a program, run
//! by the interpreter and reported with how much it shows.
//!
//! | Form | Run as | Evidence |
//! |---|---|---|
//! | `@cases` on a `fn` or `test` | one call per row | `example` |
//! | `test` without parameters | its `@given` world, then its steps | `example` |
//! | `test` with parameters | parameters generated from their types, seeded, shrunk | `sampled(n)` |
//! | `law` | random traces of the rules its `Trace[...]` names | `sampled(n traces x k steps)` |
//!
//! Nothing here reports `proven`: sampling is never proof. Every run is
//! seeded and deterministic; the counts are in [`Config`] and in the report.

pub mod config;
mod generate;
mod law;
mod missing;
mod parallel;
mod property;
pub mod report;
pub mod rng;
mod runner;
mod scenario;
mod shrink;
mod trace;

pub use config::Config;
pub use report::{CheckResult, Evidence, Kind, Report, Status};
pub use runner::run_all;
