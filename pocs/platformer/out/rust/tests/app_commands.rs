// Projected by hand from the run and draw commands the game vocabulary derives from app SkyhopRust (src/apps/rust.slg, docs/game-vocabulary.md). Stand-in for spec-lang build output; change the spec, not this file.
//! Whole commands in process, with the streams as buffers: what `main` does, minus the process.

use std::path::Path;

use skyhop::app::{self, Io, fnv1a64};
use skyhop::cli::Exit;

struct Ran {
    exit: Exit,
    out: String,
    err: String,
}

fn skyhop(args: &[&str], stdin: &str) -> Ran {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let exit = app::main(
        args,
        &mut Io {
            stdin: &mut stdin.as_bytes(),
            stdout: &mut out,
            stderr: &mut err,
            assets: Path::new("assets"),
        },
    );
    Ran {
        exit,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

const MEADOW_WIN: &str = "75 R\n20 RJ\n41 R\n20 RJ\n35 R\n20 RJ\n23 R\n20 RJ\n62 R\n";

#[test]
fn run_prints_every_line_then_the_summary() {
    let r = skyhop(&["run"], MEADOW_WIN);
    assert_eq!(r.exit, Exit::Success);
    assert_eq!(r.err, "");
    let want = "0 level meadow\n1 land\n76 jump\n114 land\n137 jump\n140 gem 1\n167 land\n192 jump\n231 land\n235 jump\n238 gem 2\n279 land\n284 gem 3\n316 won\nend 316 won gems 3/3 lives 3 at 4992,0,0\n";
    assert_eq!(r.out, want);
}

#[test]
fn run_with_an_empty_script_still_starts() {
    let r = skyhop(&["run", "--level", "2"], "");
    assert_eq!(
        r.out,
        "0 level ridge\nend 0 playing gems 0/3 lives 3 at 0,0,0\n"
    );
}

#[test]
fn draw_stops_after_the_tick_asked_for() {
    let r = skyhop(&["draw", "--at", "12"], "12 LF\n");
    assert_eq!(r.exit, Exit::Success);
    let lines: Vec<&str> = r.out.lines().collect();
    assert_eq!(lines[0], "sky 135,206,235");
    assert_eq!(lines[1], "camera -102,704,1946 -102,192,-102");
    assert_eq!(lines[2], "time 12");
    assert_eq!(lines.iter().rev().nth(2), Some(&"mesh pip -102,0,-102 135"));
    let r0 = skyhop(&["draw", "--at", "0"], MEADOW_WIN);
    assert!(r0.out.contains("time 0\n"));
}

#[test]
fn draw_past_the_end_stops_at_the_end() {
    let r = skyhop(&["draw", "--at", "99999999999999999999"], MEADOW_WIN);
    assert!(
        r.out
            .starts_with("sky 135,206,235\ncamera 4992,704,2048 4992,192,0\ntime 316\n")
    );
    assert!(
        r.out
            .ends_with("label 380 250 40 255,255,255 level clear\n")
    );
}

#[test]
fn a_bad_script_prints_nothing_and_exits_2() {
    for command in ["run", "draw", "shot"] {
        let r = skyhop(&[command], "300 R\n1 Q\n");
        assert_eq!(r.exit, Exit::Usage);
        assert_eq!(r.out, "");
        assert_eq!(r.err, "error: input line 2: unknown key \"Q\"\n");
    }
}

#[test]
fn usage_errors_and_help() {
    let r = skyhop(&[], "");
    assert_eq!((r.exit, r.out.as_str()), (Exit::Usage, ""));
    assert!(r.err.starts_with("usage: skyhop <command> [options]\n"));
    let r = skyhop(&["draw", "--at", "-h"], "");
    assert_eq!((r.exit, r.err.as_str()), (Exit::Success, ""));
    assert!(r.out.starts_with("usage: skyhop"));
    let r = skyhop(&["run", "--level", "3"], "nonsense");
    assert_eq!(r.exit, Exit::Usage);
    assert_eq!(r.err, "error: --level: not a LevelNo\n");
}

#[test]
fn fnv1a64_known_values() {
    assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
}
