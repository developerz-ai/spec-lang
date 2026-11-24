// Projected by hand from vocabulary cli: "Parsing, in this order" (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

use std::ffi::{OsStr, OsString};

use super::commands::COMMANDS;
use super::params::{CommandSpec, FlagKind, Parsed};
use super::usage::UsageError;
use crate::board::Command;

/// What a command line asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Invocation {
    /// No arguments: help on standard error, exit 2.
    NoArguments,
    /// `--help` or `-h` before the first `--`: help on standard output, exit 0.
    Help,
    Run(Command),
}

/// The contract's steps 1 to 5, in that order. The first problem met is the one reported.
pub fn parse(arguments: Vec<OsString>) -> Result<Invocation, UsageError> {
    if arguments.is_empty() {
        return Ok(Invocation::NoArguments);
    }
    if asks_for_help(&arguments) {
        return Ok(Invocation::Help);
    }
    let mut arguments = unicode(arguments)?.into_iter();
    let Some(name) = arguments.next() else {
        return Ok(Invocation::NoArguments);
    };
    let Some(spec) = COMMANDS.iter().find(|spec| spec.name == name) else {
        return Err(UsageError::UnknownCommand(name));
    };
    let parsed = parameters(spec, arguments)?;
    (spec.bind)(parsed).map(Invocation::Run)
}

fn asks_for_help(arguments: &[OsString]) -> bool {
    arguments
        .iter()
        .take_while(|argument| argument.as_os_str() != OsStr::new("--"))
        .any(|argument| argument == OsStr::new("--help") || argument == OsStr::new("-h"))
}

/// Every type the rules take is `Text` or parsed from it, so an argument that is not Unicode is of no type.
fn unicode(arguments: Vec<OsString>) -> Result<Vec<String>, UsageError> {
    arguments
        .into_iter()
        .enumerate()
        .map(|(index, argument)| {
            argument
                .into_string()
                .map_err(|_| UsageError::NotUnicode(index + 1))
        })
        .collect()
}

/// Step 4 and 5: the arguments after the command, left to right, against the command's table.
fn parameters(
    spec: &CommandSpec,
    mut arguments: impl Iterator<Item = String>,
) -> Result<Parsed, UsageError> {
    let mut parsed = Parsed::default();
    let mut positionals = spec.positionals.iter();
    let mut flags_ended = false;
    while let Some(argument) = arguments.next() {
        if !flags_ended && argument == "--" {
            flags_ended = true;
            continue;
        }
        let flag_name = if flags_ended {
            None
        } else {
            argument.strip_prefix("--")
        };
        let Some(flag_name) = flag_name else {
            let Some(positional) = positionals.next() else {
                return Err(UsageError::Unexpected(argument));
            };
            let value = positional
                .value
                .parse(argument)
                .ok_or(UsageError::PositionalNotA {
                    name: positional.name,
                    type_name: positional.value.name(),
                })?;
            parsed.push(positional.name, value);
            continue;
        };
        let Some(flag) = spec.flags.iter().find(|flag| flag.name == flag_name) else {
            return Err(UsageError::UnknownFlag(argument));
        };
        let (value_type, repeats) = match flag.kind {
            FlagKind::Switch => {
                if parsed.switch(flag.name) {
                    return Err(UsageError::GivenTwice(flag.name));
                }
                parsed.switch_on(flag.name);
                continue;
            }
            FlagKind::Option(value_type) => (value_type, false),
            FlagKind::Set(value_type) => (value_type, true),
        };
        // The next argument is the value, whatever it is.
        let given = arguments.next().ok_or(UsageError::NeedsValue(flag.name))?;
        if !repeats && parsed.has(flag.name) {
            return Err(UsageError::GivenTwice(flag.name));
        }
        let value = value_type.parse(given).ok_or(UsageError::FlagNotA {
            flag: flag.name,
            type_name: value_type.name(),
        })?;
        parsed.push(flag.name, value);
    }
    match positionals.next() {
        Some(missing) => Err(UsageError::Missing(missing.name)),
        None => Ok(parsed),
    }
}
