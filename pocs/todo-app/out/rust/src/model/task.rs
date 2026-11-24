// Projected by hand from type Task (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::collections::BTreeSet;

use super::{Priority, Status, Tag, TaskNo, Title};

/// `type Task(no, title, status = Open, priority = Normal, tags = Set())`.
/// Every field is already a checked type, so the record itself has no claim to keep.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub no: TaskNo,
    pub title: Title,
    pub status: Status,
    pub priority: Priority,
    pub tags: BTreeSet<Tag>,
}

impl Task {
    /// `Task(no, title: title)`: the rest at their defaults.
    pub fn new(no: TaskNo, title: Title) -> Self {
        Self {
            no,
            title,
            status: Status::default(),
            priority: Priority::default(),
            tags: BTreeSet::new(),
        }
    }
}
