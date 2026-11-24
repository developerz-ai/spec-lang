//! Threads change how long a run takes, never what it prints: the same seed
//! gives the same report, counterexamples included, with 1 or 16 workers.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;
use std::process::Command;

/// The report without the wall-clock time on its last line.
fn report(dir: &Path, jobs: &str) -> String {
    let out =
        Command::new(env!("CARGO_BIN_EXE_spec-lang")).arg("test").arg(dir).args(["--jobs", jobs]).output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    text.lines().map(|l| l.split(" in ").next().unwrap_or(l).to_string()).collect::<Vec<_>>().join("\n")
}

#[test]
fn one_or_sixteen_workers_print_the_same_report() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../pocs/todo-app/src");
    assert_eq!(report(&dir, "1"), report(&dir, "16"));
}

#[test]
fn a_failing_law_reports_the_same_counterexample_on_any_number_of_workers() {
    let dir = std::env::temp_dir().join(format!("spec-lang-jobs-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("m.slg"),
        "state n: Int = 0;\nrule up(k: Int) = { n = n + k; }\nlaw small(t: Trace[up]) = t.states.all(w => w.n < 50);\n\
         test grows(k: Int) = { assert k < 30; }\n",
    )
    .unwrap();
    let one = report(&dir, "1");
    assert!(one.contains("F-LAW") && one.contains("small_counterexample"), "{one}");
    assert_eq!(one, report(&dir, "16"));
    std::fs::remove_dir_all(&dir).unwrap();
}
