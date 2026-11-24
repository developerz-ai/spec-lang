//! The `cli` library's runtime for Rust: a program its user runs from a shell. Reads the app's manifest
//! (`crate::rt::app::App`): `@bin`, `@store`, every rule with `@on(cli(..))`, and `out.text` for what a command
//! printed. Exit codes: 0 success, 1 a rule refused, 2 usage, 3 store.

pub mod args;
pub mod store;

use crate::machine::Machine;
use crate::rt::Halt;
use crate::rt::app::{App, NoteValue};
use crate::rt::data::Data;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

/// The whole text in one write. A stream that cannot take it (a closed pipe) has no one left to tell.
fn emit(stream: &mut dyn Write, text: &str) {
    let _ = stream.write_all(text.as_bytes()).and_then(|()| stream.flush());
}

/// `@store(file(path))`, replaced by `<BIN>_STORE` when that is set and not empty.
fn store_path(app: &App, bin: &str) -> Option<PathBuf> {
    let variable = format!("{}_STORE", bin.to_uppercase().replace('-', "_"));
    if let Some(v) = std::env::var_os(variable).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(v));
    }
    match app.note("store")?.arg("at")? {
        NoteValue::Variant("file", args) => args.iter().find(|a| a.param == "path").and_then(|a| match a.value {
            NoteValue::Text(t) => Some(PathBuf::from(t)),
            _ => None,
        }),
        _ => None,
    }
}

pub fn main(app: &'static App) -> ExitCode {
    let bin = app.note("bin").and_then(|n| n.text("name")).unwrap_or(app.name);
    let commands = args::commands(app);
    let help = args::help(bin, &commands);
    let (mut stdout, mut stderr) = (std::io::stdout(), std::io::stderr());
    let (command, values) = match args::parse(std::env::args_os().skip(1).collect(), &commands) {
        Ok(args::Invocation::Run(c, v)) => (c, v),
        Ok(args::Invocation::Help) => {
            emit(&mut stdout, &help);
            return ExitCode::SUCCESS;
        }
        Ok(args::Invocation::NoArguments) => {
            emit(&mut stderr, &help);
            return ExitCode::from(2);
        }
        Err(usage) => {
            emit(&mut stderr, &format!("error: {usage}\n"));
            return ExitCode::from(2);
        }
    };
    let store = store_path(app, bin).map(store::Store::at);
    match run(app, command.rule, values, store.as_ref()) {
        Ok(text) => {
            emit(&mut stdout, &text);
            ExitCode::SUCCESS
        }
        Err(Failure::Refused(name)) => {
            emit(&mut stderr, &format!("error: {name}\n"));
            ExitCode::from(1)
        }
        Err(Failure::Store(store::StoreError::Contended)) => {
            emit(&mut stderr, "error: Contended\n");
            ExitCode::from(1)
        }
        Err(Failure::Store(e)) => {
            let why = match e {
                store::StoreError::Unreadable => "unreadable",
                store::StoreError::Full => "full",
                _ => "unwritable",
            };
            let path = store.as_ref().map_or(String::new(), |s| s.path().display().to_string());
            emit(&mut stderr, &format!("error: store {path}: {why}\n"));
            ExitCode::from(3)
        }
        Err(Failure::Stopped(message)) => {
            emit(&mut stderr, &format!("error: {message}\n"));
            ExitCode::from(1)
        }
    }
}

#[derive(Debug)]
enum Failure {
    Refused(String),
    Store(store::StoreError),
    /// Evaluation stopped: a checked type broken, an overflow.
    Stopped(String),
}

/// One command: the world from the store, the rule, the world back when it wrote and changed, and the text it
/// printed (`out.text`), sent only after the store holds the new world.
fn run(
    app: &'static App,
    rule: &crate::rt::app::Rule,
    values: Vec<Option<Data>>,
    store: Option<&store::Store>,
) -> Result<String, Failure> {
    let stopped = |h: Halt| Failure::Stopped(h.message());
    let mut m = Machine::new().map_err(stopped)?;
    let lock = store.map(|s| s.lock(rule.writes)).transpose().map_err(Failure::Store)?;
    let before = store.map(store::Store::bytes).transpose().map_err(Failure::Store)?.flatten();
    if let Some(bytes) = &before {
        m.world = store::load(app, bytes).map_err(Failure::Store)?;
    }
    match (rule.call)(&mut m, values) {
        Ok(_) => {}
        Err(Halt::Fail(f)) => return Err(Failure::Refused(f.name)),
        Err(h) => return Err(stopped(h)),
    }
    if let (Some(s), true) = (store, rule.writes) {
        let after = store::dump(app, &m.world).map_err(Failure::Store)?;
        if before.as_deref() != Some(after.as_slice()) {
            s.replace(&after).map_err(Failure::Store)?;
        }
    }
    drop(lock);
    let text = match app.read("out", "text") {
        Some(r) => match (r.read)(&mut m).map_err(stopped)? {
            Data::Text(t) => t,
            _ => String::new(),
        },
        None => String::new(),
    };
    Ok(text)
}
