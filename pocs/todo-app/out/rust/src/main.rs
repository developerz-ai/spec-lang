// Projected by hand from app TodoRust, note @bin("todo") (src/apps/rust.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::env;
use std::io;
use std::process::ExitCode;

use todo::cli;
use todo::store::{Store, path};

/// The only place that touches the process: arguments, environment, the two streams, the exit code.
fn main() -> ExitCode {
    let arguments = env::args_os().skip(1).collect();
    let store = Store::at(path::configured(env::var_os(path::VARIABLE)));
    let exit = cli::run(
        arguments,
        &store,
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    );
    ExitCode::from(exit.code())
}
