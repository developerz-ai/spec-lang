//! The mutation proof: small wrong edits to the todo program must make
//! `spec-lang test` fail. Each mutant is a copy of `pocs/todo-app/src` in a
//! temporary directory with one textual edit; the test runs the real binary
//! with default counts and records which checks caught it.
//!
//! Mutants that are expected to survive are listed with the reason; a
//! survivor not on that list, or a listed one that starts being caught,
//! fails this test so the list stays true.
//!
//! Run with `--nocapture` to see the table.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::Command;

struct Mutant {
    name: &'static str,
    file: &'static str,
    from: &'static str,
    to: &'static str,
    /// Why no test or law can see it, if none can.
    survives: Option<&'static str>,
}

const MUTANTS: &[Mutant] = &[
    Mutant {
        name: "next_no is not advanced",
        file: "todo/board.slg",
        from: "next_no = no + 1;",
        to: "next_no = no;",
        survives: None,
    },
    Mutant {
        name: "done's guards swapped",
        file: "todo/board.slg",
        from: "@guard(no in tasks else NoTask, tasks[no].status == Open else AlreadyDone)",
        to: "@guard(tasks[no].status == Open else AlreadyDone, no in tasks else NoTask)",
        survives: None,
    },
    Mutant {
        name: "list shows done tasks",
        file: "todo/board.slg",
        from: "(all or t.status == Open)",
        to: "(all or true)",
        survives: None,
    },
    Mutant {
        name: "clear removes nothing",
        file: "todo/board.slg",
        from: "for t in gone { tasks.remove(t.no); }",
        to: "for t in gone { }",
        survives: None,
    },
    Mutant {
        name: "reopen guard checks Open",
        file: "todo/board.slg",
        from: "tasks[no].status == Done else NotDone",
        to: "tasks[no].status == Open else NotDone",
        survives: None,
    },
    Mutant {
        name: "done sets Open",
        file: "todo/board.slg",
        from: "tasks[no].status = Done;",
        to: "tasks[no].status = Open;",
        survives: None,
    },
    Mutant {
        name: "rename keeps the old title",
        file: "todo/board.slg",
        from: "tasks[no].title = title;",
        to: "tasks[no].title = tasks[no].title;",
        survives: None,
    },
    Mutant {
        name: "remove has no guard",
        file: "todo/board.slg",
        from: "@guard(no in tasks else NoTask)\npub rule remove",
        to: "pub rule remove",
        survives: None,
    },
    Mutant {
        name: "stats counts all as done",
        file: "todo/board.slg",
        from: "{tasks.size - open} done",
        to: "{tasks.size} done",
        survives: None,
    },
    Mutant {
        name: "no tag filter matches nothing",
        file: "todo/board.slg",
        from: "none => true;",
        to: "none => false;",
        survives: None,
    },
    Mutant {
        name: "rank by number first",
        file: "todo/render.slg",
        from: "(urgency(t.priority), t.no)",
        to: "(t.no, urgency(t.priority))",
        survives: None,
    },
    Mutant {
        name: "high badge misspelt",
        file: "todo/render.slg",
        from: "High => \" !high\";",
        to: "High => \" !hi\";",
        survives: None,
    },
    Mutant { name: "tags allow _ not -", file: "todo/model.slg", from: "it == 45", to: "it == 95", survives: None },
    Mutant {
        name: "titles up to 2000 code points",
        file: "todo/model.slg",
        from: "t.size <= 200",
        to: "t.size <= 2000",
        survives: Some("no case or test has a title of 201 to 2000 code points; generated titles are at most 12"),
    },
    Mutant {
        name: "add ignores its priority",
        file: "todo/board.slg",
        from: "priority: priority, tags: tags",
        to: "priority: Normal, tags: tags",
        survives: Some(
            "every scenario that reads a priority sets it through @given, and no law relates add's arguments to the stored task",
        ),
    },
];

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let p = e.unwrap().path();
        let dest = to.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_dir(&p, &dest);
        } else {
            std::fs::copy(&p, &dest).unwrap();
        }
    }
}

/// Run `spec-lang test --json` on `dir`: exit code and the failing checks.
fn run(dir: &Path) -> (i32, Vec<String>) {
    let out = Command::new(env!("CARGO_BIN_EXE_spec-lang")).arg("test").arg(dir).arg("--json").output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let caught = stdout
        .lines()
        .filter(|l| l.contains(r#""status":"fail""#) || l.contains(r#""status":"error""#))
        .map(|l| {
            let field = |k: &str| {
                l.split(&format!(r#""{k}":""#)).nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string()
            };
            if l.contains(r#""stage":"test""#) {
                format!("{} {}", field("kind"), field("decl"))
            } else {
                format!("{} {}", field("stage"), field("code"))
            }
        })
        .collect();
    (out.status.code().unwrap_or(-1), caught)
}

fn workdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("spec-lang-mutants-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn every_mutant_is_caught_or_listed() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../pocs/todo-app/src");
    let base = workdir("base");
    copy_dir(&src, &base);
    let (code, caught) = run(&base);
    assert_eq!(code, 0, "the unmutated program must be green: {caught:?}");
    std::fs::remove_dir_all(&base).unwrap();

    let results: Vec<(usize, i32, Vec<String>)> = std::thread::scope(|s| {
        let handles: Vec<_> = MUTANTS
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let src = src.clone();
                s.spawn(move || {
                    let dir = workdir(&i.to_string());
                    copy_dir(&src, &dir);
                    let path = dir.join(m.file);
                    let text = std::fs::read_to_string(&path).unwrap();
                    assert_eq!(
                        text.matches(m.from).count(),
                        1,
                        "mutant {:?}: `{}` must occur once in {}",
                        m.name,
                        m.from,
                        m.file
                    );
                    std::fs::write(&path, text.replace(m.from, m.to)).unwrap();
                    let (code, caught) = run(&dir);
                    std::fs::remove_dir_all(&dir).unwrap();
                    (i, code, caught)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    let mut wrong = Vec::new();
    eprintln!("\n| mutant | caught by |\n|---|---|");
    for (i, code, caught) in results {
        let m = &MUTANTS[i];
        let killed = code != 0;
        let by = if killed { caught.join(", ") } else { format!("SURVIVED: {}", m.survives.unwrap_or("unexplained")) };
        eprintln!("| {} | {} |", m.name, by);
        if killed == m.survives.is_some() {
            wrong.push(m.name);
        }
    }
    assert!(wrong.is_empty(), "mutants whose fate differs from the list: {wrong:?}");
}
