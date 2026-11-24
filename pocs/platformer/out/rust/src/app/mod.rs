// Projected by hand from app SkyhopRust(start: world.start, tick: world.step, input: screen.read_pad, frame: screen.present, dump: render.dump, report: world.summary) (src/apps/rust.slg). Stand-in for spec-lang build output; change the spec, not this file.
//! The four commands the game vocabulary derives: `play`, `run`, `draw`, `shot`.

mod draw;
mod hash;
mod play;
mod run;
mod shot;

use std::io::{Read, Write};
use std::path::Path;

use crate::cli::{self, Command, Exit, Parsed};
use crate::script::{self, Script};

pub use draw::simulate;
pub use hash::fnv1a64;

/// The process edge, passed in: standard streams and where the assets are.
pub struct Io<'a> {
    pub stdin: &'a mut dyn Read,
    pub stdout: &'a mut dyn Write,
    pub stderr: &'a mut dyn Write,
    pub assets: &'a Path,
}

impl Io<'_> {
    /// One error line on standard error. If standard error itself fails there is nobody left to tell; the exit code still says it.
    fn error(&mut self, text: impl std::fmt::Display) {
        let _ = writeln!(self.stderr, "error: {text}");
    }

    /// Writes what a command printed; a failed write is exit 1 (NOTES.md, open choices).
    fn print(&mut self, text: &[u8]) -> Exit {
        match self
            .stdout
            .write_all(text)
            .and_then(|()| self.stdout.flush())
        {
            Ok(()) => Exit::Success,
            Err(_) => Exit::Output,
        }
    }

    /// Standard input, read whole and checked before the first tick.
    fn script(&mut self) -> Result<Script, Exit> {
        let mut input = Vec::new();
        if self.stdin.read_to_end(&mut input).is_err() {
            self.error("input: cannot read");
            return Err(Exit::Usage);
        }
        script::read(&input).map_err(|e| {
            self.error(e);
            Exit::Usage
        })
    }
}

/// `skyhop <args>`: parse, then run one command.
pub fn main<A: AsRef<[u8]>>(args: &[A], io: &mut Io<'_>) -> Exit {
    let invocation = match cli::parse(args) {
        Ok(Parsed::Help) => return io.print(cli::HELP.as_bytes()),
        Ok(Parsed::Run(invocation)) => invocation,
        Err(e) => {
            let _ = io.stderr.write_all(&e.message());
            return Exit::Usage;
        }
    };
    let options = invocation.options;
    if invocation.command == Command::Play {
        return play::play(options.level, io);
    }
    let script = match io.script() {
        Ok(script) => script,
        Err(exit) => return exit,
    };
    match invocation.command {
        Command::Run => run::run(options.level, &script, io),
        Command::Draw => draw::draw(options.level, options.at, &script, io),
        Command::Shot => shot::shot(&options, &script, io),
        Command::Play => Exit::Success,
    }
}
