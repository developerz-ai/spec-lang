// Projected by hand from state tasks, next_no (src/todo/board.slg) and law numbers_below_next (src/todo/laws.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::collections::BTreeMap;

use crate::model::{Task, TaskNo};

/// `state tasks: Map[TaskNo, Task]` and `state next_no: TaskNo = 1`.
///
/// The module owns its state: the fields are open to the rules beside this
/// file and to nothing else. Kept by every rule and checked by [`World::restore`]:
/// each task sits under its own number, every number is below `next_no`, and
/// `next_no` is at most [`TaskNo::LIMIT`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    pub(super) tasks: BTreeMap<TaskNo, Task>,
    pub(super) next_no: TaskNo,
}

/// Why a set of tasks and a `next_no` are not a world any rule could have left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestoreError {
    /// Two tasks carry this number.
    Duplicate(TaskNo),
    /// This number is not below `next_no`.
    NotBelowNext(TaskNo),
    /// `next_no` is past [`TaskNo::LIMIT`].
    NextBeyondLimit,
}

impl World {
    /// A world from outside (the store, a test's `@given`), admitted only if it keeps the invariants.
    pub fn restore(
        next_no: TaskNo,
        tasks: impl IntoIterator<Item = Task>,
    ) -> Result<Self, RestoreError> {
        if next_no > TaskNo::LIMIT {
            return Err(RestoreError::NextBeyondLimit);
        }
        let mut map = BTreeMap::new();
        for task in tasks {
            let no = task.no;
            if no >= next_no {
                return Err(RestoreError::NotBelowNext(no));
            }
            if map.insert(no, task).is_some() {
                return Err(RestoreError::Duplicate(no));
            }
        }
        Ok(Self {
            tasks: map,
            next_no,
        })
    }

    pub fn tasks(&self) -> &BTreeMap<TaskNo, Task> {
        &self.tasks
    }

    pub fn next_no(&self) -> TaskNo {
        self.next_no
    }
}

impl Default for World {
    /// The initial world: no tasks, `next_no` 1.
    fn default() -> Self {
        Self {
            tasks: BTreeMap::new(),
            next_no: TaskNo::FIRST,
        }
    }
}
