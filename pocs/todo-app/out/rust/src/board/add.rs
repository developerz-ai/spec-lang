// Projected by hand from rule add (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::collections::BTreeSet;
use std::fmt;

use super::{Out, World};
use crate::model::{Priority, Status, Tag, Task, TaskNo, Title};

/// Not a spec error: the backend limit of docs/contract.md, carried as a refusal.
/// `next_no` is at 2^53 and the store cannot hold the number after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Full;

impl fmt::Display for Full {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("full")
    }
}

impl std::error::Error for Full {}

/// `pub rule add(title, priority = Normal, tags = Set()) -> TaskNo`.
/// A number is never reused: `next_no` only moves forward.
pub fn add(
    world: &mut World,
    out: &mut Out,
    title: Title,
    priority: Priority,
    tags: BTreeSet<Tag>,
) -> Result<TaskNo, Full> {
    let no = world.next_no;
    let next_no = no.successor().ok_or(Full)?;
    let task = Task {
        no,
        title,
        status: Status::Open,
        priority,
        tags,
    };
    world.tasks.insert(no, task);
    world.next_no = next_no;
    out.line(format_args!("added {no}"));
    Ok(no)
}
