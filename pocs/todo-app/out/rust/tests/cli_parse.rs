// Projected by hand from vocabulary cli: "Parsing, in this order" and the types table (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

mod support;

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

use support::{no, tag, tags, title};
use todo::board::Command;
use todo::cli::{Invocation, UsageError, parse};
use todo::model::{Priority, TaskNo};

fn line(arguments: &[&str]) -> Result<Invocation, UsageError> {
    parse(arguments.iter().map(OsString::from).collect())
}

fn add(text: &str, priority: Priority, tagged: &[&str]) -> Result<Invocation, UsageError> {
    Ok(Invocation::Run(Command::Add {
        title: title(text),
        priority,
        tags: tags(tagged),
    }))
}

fn run(command: Command) -> Result<Invocation, UsageError> {
    Ok(Invocation::Run(command))
}

#[test]
fn no_arguments_and_help() {
    assert_eq!(line(&[]), Ok(Invocation::NoArguments));
    assert_eq!(line(&["--help"]), Ok(Invocation::Help));
    assert_eq!(line(&["-h"]), Ok(Invocation::Help));
    assert_eq!(line(&["add", "-h"]), Ok(Invocation::Help));
    assert_eq!(
        line(&["frobnicate", "--bad", "--help"]),
        Ok(Invocation::Help)
    );
    assert_eq!(line(&["add", "--priority", "--help"]), Ok(Invocation::Help));
    assert_eq!(
        line(&["add", "--", "--help"]),
        add("--help", Priority::Normal, &[])
    );
    assert_eq!(line(&["add", "--", "-h"]), add("-h", Priority::Normal, &[]));
}

#[test]
fn every_command_binds_its_parameters() {
    assert_eq!(line(&["add", "milk"]), add("milk", Priority::Normal, &[]));
    assert_eq!(
        line(&["add", "--priority", "high", "ship it", "--tags", "work"]),
        add("ship it", Priority::High, &["work"])
    );
    assert_eq!(
        line(&["add", "x", "--priority", "low"]),
        add("x", Priority::Low, &[])
    );
    assert_eq!(
        line(&["add", "x", "--priority", "normal"]),
        add("x", Priority::Normal, &[])
    );
    assert_eq!(
        line(&["add", "x", "--tags", "b", "--tags", "a", "--tags", "b"]),
        add("x", Priority::Normal, &["a", "b"])
    );
    assert_eq!(
        line(&["list"]),
        run(Command::List {
            all: false,
            tag: None
        })
    );
    assert_eq!(
        line(&["list", "--tag", "home", "--all"]),
        run(Command::List {
            all: true,
            tag: Some(tag("home"))
        })
    );
    assert_eq!(line(&["done", "001"]), run(Command::Done { no: no(1) }));
    assert_eq!(line(&["reopen", "2"]), run(Command::Reopen { no: no(2) }));
    assert_eq!(
        line(&["rename", "3", "new"]),
        run(Command::Rename {
            no: no(3),
            title: title("new")
        })
    );
    assert_eq!(line(&["remove", "4"]), run(Command::Remove { no: no(4) }));
    assert_eq!(line(&["clear"]), run(Command::Clear));
    assert_eq!(line(&["stats"]), run(Command::Stats));
}

#[test]
fn positionals_that_look_like_flags() {
    assert_eq!(
        line(&["add", "--", "--all"]),
        add("--all", Priority::Normal, &[])
    );
    assert_eq!(line(&["add", "-x"]), add("-x", Priority::Normal, &[]));
    assert_eq!(line(&["add", "-"]), add("-", Priority::Normal, &[]));
    assert_eq!(line(&["add", "--", "--"]), add("--", Priority::Normal, &[]));
    assert_eq!(line(&["add", "x", "--"]), add("x", Priority::Normal, &[]));
}

#[test]
fn a_number_past_every_machine_word_is_a_task_no() {
    assert_eq!(
        line(&["done", "99999999999999999999999999999999"]),
        run(Command::Done { no: TaskNo::BEYOND })
    );
}

