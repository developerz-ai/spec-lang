//! Commands from the manifest: each rule with `@on(cli("name"))`, its parameters derived as the `cli` card says
//! (a parameter without a default is positional; `Bool` a switch; `Option[T]` a flag taking a value once; a
//! `Set[T]` with a default a repeatable flag; any other default a flag taking a value once), help, and parsing in
//! the order of the todo POC's contract, section 2.

use crate::rt::app::{App, Decl, Rule};
use crate::rt::data::{Data, Shape, Ty};
use std::ffi::{OsStr, OsString};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Positional,
    Switch,
    /// A value, at most once; `wrap` when the parameter is an `Option` (the value is `Some`).
    Single {
        wrap: bool,
    },
    /// A value, repeatable; repeats collapse.
    Repeat,
}

#[derive(Debug)]
pub struct Arg {
    pub name: &'static str,
    pub kind: Kind,
    /// The type of one value given on the command line.
    pub ty: &'static Ty,
}

#[derive(Debug)]
pub struct Command {
    pub name: &'static str,
    pub rule: &'static Rule,
    /// One per parameter of the rule, in order.
    pub args: Vec<Arg>,
}

/// Every command, in the order the app names its rules.
pub fn commands(app: &'static App) -> Vec<Command> {
    let mut out = Vec::new();
    for d in app.param("rules") {
        let Decl::Rule(i) = d else { continue };
        let Some(rule) = app.rules.get(*i) else { continue };
        let Some(name) = rule.triggers.iter().find(|t| t.name == "cli").and_then(|t| t.text("command")) else {
            continue;
        };
        let args = rule
            .params
            .iter()
            .map(|p| {
                let (kind, ty) = match (&p.ty.shape, p.has_default) {
                    (_, false) => (Kind::Positional, p.ty),
                    (Shape::Bool, true) => (Kind::Switch, p.ty),
                    (Shape::Option(t), true) => (Kind::Single { wrap: true }, *t),
                    (Shape::Set(t), true) => (Kind::Repeat, *t),
                    (_, true) => (Kind::Single { wrap: false }, p.ty),
                };
                Arg { name: p.name, kind, ty }
            })
            .collect();
        out.push(Command { name, rule, args });
    }
    out
}

/// A value's placeholder in help: the variants of a sum without data, else the type's name in lower case.
fn placeholder(ty: &Ty) -> String {
    match &ty.shape {
        Shape::Sum(vs) if vs.iter().all(|(_, f)| f.is_empty()) => {
            vs.iter().map(|(n, _)| n.to_lowercase()).collect::<Vec<_>>().join("|")
        }
        _ => format!("<{}>", ty.name.to_lowercase()),
    }
}

/// The help text: one line per command.
pub fn help(bin: &str, commands: &[Command]) -> String {
    let mut s = format!("usage: {bin} <command> [args]\n\ncommands:\n");
    for c in commands {
        s.push_str("  ");
        s.push_str(c.name);
        for a in &c.args {
            match a.kind {
                Kind::Positional => s.push_str(&format!(" <{}>", a.name)),
                Kind::Switch => s.push_str(&format!(" [--{}]", a.name)),
                Kind::Single { .. } => s.push_str(&format!(" [--{} {}]", a.name, placeholder(a.ty))),
                Kind::Repeat => s.push_str(&format!(" [--{} {}]...", a.name, placeholder(a.ty))),
            }
        }
        s.push('\n');
    }
    s
}

/// One word as a value of `ty`, its claim checked: `Int` from ASCII digits (beyond the range, the largest
/// `Int`, which no world holds), `Text` as given, `Bool` as `true`/`false`, a sum without data by a variant's
/// name in lower case.
pub fn value(ty: &'static Ty, word: &str) -> Option<Data> {
    let d =
        match &ty.shape {
            Shape::Int => {
                if word.is_empty() || !word.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                Data::Int(word.bytes().fold(0, |n: crate::rt::Int, b| {
                    n.saturating_mul(10).saturating_add(crate::rt::Int::from(b - b'0'))
                }))
            }
            Shape::Text => Data::Text(word.to_string()),
            Shape::Bool => match word {
                "true" => Data::Bool(true),
                "false" => Data::Bool(false),
                _ => return None,
            },
            Shape::Sum(vs) => {
                let (n, _) = vs.iter().find(|(n, f)| f.is_empty() && n.to_lowercase() == word)?;
                Data::Variant(n, Vec::new())
            }
            _ => return None,
        };
    (ty.admits)(&d).then_some(d)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Usage {
    NotUnicode(usize),
    UnknownCommand(String),
    UnknownFlag(String),
    GivenTwice(String),
    NeedsValue(String),
    FlagNotA(String, String),
    Unexpected(String),
    PositionalNotA(String, String),
    Missing(String),
}

impl fmt::Display for Usage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Usage::NotUnicode(i) => write!(f, "argument {i}: not UTF-8"),
            Usage::UnknownCommand(c) => write!(f, "unknown command \"{c}\""),
            Usage::UnknownFlag(a) => write!(f, "unknown flag {a}"),
            Usage::GivenTwice(n) => write!(f, "--{n} given twice"),
            Usage::NeedsValue(n) => write!(f, "--{n} needs a value"),
            Usage::FlagNotA(n, t) => write!(f, "--{n}: not a {t}"),
            Usage::Unexpected(a) => write!(f, "unexpected argument \"{a}\""),
            Usage::PositionalNotA(n, t) => write!(f, "<{n}>: not a {t}"),
            Usage::Missing(n) => write!(f, "missing <{n}>"),
        }
    }
}

