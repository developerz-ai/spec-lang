// Projected by hand from vocabulary cli: "Derived, never written", parameters to arguments (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

use std::collections::BTreeSet;

use super::UsageError;
use crate::board::Command;
use crate::model::{Priority, Tag, TaskNo, Title};

/// The parameter types the board's rules use, as the command line reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueType {
    TaskNo,
    Title,
    Tag,
    Priority,
}

/// One argument, parsed and with its claim checked at the edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    TaskNo(TaskNo),
    Title(Title),
    Tag(Tag),
    Priority(Priority),
}

impl ValueType {
    /// The type's name as the spec writes it, for "not a ...".
    pub const fn name(self) -> &'static str {
        match self {
            Self::TaskNo => "TaskNo",
            Self::Title => "Title",
            Self::Tag => "Tag",
            Self::Priority => "Priority",
        }
    }

    /// The argument moves into the value: a title given on the command line is never copied.
    pub fn parse(self, text: String) -> Option<Value> {
        match self {
            Self::TaskNo => TaskNo::parse(&text).map(Value::TaskNo),
            Self::Title => Title::new(text).map(Value::Title),
            Self::Tag => Tag::new(text).map(Value::Tag),
            Self::Priority => priority(&text).map(Value::Priority),
        }
    }
}

/// A sum type with no data is its variant names in lower case, exactly.
fn priority(text: &str) -> Option<Priority> {
    Priority::ALL.into_iter().find(|priority| {
        let name = priority.name().as_bytes();
        name.len() == text.len()
            && name
                .iter()
                .zip(text.bytes())
                .all(|(upper, given)| upper.to_ascii_lowercase() == given)
    })
}

/// A parameter without a default: `<name>`, by position.
#[derive(Clone, Copy, Debug)]
pub struct Positional {
    pub name: &'static str,
    pub value: ValueType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlagKind {
    /// `Bool = false`: `--name`, at most once.
    Switch,
    /// `Option[T] = none`, or any other default that is not a set (`priority: Priority = Normal`): `--name <value>`, at most once.
    Option(ValueType),
    /// `Set[T]` with a default: `--name <value>`, repeatable; repeats collapse.
    Set(ValueType),
}

/// A parameter with a default: `--name`.
#[derive(Clone, Copy, Debug)]
pub struct Flag {
    pub name: &'static str,
    pub kind: FlagKind,
}

/// One `@on(cli("name"))` rule: its parameters, and how they become its event.
#[derive(Clone, Copy, Debug)]
pub struct CommandSpec {
    pub name: &'static str,
    pub positionals: &'static [Positional],
    pub flags: &'static [Flag],
    pub bind: fn(Parsed) -> Result<Command, UsageError>,
}

/// The arguments of one command, each already of its parameter's type.
#[derive(Debug, Default)]
pub struct Parsed {
    values: Vec<(&'static str, Value)>,
    switches: Vec<&'static str>,
}

impl Parsed {
    pub(super) fn push(&mut self, name: &'static str, value: Value) {
        self.values.push((name, value));
    }

    pub(super) fn switch_on(&mut self, name: &'static str) {
        self.switches.push(name);
    }

    pub fn has(&self, name: &str) -> bool {
        self.values.iter().any(|(given, _)| *given == name)
    }

    pub fn switch(&self, name: &str) -> bool {
        self.switches.contains(&name)
    }

    fn take(&mut self, name: &str) -> Option<Value> {
        let at = self.values.iter().position(|(given, _)| *given == name)?;
        Some(self.values.remove(at).1)
    }

    pub fn task_no(&mut self, name: &str) -> Option<TaskNo> {
        match self.take(name)? {
            Value::TaskNo(no) => Some(no),
            _ => None,
        }
    }

    pub fn title(&mut self, name: &str) -> Option<Title> {
        match self.take(name)? {
            Value::Title(title) => Some(title),
            _ => None,
        }
    }

    pub fn tag(&mut self, name: &str) -> Option<Tag> {
        match self.take(name)? {
            Value::Tag(tag) => Some(tag),
            _ => None,
        }
    }

    pub fn priority(&mut self, name: &str) -> Option<Priority> {
        match self.take(name)? {
            Value::Priority(priority) => Some(priority),
            _ => None,
        }
    }

    /// Every value given under `name`; the set collapses repeats.
    pub fn tags(&mut self, name: &str) -> BTreeSet<Tag> {
        std::iter::from_fn(|| self.tag(name)).collect()
    }
}
