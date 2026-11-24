//! The TypeScript-on-Bun printer of `spec-lang build` (decision 009): the LIR of any program to a Bun
//! project in strict TypeScript, with a small runtime of its own (`runtime/`) and, for an app, the runtimes
//! of its libraries from `libs/<name>/real/ts/`. It knows TypeScript and its runtime, never a program or a
//! library by name. See `README.md` in this crate.

mod app;
mod checks;
mod expr;
mod file;
mod machine;
mod module;
mod names;
mod plan;
mod project;
mod stmt;
mod types;

use plan::Plan;
use spec_lang_diag::Diag;
use spec_lang_lir::{Lir, OutFile, Printer};

/// The printer `spec-lang build` finds by target.
#[derive(Debug)]
pub struct TsPrinter;

impl Printer for TsPrinter {
    fn targets(&self) -> &'static [&'static str] {
        &["ts", "bun"]
    }

    fn tests(&self, lir: &Lir) -> Result<Vec<OutFile>, Vec<Diag>> {
        let plan = Plan::new(lir);
        let mut files = project::common("spec-lang-checks", "the program's checks");
        files.extend(program(&plan)?);
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    fn app(&self, lir: &Lir, app: &str) -> Result<Vec<OutFile>, Vec<Diag>> {
        let plan = Plan::new(lir);
        app::project(&plan, app, program(&plan)?)
    }
}

/// The program's code and its checks: every module, the machine, the program as data, the tests.
fn program(plan: &Plan<'_>) -> Result<Vec<OutFile>, Vec<Diag>> {
    let mut files = Vec::new();
    let mut errors = Vec::new();
    let mut add = |r: Result<(String, String), Vec<Diag>>| match r {
        Ok((p, t)) => files.push(project::text(p, t)),
        Err(mut e) => errors.append(&mut e),
    };
    for m in plan.printed_modules() {
        add(module::module_file(plan, m));
        match checks::test_file(plan, m) {
            Ok(Some(f)) => add(Ok(f)),
            Ok(None) => {}
            Err(e) => add(Err(e)),
        }
    }
    add(machine::machine_file(plan));
    add(machine::program_file(plan));
    add(checks::pretend_file(plan));
    add(checks::checks_file(plan));
    if errors.is_empty() { Ok(files) } else { Err(errors) }
}
