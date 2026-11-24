//! `spec-lang`: the command line.
//!
//! ```text
//! spec-lang parse <path>...          form check of .slg files and ```slg fences in .md files
//! spec-lang test <dir|file> [--seed N] [--runs N] [--traces N] [--steps N] [--json] [--deny warnings]
//! spec-lang view <dir|file> <decl> [--json] | --as pseudo|diagram ...
//! spec-lang build <dir|file> [--app <name>] --out <dir>
//! spec-lang lir <dir|file> [--json] [--eval [--seed N] ...]
//! ```
//! Exit codes: 0 all green, 1 a check failed, 2 usage or load error.

mod args;
mod build_cmd;
mod lir_cmd;
mod output;
mod parse_cmd;
mod sources;
mod test_cmd;
mod view_cmd;

use std::process::ExitCode;

const USAGE: &str = "usage: spec-lang <command> [args]

commands:
  parse <path>...                  check the form of .slg files and ```slg fences in .md files
  test <project> [options]         run every @cases, test and law of a project directory or file
    --seed <n>                     base seed (default 0)
    --runs <n>                     runs per property test (default 100)
    --traces <n>                   traces per law (default 200)
    --steps <n>                    steps per trace (default 40)
    --jobs <n>                     worker threads (default: every core); output does not depend on it
    --json                         one JSON object per line
    --deny warnings                lint warnings fail the run
  view <project> <decl> [--json]   a declaration: source, what names it, every check that reaches it
  build <project> [--app <name>] --out <dir>   check, then print each app with the printer its @target names
  lir <project> [--json]           the program lowered to the LIR (crates/lir/README.md)
  lir <project> --eval [options]   run every check on the LIR evaluator; options as `test`; --json: one object per check
  view <project> --as pseudo [<module>]
  view <project> --as diagram --kind <kind> [--format mermaid|plantuml|json|markdown] [<subject>]
";

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let code = match argv.first().map(String::as_str) {
        Some("parse") => parse_cmd::run(argv.get(1..).unwrap_or(&[])),
        Some("test") => test_cmd::run(argv.get(1..).unwrap_or(&[])),
        Some("view") => view_cmd::run(argv.get(1..).unwrap_or(&[])),
        Some("build") => build_cmd::run(argv.get(1..).unwrap_or(&[])),
        Some("lir") => lir_cmd::run(argv.get(1..).unwrap_or(&[])),
        Some("--help" | "-h") => {
            print!("{USAGE}");
            0
        }
        _ => {
            eprint!("{USAGE}");
            2
        }
    };
    ExitCode::from(code)
}
