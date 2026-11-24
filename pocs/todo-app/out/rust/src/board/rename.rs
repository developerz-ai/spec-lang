// Projected by hand from rule rename (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

use super::{Out, Refusal, World};
use crate::model::{TaskNo, Title};
use crate::render::Line;

/// `@guard(no in tasks else NoTask) pub rule rename(no, title)`.
pub fn rename(world: &mut World, out: &mut Out, no: TaskNo, title: Title) -> Result<(), Refusal> {
    let task = world.tasks.get_mut(&no).ok_or(Refusal::NoTask)?;
    task.title = title;
    out.line(Line(task));
    Ok(())
}
