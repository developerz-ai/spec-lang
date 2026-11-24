// Projected by hand from the parameters of rule add, list, done, reopen, rename, remove, clear, stats (src/todo/board.slg) via trigger @on(cli(...)) (docs/cli-vocabulary.md). Stand-in for spec-lang build output; change the spec, not this file.

use super::params::{CommandSpec, Flag, FlagKind, Parsed, Positional, ValueType};
use super::usage::UsageError;
use crate::board::Command;

const NO: Positional = Positional {
    name: "no",
    value: ValueType::TaskNo,
};
const TITLE: Positional = Positional {
    name: "title",
    value: ValueType::Title,
};

/// Every `cli` rule in declaration order, which is the order of `--help`.
/// Static: the table costs nothing at run time.
pub static COMMANDS: [CommandSpec; 8] = [
    CommandSpec {
        name: "add",
        positionals: &[TITLE],
        flags: &[
            Flag {
                name: "priority",
                kind: FlagKind::Option(ValueType::Priority),
            },
            Flag {
                name: "tags",
                kind: FlagKind::Set(ValueType::Tag),
            },
        ],
        bind: |mut given| {
            Ok(Command::Add {
                title: required(given.title("title"), "title")?,
                priority: given.priority("priority").unwrap_or_default(),
                tags: given.tags("tags"),
            })
        },
    },
    CommandSpec {
        name: "list",
        positionals: &[],
        flags: &[
            Flag {
                name: "all",
                kind: FlagKind::Switch,
            },
            Flag {
                name: "tag",
                kind: FlagKind::Option(ValueType::Tag),
            },
        ],
        bind: |mut given| {
            Ok(Command::List {
                all: given.switch("all"),
                tag: given.tag("tag"),
            })
        },
    },
    CommandSpec {
        name: "done",
        positionals: &[NO],
        flags: &[],
        bind: |mut given| {
            Ok(Command::Done {
                no: required(given.task_no("no"), "no")?,
            })
        },
    },
    CommandSpec {
        name: "reopen",
        positionals: &[NO],
        flags: &[],
        bind: |mut given| {
            Ok(Command::Reopen {
                no: required(given.task_no("no"), "no")?,
            })
        },
    },
    CommandSpec {
        name: "rename",
        positionals: &[NO, TITLE],
        flags: &[],
        bind: |mut given| {
            Ok(Command::Rename {
                no: required(given.task_no("no"), "no")?,
                title: required(given.title("title"), "title")?,
            })
        },
    },
    CommandSpec {
        name: "remove",
        positionals: &[NO],
        flags: &[],
        bind: |mut given| {
            Ok(Command::Remove {
                no: required(given.task_no("no"), "no")?,
            })
        },
    },
    CommandSpec {
        name: "clear",
        positionals: &[],
        flags: &[],
        bind: |_: Parsed| Ok(Command::Clear),
    },
    CommandSpec {
        name: "stats",
        positionals: &[],
        flags: &[],
        bind: |_: Parsed| Ok(Command::Stats),
    },
];

/// The parser has already reported a missing positional; this keeps the binder total without a panic.
fn required<T>(value: Option<T>, name: &'static str) -> Result<T, UsageError> {
    value.ok_or(UsageError::Missing(name))
}
