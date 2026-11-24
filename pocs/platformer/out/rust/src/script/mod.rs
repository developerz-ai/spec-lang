// Projected by hand from the tick rule's parameter pad: Pad (src/hop/world.slg, src/hop/model.slg) and the input script of contract section 2. Stand-in for spec-lang build output; change the spec, not this file.
//! The input script: one letter per `Pad` field, the field's first letter in upper case (`L R F B J`).

mod read;

pub use read::{Hold, MAX_HOLD, MAX_TOTAL, Script, ScriptError, Why, read};
