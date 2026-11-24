// Projected by hand from the derived command line of app SkyhopRust (src/apps/rust.slg, docs/game-vocabulary.md) as contract section 2 fixes it. Stand-in for spec-lang build output; change the spec, not this file.

use std::path::PathBuf;

use skyhop::cli::{Command, HELP, Invocation, Options, Parsed, UsageError, parse};
use skyhop::levels::LevelNo;

fn error(args: &[&str]) -> String {
    match parse(args) {
        Err(e) => String::from_utf8(e.message()).unwrap(),
        Ok(p) => panic!("{args:?} parsed as {p:?}"),
    }
}

#[test]
fn the_help_text_is_the_contracts() {
    let want = "usage: skyhop <command> [options]\n\ncommands:\n  play [--level <n>]\n  run [--level <n>]\n  draw [--level <n>] [--at <tick>]\n  shot [--level <n>] [--at <tick>] [--out <file.png>]\n\nplay opens a window: arrows or WASD to run, space to jump, Esc to quit.\nrun, draw and shot read a script of controls on standard input.\n";
    assert_eq!(HELP, want);
}

#[test]
fn no_arguments_is_help_on_standard_error() {
    assert_eq!(parse::<&str>(&[]), Err(UsageError::NoArguments));
    assert_eq!(UsageError::NoArguments.message(), HELP.as_bytes());
}

#[test]
fn help_anywhere_wins() {
    for args in [
        &["--help"][..],
        &["-h"],
        &["run", "--level", "9", "-h"],
        &["draw", "--at", "-h"],
        &["jump", "--help"],
    ] {
        assert_eq!(parse(args), Ok(Parsed::Help), "{args:?}");
    }
}

#[test]
fn defaults_and_values() {
    let run = |command, options| Ok(Parsed::Run(Invocation { command, options }));
    assert_eq!(parse(&["play"]), run(Command::Play, Options::default()));
    assert_eq!(
        parse(&["shot", "--out", "a.png", "--at", "007", "--level", "002"]),
        run(
            Command::Shot,
            Options {
                level: LevelNo::Two,
                at: Some(7),
                out: Some(PathBuf::from("a.png")),
            }
        )
    );
    assert_eq!(
        parse(&["draw", "--at", "99999999999999999999999999"]),
        run(
            Command::Draw,
            Options {
                at: Some(u64::MAX),
                ..Options::default()
            }
        )
    );
}

#[test]
fn errors_are_the_contracts_lines() {
    let cases: &[(&[&str], &str)] = &[
        (&["jump"], "error: unknown command \"jump\"\n"),
        (&["--level"], "error: unknown command \"--level\"\n"),
        (&["run", "--fast"], "error: unknown flag --fast\n"),
        (&["run", "--level=2"], "error: unknown flag --level=2\n"),
        (&["run", "--at", "3"], "error: unknown flag --at\n"),
        (&["play", "--out", "x.png"], "error: unknown flag --out\n"),
        (&["run", "--level"], "error: --level needs a value\n"),
        (
            &["draw", "--at", "1", "--at", "2"],
            "error: --at given twice\n",
        ),
        (
            &["draw", "--level", "1", "--level"],
            "error: --level needs a value\n",
        ),
        (&["run", "--level", "0"], "error: --level: not a LevelNo\n"),
        (&["run", "--level", "3"], "error: --level: not a LevelNo\n"),
        (&["run", "--level", "-1"], "error: --level: not a LevelNo\n"),
        (
            &["run", "--level", "one"],
            "error: --level: not a LevelNo\n",
        ),
        (&["run", "--level", ""], "error: --level: not a LevelNo\n"),
        (
            &["run", "--level", "99999999999999999999999"],
            "error: --level: not a LevelNo\n",
        ),
        (
            &["run", "--level", "--fast"],
            "error: --level: not a LevelNo\n",
        ),
        (&["draw", "--at", "-1"], "error: --at: not a tick\n"),
        (&["draw", "--at", "1.5"], "error: --at: not a tick\n"),
        (&["draw", "--at", ""], "error: --at: not a tick\n"),
        (&["shot", "--out", ""], "error: --out: not a path\n"),
        (
            &["run", "meadow"],
            "error: unexpected argument \"meadow\"\n",
        ),
        (&["draw", "-"], "error: unexpected argument \"-\"\n"),
        (
            &["run", "--level", "7", "--fast"],
            "error: --level: not a LevelNo\n",
        ),
        (
            &["run", "--fast", "--level", "7"],
            "error: unknown flag --fast\n",
        ),
    ];
    for (args, want) in cases {
        assert_eq!(error(args), *want, "{args:?}");
    }
}

#[test]
fn arguments_are_printed_as_given_bytes() {
    let args: [&[u8]; 2] = [b"run", b"\xff\xfe"];
    let e = parse(&args).unwrap_err();
    assert_eq!(e.message(), b"error: unexpected argument \"\xff\xfe\"\n");
}
