// Projected by hand from vocabulary cli: dispatch, "fail E: error: E on standard error, exit 1", the error table (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

use std::ffi::OsString;
use std::io::Write;

use super::exit::Exit;
use super::help::HELP;
use super::parse::{Invocation, parse};
use crate::app::{Failure, execute};
use crate::board::Halt;
use crate::store::{Store, StoreError};

/// One run of `todo`: the arguments in, bytes on the two streams, the exit out.
/// Standard output is written only on success, and only after the store holds the new world.
pub fn run(
    arguments: Vec<OsString>,
    store: &Store,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Exit {
    let command = match parse(arguments) {
        Ok(Invocation::Run(command)) => command,
        Ok(Invocation::Help) => {
            emit(stdout, HELP);
            return Exit::Success;
        }
        Ok(Invocation::NoArguments) => {
            emit(stderr, HELP);
            return Exit::Usage;
        }
        Err(usage) => {
            emit(stderr, &format!("error: {usage}\n"));
            return Exit::Usage;
        }
    };
    match execute(store, command) {
        Ok(out) => {
            emit(stdout, out.text());
            Exit::Success
        }
        Err(failure) => {
            let (message, exit) = report(&failure, store);
            emit(stderr, &message);
            exit
        }
    }
}

/// The contract's error table.
fn report(failure: &Failure, store: &Store) -> (String, Exit) {
    let path = store.path().display();
    match failure {
        Failure::Halted(Halt::Refused(refusal)) => (format!("error: {refusal}\n"), Exit::Refused),
        Failure::Store(StoreError::Contended) => ("error: Contended\n".to_owned(), Exit::Refused),
        Failure::Halted(halt @ Halt::Full) => {
            (format!("error: store {path}: {halt}\n"), Exit::Store)
        }
        Failure::Store(error) => (format!("error: store {path}: {error}\n"), Exit::Store),
    }
}

/// The whole text in one write. A stream that cannot take it (a closed pipe)
/// has no one left to tell: the world is already committed and the exit code
/// already decided, so the write's failure changes neither.
fn emit(stream: &mut dyn Write, text: &str) {
    let _ = stream
        .write_all(text.as_bytes())
        .and_then(|()| stream.flush());
}
