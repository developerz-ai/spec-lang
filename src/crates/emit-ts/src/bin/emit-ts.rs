//! `emit-ts <spec dir> <out dir> [--normalize]`: build the app with `@target(bun)`.
//!
//! With `--normalize` the project's own `bin/normalize` runs afterwards and
//! `provenance.json` is rewritten for the formatted text.

use std::path::Path;
use std::process::{Command, ExitCode};

/// `--without a,b` switches strategies off; with none named, every strategy is on.
fn strategies(rest: &[String]) -> Result<emit_ts::Strategies, String> {
    let named = rest.iter().position(|a| a == "--without").and_then(|i| rest.get(i + 1));
    let names: Vec<&str> = named.map(|n| n.split(',').collect()).unwrap_or_default();
    emit_ts::Strategies::without(&names)
}

fn run(args: &[String]) -> Result<(), String> {
    let [src, out, rest @ ..] = args else {
        return Err("usage: emit-ts <spec dir> <out dir> [--normalize] [--without strategy,...]".into());
    };
    let strategies = strategies(rest)?;
    let program = spec_lang_ir::load_project(Path::new(src)).map_err(|e| e.render())?;
    let app =
        program.apps.iter().find(|a| a.target.as_deref() == Some(emit_ts::TARGET)).ok_or("no app with @target(bun)")?;
    let name = program.qualified(&app.info);
    let tracked = emit_ts::emit_tracked_with(&program, app, strategies).map_err(|d| d.human(&program.sources))?;
    let files = emit_ts::out::finish(emit_ts::TARGET, &name, tracked.clone()).map_err(|d| d.human(&program.sources))?;
    let dir = Path::new(out);
    emit_ts::out::write(dir, &files).map_err(|e| e.to_string())?;
    if rest.iter().any(|a| a == "--normalize") {
        let status = Command::new(dir.join("bin/normalize")).status().map_err(|e| format!("bin/normalize: {e}"))?;
        if !status.success() {
            return Err(format!("bin/normalize: {status}"));
        }
        emit_ts::out::reanchor(dir, emit_ts::TARGET, &name, &tracked)?;
    }
    println!("{}: {} files", dir.display(), files.len());
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
