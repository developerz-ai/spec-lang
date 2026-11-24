//! The spec-lang interpreter: it runs the resolved program ([`spec_lang_ir`])
//! to check it, never to ship it.
//!
//! * [`value::Value`]: immutable values with shared structure; `Map` and
//!   `Set` are ordered, so iteration is ascending and the same everywhere.
//! * [`world::World`]: the value of every `state`. Forking a world copies a
//!   vector of shared roots.
//! * [`Interp`]: evaluation. A rule called from outside runs atomically: on a
//!   failure the world and the pending `out.line` lines are restored.
//! * `Int` is `i128`; a result outside it stops evaluation with `E-RUNTIME`
//!   instead of wrapping. The language says unbounded; this is the engine's limit.

pub mod conform;
pub mod display;
mod error;
mod eval;
mod interp;
mod stdlib;
pub mod value;
pub mod world;

pub use error::{Halt, Halted};
pub use eval::{Env, Flow};
pub use interp::{Interp, Outcome};
pub use value::{Closure, Event, Failure, Step, Trace, Value};
pub use world::World;
