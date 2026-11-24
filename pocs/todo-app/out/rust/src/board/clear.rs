// Projected by hand from rule clear (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

use super::{Out, World};
use crate::model::Status;

/// `pub rule clear() -> Int`. Finished work leaves the list in one step.
/// `gone` is only ever counted, so it is never built: one pass removes and the sizes give the count.
pub fn clear(world: &mut World, out: &mut Out) -> usize {
    let before = world.tasks.len();
    world.tasks.retain(|_, task| task.status != Status::Done);
    let gone = before - world.tasks.len();
    out.line(format_args!("cleared {gone}"));
    gone
}
