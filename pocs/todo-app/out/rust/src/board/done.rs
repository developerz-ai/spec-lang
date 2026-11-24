// Projected by hand from rule done (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

use super::{Out, Refusal, World};
use crate::model::{Status, TaskNo};
use crate::render::Line;

/// `@guard(no in tasks else NoTask, tasks[no].status == Open else AlreadyDone) pub rule done(no)`.
/// Both guards read one entry, so one lookup serves them and the write.
pub fn done(world: &mut World, out: &mut Out, no: TaskNo) -> Result<(), Refusal> {
    let task = world.tasks.get_mut(&no).ok_or(Refusal::NoTask)?;
    if task.status != Status::Open {
        return Err(Refusal::AlreadyDone);
    }
    task.status = Status::Done;
    out.line(Line(task));
    Ok(())
}
