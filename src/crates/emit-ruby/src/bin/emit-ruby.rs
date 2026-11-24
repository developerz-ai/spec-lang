//! `emit-ruby <spec dir> <out dir>`: build the app with `@target(ruby)`.

use std::path::Path;
use std::process::ExitCode;

/// `--without a,b` switches strategies off; with none named, every strategy is on.
fn strategies(rest: &[String]) -> Result<emit_ruby::Strategies, String> {
    let named = rest.iter().position(|a| a == "--without").and_then(|i| rest.get(i + 1));
    let names: Vec<&str> = named.map(|n| n.split(',').collect()).unwrap_or_default();
    emit_ruby::Strategies::without(&names)
}

fn run(args: &[String]) -> Result<(), String> {
    let [src, out, rest @ ..] = args else {
        return Err("usage: emit-ruby <spec dir> <out dir> [--without strategy,...]".into());
    };
    let strategies = strategies(rest)?;
    let program = spec_lang_ir::load_project(Path::new(src)).map_err(|e| e.render())?;
    let app = program
        .apps
        .iter()
        .find(|a| a.target.as_deref() == Some(emit_ruby::TARGET))
        .ok_or("no app with @target(ruby)")?;
    let tracked = emit_ruby::emit_tracked_with(&program, app, strategies).map_err(|d| d.human(&program.sources))?;
    let name = program.qualified(&app.info);
    let files = emit_ts::out::finish(emit_ruby::TARGET, &name, tracked).map_err(|d| d.human(&program.sources))?;
    let dir = Path::new(out);
    emit_ts::out::write(dir, &files).map_err(|e| e.to_string())?;
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
