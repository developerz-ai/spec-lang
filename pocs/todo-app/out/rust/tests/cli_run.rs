// Projected by hand from vocabulary cli: dispatch, the streams and the exit codes (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.
// The whole program in process, against a store in a directory of its own.

mod support;

use std::ffi::OsString;
use std::fs;
use std::time::Duration;

use support::TempDir;
use todo::cli::{Exit, HELP, run};
use todo::store::Store;
use todo::store::lock::{self, Mode, Patience};

const SHORT: Patience = Patience {
    every: Duration::from_millis(5),
    up_to: Duration::from_millis(40),
};

/// What one run printed and how it ended.
#[derive(Debug, PartialEq, Eq)]
struct Ran {
    out: String,
    err: String,
    exit: Exit,
}

fn todo(store: &Store, arguments: &[&str]) -> Ran {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let arguments = arguments.iter().map(OsString::from).collect();
    let exit = run(arguments, store, &mut out, &mut err);
    Ran {
        out: String::from_utf8(out).expect("UTF-8 on standard output"),
        err: String::from_utf8(err).expect("UTF-8 on standard error"),
        exit,
    }
}

fn said(out: &str) -> Ran {
    Ran {
        out: out.to_owned(),
        err: String::new(),
        exit: Exit::Success,
    }
}

fn failed(err: &str, exit: Exit) -> Ran {
    Ran {
        out: String::new(),
        err: err.to_owned(),
        exit,
    }
}

#[test]
fn exit_codes() {
    assert_eq!(
        [Exit::Success, Exit::Refused, Exit::Usage, Exit::Store].map(Exit::code),
        [0, 1, 2, 3]
    );
}

#[test]
fn help_goes_to_standard_output_or_with_no_arguments_to_standard_error() {
    let dir = TempDir::new();
    let store = Store::at(dir.join(".todo.json"));
    assert_eq!(todo(&store, &["--help"]), said(HELP));
    assert_eq!(todo(&store, &["add", "-h"]), said(HELP));
    assert_eq!(todo(&store, &[]), failed(HELP, Exit::Usage));
    assert!(!dir.join(".todo.json").exists());
    assert!(
        !dir.join(".todo.json.lock").exists(),
        "help and usage never reach the store"
    );
}

#[test]
fn a_usage_error_is_one_line_and_never_reaches_the_store() {
    let dir = TempDir::new();
    let store = Store::at(dir.join(".todo.json"));
    assert_eq!(
        todo(&store, &["add"]),
        failed("error: missing <title>\n", Exit::Usage)
    );
    assert_eq!(
        todo(&store, &["frobnicate"]),
        failed("error: unknown command \"frobnicate\"\n", Exit::Usage)
    );
    assert!(!dir.join(".todo.json.lock").exists());
}

#[test]
fn the_rules_end_to_end() {
    let dir = TempDir::new();
    let store = Store::at(dir.join(".todo.json"));
    assert_eq!(todo(&store, &["list"]), said("nothing to do\n"));
    assert_eq!(todo(&store, &["stats"]), said("0 open, 0 done\n"));
    assert!(!dir.join(".todo.json").exists());
    assert_eq!(todo(&store, &["add", "milk"]), said("added 1\n"));
    assert_eq!(
        todo(
            &store,
            &[
                "add",
                "bread",
                "--tags",
                "shop",
                "--tags",
                "home",
                "--priority",
                "high"
            ]
        ),
        said("added 2\n")
    );
    assert_eq!(
        todo(&store, &["list"]),
        said("2 [ ] bread !high #home #shop\n1 [ ] milk\n")
    );
    assert_eq!(todo(&store, &["done", "1"]), said("1 [x] milk\n"));
    assert_eq!(
        todo(&store, &["done", "1"]),
        failed("error: AlreadyDone\n", Exit::Refused)
    );
    assert_eq!(todo(&store, &["stats"]), said("1 open, 1 done\n"));
    assert_eq!(
        todo(&store, &["reopen", "2"]),
        failed("error: NotDone\n", Exit::Refused)
    );
    assert_eq!(
        todo(&store, &["rename", "2", "rye"]),
        said("2 [ ] rye !high #home #shop\n")
    );
    assert_eq!(
        todo(&store, &["list", "--all", "--tag", "shop"]),
        said("2 [ ] rye !high #home #shop\n")
    );
    assert_eq!(todo(&store, &["clear"]), said("cleared 1\n"));
    assert_eq!(todo(&store, &["remove", "2"]), said("removed 2\n"));
    assert_eq!(
        todo(&store, &["remove", "99999999999999999999999999999999"]),
        failed("error: NoTask\n", Exit::Refused)
    );
    assert_eq!(
        fs::read_to_string(dir.join(".todo.json")).expect("the store"),
        "{\"version\":1,\"next_no\":3,\"tasks\":[]}\n"
    );
}

#[test]
fn store_errors_name_the_path_as_configured() {
    let dir = TempDir::new();
    let store = Store::at(dir.join("bad.json"));
    let path = dir.join("bad.json").display().to_string();
    fs::write(dir.join("bad.json"), "[1]\n").expect("seeded");
    for command in [&["list"][..], &["stats"], &["add", "x"], &["clear"]] {
        assert_eq!(
            todo(&store, command),
            failed(&format!("error: store {path}: unreadable\n"), Exit::Store)
        );
    }
    assert_eq!(
        fs::read_to_string(dir.join("bad.json")).expect("the store"),
        "[1]\n"
    );

    let nowhere = Store::at(dir.join("lists/home.json"));
    let path = dir.join("lists/home.json").display().to_string();
    for command in [&["list"][..], &["add", "x"]] {
        assert_eq!(
            todo(&nowhere, command),
            failed(&format!("error: store {path}: unwritable\n"), Exit::Store)
        );
    }
}

#[test]
fn the_last_number_then_full() {
    let dir = TempDir::new();
    let store = Store::at(dir.join("s.json"));
    let path = dir.join("s.json").display().to_string();
    fs::write(
        dir.join("s.json"),
        "{\"version\":1,\"next_no\":9007199254740991,\"tasks\":[]}\n",
    )
    .expect("seeded");
    assert_eq!(
        todo(&store, &["add", "a"]),
        said("added 9007199254740991\n")
    );
    let stored = fs::read_to_string(dir.join("s.json")).expect("the store");
    assert_eq!(
        todo(&store, &["add", "b"]),
        failed(&format!("error: store {path}: full\n"), Exit::Store)
    );
    assert_eq!(
        fs::read_to_string(dir.join("s.json")).expect("the store"),
        stored
    );
}

#[test]
fn a_held_lock_is_contended() {
    let dir = TempDir::new();
    let store = Store::with_patience(dir.join("s.json"), SHORT);
    let _held = lock::acquire(&dir.join("s.json.lock"), Mode::Exclusive, SHORT).expect("the lock");
    assert_eq!(
        todo(&store, &["add", "x"]),
        failed("error: Contended\n", Exit::Refused)
    );
    assert_eq!(
        todo(&store, &["list"]),
        failed("error: Contended\n", Exit::Refused)
    );
    assert!(!dir.join("s.json").exists());
}