#[test]
fn each_usage_error_and_which_one_wins() {
    let long_tag = "abcdefghijklmnopqrstuvwxyz1234567";
    let cases: &[(&[&str], &str)] = &[
        (&["frobnicate"], "unknown command \"frobnicate\""),
        (&["Add", "x"], "unknown command \"Add\""),
        (&["--", "--help"], "unknown command \"--\""),
        (&["--all"], "unknown command \"--all\""),
        (&["add"], "missing <title>"),
        (&["done"], "missing <no>"),
        (&["rename", "1"], "missing <title>"),
        (&["rename"], "missing <no>"),
        (&["add", "buy", "milk"], "unexpected argument \"milk\""),
        (&["clear", "now"], "unexpected argument \"now\""),
        (&["list", "--", "--all"], "unexpected argument \"--all\""),
        (&["done", "0"], "<no>: not a TaskNo"),
        (&["done", "-1"], "<no>: not a TaskNo"),
        (&["done", "+1"], "<no>: not a TaskNo"),
        (&["done", "1.5"], "<no>: not a TaskNo"),
        (&["done", "abc"], "<no>: not a TaskNo"),
        (&["done", ""], "<no>: not a TaskNo"),
        (&["done", " 1"], "<no>: not a TaskNo"),
        (&["done", "\u{661}"], "<no>: not a TaskNo"),
        (&["add", ""], "<title>: not a Title"),
        (&["add", "a\tb"], "<title>: not a Title"),
        (&["add", "a\u{2028}b"], "<title>: not a Title"),
        (&["add", "a\u{2029}b"], "<title>: not a Title"),
        (&["add", "a\u{85}b"], "<title>: not a Title"),
        (&["rename", "1", " "], "<title>: not a Title"),
        (&["add", "x", "--force"], "unknown flag --force"),
        (
            &["add", "x", "--priority=high"],
            "unknown flag --priority=high",
        ),
        (&["done", "1", "--all"], "unknown flag --all"),
        (&["add", "x", "---"], "unknown flag ---"),
        (
            &["add", "x", "--Priority", "high"],
            "unknown flag --Priority",
        ),
        (&["add", "x", "--priority"], "--priority needs a value"),
        (&["add", "x", "--tags"], "--tags needs a value"),
        (&["list", "--tag"], "--tag needs a value"),
        (
            &["add", "x", "--priority", "High"],
            "--priority: not a Priority",
        ),
        (
            &["add", "x", "--priority", "--tags"],
            "--priority: not a Priority",
        ),
        (
            &["add", "x", "--priority", "--"],
            "--priority: not a Priority",
        ),
        (&["add", "x", "--tags", "Home"], "--tags: not a Tag"),
        (&["add", "x", "--tags", ""], "--tags: not a Tag"),
        (&["add", "x", "--tags", long_tag], "--tags: not a Tag"),
        (&["add", "x", "--tags", "-x"], "--tags: not a Tag"),
        (&["add", "x", "--tags", "--all"], "--tags: not a Tag"),
        (&["list", "--tag", "Home"], "--tag: not a Tag"),
        (&["list", "--tag", "--all"], "--tag: not a Tag"),
        (&["list", "--all", "--all"], "--all given twice"),
        (&["list", "--tag", "a", "--tag", "b"], "--tag given twice"),
        (&["list", "--tag", "a", "--tag", "B"], "--tag given twice"),
        (&["list", "--tag", "a", "--tag"], "--tag needs a value"),
        (
            &["add", "x", "--priority", "low", "--priority", "high"],
            "--priority given twice",
        ),
        (&["done", "abc", "--force"], "<no>: not a TaskNo"),
        (&["done", "--force", "abc"], "unknown flag --force"),
        (
            &["add", "x", "y", "--priority", "nope"],
            "unexpected argument \"y\"",
        ),
        (&["add", "--priority", "nope"], "--priority: not a Priority"),
        (&["rename", "x"], "<no>: not a TaskNo"),
    ];
    for (arguments, expected) in cases {
        let error = line(arguments).expect_err(expected);
        assert_eq!(error.to_string(), *expected, "{arguments:?}");
    }
}

#[test]
fn an_argument_that_is_not_unicode_is_a_usage_error() {
    let bad = OsString::from_vec(vec![b'a', 0xFF]);
    assert_eq!(
        parse(vec!["add".into(), bad.clone()]),
        Err(UsageError::NotUnicode(2))
    );
    assert_eq!(
        parse(vec![bad.clone(), "--help".into()]),
        Ok(Invocation::Help)
    );
    assert_eq!(
        UsageError::NotUnicode(2).to_string(),
        "argument 2: not UTF-8"
    );
}
