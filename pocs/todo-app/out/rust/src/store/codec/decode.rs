// Projected by hand from note @store: the table "Accepted on read" (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

use std::borrow::Cow;

use super::Unreadable;
use super::cursor::Cursor;
use crate::board::{RestoreError, World};
use crate::model::{Priority, Status, Tag, Task, TaskNo, Title, is_tag, is_title};

/// One stored task, already checked, borrowed from the store's bytes.
#[derive(Clone, Copy, Debug)]
pub struct TaskView<'a> {
    pub no: TaskNo,
    pub title: &'a str,
    pub status: Status,
    pub priority: Priority,
    /// In code-point order, none twice.
    pub tags: &'a [Cow<'a, str>],
}

/// Checks the whole store against the contract and shows each task to `visit`
/// in file order. Returns `next_no`. An error may come after some tasks were
/// shown; the caller must then drop what it gathered.
pub fn scan(
    bytes: &[u8],
    mut visit: impl FnMut(TaskView<'_>) -> Result<(), Unreadable>,
) -> Result<TaskNo, Unreadable> {
    let mut cursor = Cursor::new(bytes)?;
    let mut version = None;
    let mut next_no = None;
    let mut tasks = None;
    let mut numbers = Numbers::default();
    let mut tags = Vec::new();
    cursor.object(|cursor, key| match key {
        "version" => match cursor.integer()? {
            1 => set(&mut version, ()),
            _ => Err(Unreadable::Version),
        },
        "next_no" => {
            let at_most_limit = cursor.integer()?;
            set(&mut next_no, at_least_one(at_most_limit)?)
        }
        "tasks" => {
            set(&mut tasks, ())?;
            cursor.array(|cursor| task(cursor, &mut tags, &mut numbers, &mut visit))
        }
        _ => Err(Unreadable::UnknownKey),
    })?;
    cursor.end()?;
    let (Some(()), Some(next_no), Some(())) = (version, next_no, tasks) else {
        return Err(Unreadable::MissingKey);
    };
    numbers.check(next_no)?;
    Ok(next_no)
}

/// The world the store holds.
pub fn decode(bytes: &[u8]) -> Result<World, Unreadable> {
    let mut tasks = Vec::new();
    let next_no = scan(bytes, |view| {
        tasks.push(owned(view)?);
        Ok(())
    })?;
    World::restore(next_no, tasks).map_err(|why| match why {
        RestoreError::Duplicate(_) => Unreadable::NumberTwice,
        RestoreError::NotBelowNext(_) => Unreadable::NumberNotBelowNext,
        RestoreError::NextBeyondLimit => Unreadable::Integer,
    })
}

fn owned(view: TaskView<'_>) -> Result<Task, Unreadable> {
    Ok(Task {
        no: view.no,
        title: Title::new(view.title).ok_or(Unreadable::Title)?,
        status: view.status,
        priority: view.priority,
        tags: view
            .tags
            .iter()
            .map(|tag| Tag::new(tag.as_ref()).ok_or(Unreadable::Tag))
            .collect::<Result<_, _>>()?,
    })
}

/// One element of `tasks`: an object with exactly the five keys, in any order.
fn task<'a>(
    cursor: &mut Cursor<'a>,
    tags: &mut Vec<Cow<'a, str>>,
    numbers: &mut Numbers,
    visit: &mut impl FnMut(TaskView<'_>) -> Result<(), Unreadable>,
) -> Result<(), Unreadable> {
    let mut no = None;
    let mut title = None;
    let mut status = None;
    let mut priority = None;
    let mut tagged = None;
    tags.clear();
    cursor.object(|cursor, key| match key {
        "no" => {
            let at_most_limit = cursor.integer()?;
            set(&mut no, at_least_one(at_most_limit)?)
        }
        "title" => {
            let text = cursor.string()?;
            if !is_title(&text) {
                return Err(Unreadable::Title);
            }
            set(&mut title, text)
        }
        "status" => {
            let name = cursor.string()?;
            set(
                &mut status,
                Status::from_name(&name).ok_or(Unreadable::Status)?,
            )
        }
        "priority" => {
            let name = cursor.string()?;
            set(
                &mut priority,
                Priority::from_name(&name).ok_or(Unreadable::Priority)?,
            )
        }
        "tags" => {
            set(&mut tagged, ())?;
            cursor.array(|cursor| {
                let tag = cursor.string()?;
                if !is_tag(&tag) {
                    return Err(Unreadable::Tag);
                }
                tags.push(tag);
                Ok(())
            })
        }
        _ => Err(Unreadable::UnknownKey),
    })?;
    let (Some(no), Some(title), Some(status), Some(priority), Some(())) =
        (no, title, status, priority, tagged)
    else {
        return Err(Unreadable::MissingKey);
    };
    // A canonical store has them sorted already, so this is one pass.
    tags.sort_unstable();
    if tags.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(Unreadable::TagTwice);
    }
    numbers.push(no);
    visit(TaskView {
        no,
        title: &title,
        status,
        priority,
        tags,
    })
}

/// Fills a key's slot; a slot already full is the key given twice.
fn set<T>(slot: &mut Option<T>, value: T) -> Result<(), Unreadable> {
    match slot.replace(value) {
        None => Ok(()),
        Some(_) => Err(Unreadable::DuplicateKey),
    }
}

fn at_least_one(value: u64) -> Result<TaskNo, Unreadable> {
    TaskNo::new(value).ok_or(Unreadable::Integer)
}

/// The task numbers seen, for "below `next_no`" and "unique". Both wait for
/// the end of the file because `next_no` may come after `tasks`.
#[derive(Default)]
struct Numbers {
    seen: Vec<TaskNo>,
    ascending: bool,
}

impl Numbers {
    fn push(&mut self, no: TaskNo) {
        self.ascending = match self.seen.last() {
            None => true,
            Some(&last) => self.ascending && last < no,
        };
        self.seen.push(no);
    }

    fn check(mut self, next_no: TaskNo) -> Result<(), Unreadable> {
        // A canonical store is ascending, which is unique for free; only a hand-edited one pays for a sort.
        if !self.ascending {
            self.seen.sort_unstable();
            if self.seen.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(Unreadable::NumberTwice);
            }
        }
        match self.seen.last() {
            Some(&largest) if largest >= next_no => Err(Unreadable::NumberNotBelowNext),
            _ => Ok(()),
        }
    }
}
