// Projected by hand from rule remove (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

use super::{Out, Refusal, World};
use crate::model::TaskNo;

/// `@guard(no in tasks else NoTask) pub rule remove(no)`. The guard and the removal are one lookup.
pub fn remove(world: &mut World, out: &mut Out, no: TaskNo) -> Result<(), Refusal> {
    world.tasks.remove(&no).ok_or(Refusal::NoTask)?;
    out.line(format_args!("removed {no}"));
    Ok(())
}
