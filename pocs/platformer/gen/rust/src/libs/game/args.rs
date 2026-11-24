//! The command line: `play`, `run`, `draw` and `shot`, their flags and help, parsed in the order of the platformer
//! POC's contract, section 2. Arguments are bytes: one is printed back exactly as given, UTF-8 or not.

use super::script::digits;
use super::shape::Shape;
use crate::rt::Int;
use crate::rt::data::Data;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Play,
    Run,
    Draw,
    Shot,
}

const COMMANDS: &[(&str, Command)] =
    &[("play", Command::Play), ("run", Command::Run), ("draw", Command::Draw), ("shot", Command::Shot)];

#[derive(Debug)]
pub struct Invocation {
    pub command: Command,
    /// The start rule's argument; `None` takes its default.
    pub level: Option<Data>,
    /// `--at`; `None` is the end of the script. Beyond the word it is the largest: no script is that long.
    pub at: Option<u64>,
    pub out: Option<PathBuf>,
}

#[derive(Debug)]
pub enum Parsed {
    NoArguments,
    Help,
    Run(Invocation),
}

impl Command {
    /// Its flags besides the level's, and how help shows their value.
    fn flags(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Command::Play | Command::Run => &[],
            Command::Draw => &[("at", "<tick>")],
            Command::Shot => &[("at", "<tick>"), ("out", "<file.png>")],
        }
    }
}

pub fn help(shape: &Shape) -> String {
    let mut s = format!("usage: {} <command> [options]\n\ncommands:\n", shape.bin);
    for (name, command) in COMMANDS {
        s.push_str(&format!("  {name}"));
        if let Some(p) = shape.level() {
            s.push_str(&format!(" [--{} <n>]", p.name));
        }
        for (flag, hint) in command.flags() {
            s.push_str(&format!(" [--{flag} {hint}]"));
        }
        s.push('\n');
    }
    let controls = shape.input.why.map_or(String::new(), |w| format!("{w}, "));
    s.push_str(&format!(
        "\nplay opens a window: {controls}Esc to quit.\nrun, draw and shot read a script of controls on standard input.\n"
    ));
    s
}

fn message(head: &str, arg: &[u8], tail: &str) -> Vec<u8> {
    [b"error: ", head.as_bytes(), arg, tail.as_bytes(), b"\n"].concat()
}

/// The invocation, or the whole line for standard error.
pub fn parse(shape: &Shape, args: &[Vec<u8>]) -> Result<Parsed, Vec<u8>> {
    let Some((first, rest)) = args.split_first() else { return Ok(Parsed::NoArguments) };
    if args.iter().any(|a| a == b"--help" || a == b"-h") {
        return Ok(Parsed::Help);
    }
    let Some((_, command)) = COMMANDS.iter().find(|(n, _)| n.as_bytes() == first) else {
        return Err(message("unknown command \"", first, "\""));
    };
    let level = shape.level().map(|p| p.name);
    let names: Vec<&str> = level.into_iter().chain(command.flags().iter().map(|(n, _)| *n)).collect();
    let mut seen: Vec<&str> = Vec::new();
    let mut invocation = Invocation { command: *command, level: None, at: None, out: None };
    let mut rest = rest.iter();
    while let Some(arg) = rest.next() {
        let Some(flag) = arg.strip_prefix(b"--") else {
            return Err(message("unexpected argument \"", arg, "\""));
        };
        let Some(name) = names.iter().copied().find(|n| n.as_bytes() == flag) else {
            return Err(message("unknown flag ", arg, ""));
        };
        let Some(value) = rest.next() else { return Err(message("", arg, " needs a value")) };
        if seen.contains(&name) {
            return Err(message("", arg, " given twice"));
        }
        seen.push(name);
        let not_a = |what: &str| message("", arg, &format!(": not a {what}"));
        match shape.level() {
            Some(p) if p.name == name => {
                let d = digits(value).map(|n| Data::Int(Int::from(n))).filter(|d| (p.ty.admits)(d));
                invocation.level = Some(d.ok_or_else(|| not_a(p.ty.name))?);
            }
            _ if name == "at" => invocation.at = Some(digits(value).ok_or_else(|| not_a("tick"))?),
            _ => {
                use std::os::unix::ffi::OsStrExt;
                if value.is_empty() {
                    return Err(not_a("path"));
                }
                invocation.out = Some(PathBuf::from(std::ffi::OsStr::from_bytes(value)));
            }
        }
    }
    if invocation.level.is_none() {
        invocation.level = shape.default_level().map_err(|m| message(&m, b"", ""))?;
    }
    Ok(Parsed::Run(invocation))
}
