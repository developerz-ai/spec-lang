// Projected by hand from rule stats (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

use super::{Out, World};
use crate::model::Status;

/// All `stats` reads of the world: how many tasks, and how many of them are open.
/// The rule is this fold, so anything that can walk the statuses can run it
/// without a world; the store's reader does (`store::Store::fold`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    open: usize,
    size: usize,
}

impl Tally {
    pub fn count(&mut self, status: Status) {
        self.size += 1;
        if status == Status::Open {
            self.open += 1;
        }
    }

    /// `out.line("{open} open, {tasks.size - open} done")`.
    pub fn report(self, out: &mut Out) {
        out.line(format_args!(
            "{} open, {} done",
            self.open,
            self.size - self.open
        ));
    }
}

/// `pub rule stats()`. `@uses(tasks.read, out.write)`: the world is borrowed shared.
pub fn stats(world: &World, out: &mut Out) {
    let mut tally = Tally::default();
    for task in world.tasks.values() {
        tally.count(task.status);
    }
    tally.report(out);
}
