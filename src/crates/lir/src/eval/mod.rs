//! The LIR evaluator: a tree-walking evaluator of the LIR (not of the IR)
//! that runs every check of a program and reports pass, fail or blocked per
//! check, named as `spec-lang test` names it. It is the proof that lowering
//! keeps the meaning: on every program it must report what the engine
//! reports. Generation uses the engine's generator and seeds, so properties
//! and laws draw the engine's inputs (`README.md`, What is reproducible).

mod conform;
mod expr;
mod generate;
mod law;
mod machine;
mod parallel;
mod prims;
pub mod rng;
mod run;
pub mod show;
mod stmt;
pub mod value;

pub use machine::{Env, Flow, Halt, Halted, Machine};
pub use run::{CheckOutcome, Config, Status, run_all};
