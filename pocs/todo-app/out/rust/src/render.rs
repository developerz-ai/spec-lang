// Projected by hand from fn mark, badge, tag_text, urgency, rank, line (src/todo/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::collections::BTreeSet;
use std::fmt;

use crate::model::{Priority, Status, Tag, Task, TaskNo};

pub const fn mark(status: Status) -> &'static str {
    match status {
        Status::Open => " ",
        Status::Done => "x",
    }
}

pub const fn badge(priority: Priority) -> &'static str {
    match priority {
        Priority::Low => " !low",
        Priority::Normal => "",
        Priority::High => " !high",
    }
}

/// `tags.sort(it).map(t => " #{t}").join("")`. The set is kept in code-point order, so the sort is free.
pub fn tag_text(tags: &BTreeSet<Tag>) -> String {
    let mut text = String::new();
    for tag in tags {
        text.push_str(" #");
        text.push_str(tag.as_str());
    }
    text
}

pub const fn urgency(priority: Priority) -> u8 {
    match priority {
        Priority::High => 0,
        Priority::Normal => 1,
        Priority::Low => 2,
    }
}

/// Most urgent first, then oldest first.
pub const fn rank(task: &Task) -> (u8, TaskNo) {
    (urgency(task.priority), task.no)
}

/// `"{t.no} [{mark(t.status)}] {t.title}{badge(t.priority)}{tag_text(t.tags)}"`.
pub fn line(task: &Task) -> String {
    Line(task).to_string()
}

/// [`line`] as a value that formats straight into its destination, with no string in between.
pub struct Line<'a>(pub &'a Task);

impl fmt::Display for Line<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let task = self.0;
        write!(f, "{} [{}] ", task.no, mark(task.status))?;
        f.write_str(task.title.as_str())?;
        f.write_str(badge(task.priority))?;
        for tag in &task.tags {
            f.write_str(" #")?;
            f.write_str(tag.as_str())?;
        }
        Ok(())
    }
}
