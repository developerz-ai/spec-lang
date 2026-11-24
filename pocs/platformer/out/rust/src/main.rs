// Projected by hand from app SkyhopRust, note @bin("skyhop") (src/apps/rust.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::env;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use std::process::ExitCode;

use skyhop::app::{self, Io};

/// The only place that touches the process: arguments, environment, the three streams, the exit code.
/// raylib is linked at build time, so `SKYHOP_RAYLIB` is not read (contract section 3).
fn main() -> ExitCode {
    let args: Vec<Vec<u8>> = env::args_os().skip(1).map(OsStringExt::into_vec).collect();
    let assets = env::var_os("SKYHOP_ASSETS")
        .filter(|v| !v.is_empty())
        .map_or_else(|| PathBuf::from("assets"), PathBuf::from);
    let exit = app::main(
        &args,
        &mut Io {
            stdin: &mut io::stdin().lock(),
            stdout: &mut io::stdout().lock(),
            stderr: &mut io::stderr().lock(),
            assets: &assets,
        },
    );
    ExitCode::from(exit.code())
}
