// Projected by hand from fn has_tag, rule list (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

use super::{Out, World};
use crate::model::{Status, Tag, Task};
use crate::render::{Line, rank};

pub fn has_tag(task: &Task, tag: Option<&Tag>) -> bool {
    match tag {
        None => true,
        Some(tag) => task.tags.contains(tag),
    }
}

/// `pub rule list(all = false, tag = none)`. Open work first; finished work only when asked for.
/// `@uses(tasks.read, out.write)`: the world is borrowed shared, so it cannot change here.
pub fn list(world: &World, out: &mut Out, all: bool, tag: Option<&Tag>) {
    let mut shown: Vec<&Task> = world
        .tasks
        .values()
        .filter(|task| (all || task.status == Status::Open) && has_tag(task, tag))
        .collect();
    if shown.is_empty() {
        out.line("nothing to do");
        return;
    }
    // Ranks are unique (they end in the task's number), so an unstable sort has one answer.
    shown.sort_unstable_by_key(|task| rank(task));
    for task in shown {
        out.line(Line(task));
    }
}