#[derive(Debug)]
pub enum Invocation<'c> {
    NoArguments,
    Help,
    /// The command and one argument per parameter, `None` for its default.
    Run(&'c Command, Vec<Option<Data>>),
}

/// Steps 1 to 5 of the contract; the first problem met is the one reported.
pub fn parse(arguments: Vec<OsString>, commands: &[Command]) -> Result<Invocation<'_>, Usage> {
    if arguments.is_empty() {
        return Ok(Invocation::NoArguments);
    }
    if arguments.iter().take_while(|a| a.as_os_str() != OsStr::new("--")).any(|a| a == "--help" || a == "-h") {
        return Ok(Invocation::Help);
    }
    let mut words = Vec::with_capacity(arguments.len());
    for (i, a) in arguments.into_iter().enumerate() {
        words.push(a.into_string().map_err(|_| Usage::NotUnicode(i + 1))?);
    }
    let mut words = words.into_iter();
    let Some(name) = words.next() else { return Ok(Invocation::NoArguments) };
    let Some(command) = commands.iter().find(|c| c.name == name) else {
        return Err(Usage::UnknownCommand(name));
    };
    let mut values: Vec<Option<Data>> = vec![None; command.args.len()];
    let mut repeats: Vec<Vec<Data>> = vec![Vec::new(); command.args.len()];
    let mut seen = vec![false; command.args.len()];
    let mut positionals = command.args.iter().enumerate().filter(|(_, a)| a.kind == Kind::Positional);
    let mut flags_ended = false;
    while let Some(word) = words.next() {
        if !flags_ended && word == "--" {
            flags_ended = true;
            continue;
        }
        let flag = if flags_ended { None } else { word.strip_prefix("--") };
        let Some(flag) = flag else {
            let Some((i, a)) = positionals.next() else { return Err(Usage::Unexpected(word)) };
            let v = value(a.ty, &word).ok_or_else(|| Usage::PositionalNotA(a.name.into(), a.ty.name.into()))?;
            values[i] = Some(v);
            continue;
        };
        let Some((i, a)) = command.args.iter().enumerate().find(|(_, a)| a.kind != Kind::Positional && a.name == flag)
        else {
            return Err(Usage::UnknownFlag(word));
        };
        if a.kind == Kind::Switch {
            if seen[i] {
                return Err(Usage::GivenTwice(a.name.into()));
            }
            seen[i] = true;
            values[i] = Some(Data::Bool(true));
            continue;
        }
        // The next argument is the value, whatever it is.
        let given = words.next().ok_or_else(|| Usage::NeedsValue(a.name.into()))?;
        if a.kind != Kind::Repeat && seen[i] {
            return Err(Usage::GivenTwice(a.name.into()));
        }
        seen[i] = true;
        let v = value(a.ty, &given).ok_or_else(|| Usage::FlagNotA(a.name.into(), a.ty.name.into()))?;
        match a.kind {
            Kind::Single { wrap: true } => values[i] = Some(Data::Some(Box::new(v))),
            Kind::Repeat => {
                if !repeats[i].contains(&v) {
                    repeats[i].push(v);
                }
            }
            _ => values[i] = Some(v),
        }
    }
    if let Some((_, missing)) = positionals.next() {
        return Err(Usage::Missing(missing.name.into()));
    }
    for (i, mut r) in repeats.into_iter().enumerate() {
        if seen[i] && command.args[i].kind == Kind::Repeat {
            r.sort_by(order);
            values[i] = Some(Data::Set(r));
        }
    }
    Ok(Invocation::Run(command, values))
}

/// The order of the values a command line gives: numbers by value, text by code point.
fn order(a: &Data, b: &Data) -> std::cmp::Ordering {
    match (a, b) {
        (Data::Int(x), Data::Int(y)) => x.cmp(y),
        (Data::Text(x), Data::Text(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    }
}
