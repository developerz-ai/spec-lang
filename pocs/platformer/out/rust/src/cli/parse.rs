// Projected by hand from the start rule's parameter level: LevelNo (src/hop/world.slg, src/hop/levels.slg) and the parsing order of contract section 2. Stand-in for spec-lang build output; change the spec, not this file.
//! The argument parser. Arguments are bytes: an argument is printed back exactly as given, UTF-8 or not.

use std::fmt;
use std::path::PathBuf;

use crate::levels::LevelNo;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Play,
    Run,
    Draw,
    Shot,
}

impl Command {
    fn named(name: &[u8]) -> Option<Self> {
        match name {
            b"play" => Some(Self::Play),
            b"run" => Some(Self::Run),
            b"draw" => Some(Self::Draw),
            b"shot" => Some(Self::Shot),
            _ => None,
        }
    }

    fn takes(self, flag: Flag) -> bool {
        match flag {
            Flag::Level => true,
            Flag::At => matches!(self, Self::Draw | Self::Shot),
            Flag::Out => matches!(self, Self::Shot),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flag {
    Level,
    At,
    Out,
}

impl Flag {
    fn named(arg: &[u8]) -> Option<Self> {
        match arg {
            b"--level" => Some(Self::Level),
            b"--at" => Some(Self::At),
            b"--out" => Some(Self::Out),
            _ => None,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Level => "--level",
            Self::At => "--at",
            Self::Out => "--out",
        }
    }
}

/// The options of one command, each at its default when not given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub level: LevelNo,
    /// `--at`: `None` is the end of the script. A value beyond `u64` is kept as `u64::MAX`: a script has at most
    /// 1,000,000 ticks, so every value from there on means the same stop.
    pub at: Option<u64>,
    pub out: Option<PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            level: LevelNo::One,
            at: None,
            out: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    pub command: Command,
    pub options: Options,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parsed {
    /// `--help` or `-h`: help to standard output, exit 0.
    Help,
    Run(Invocation),
}

/// One line on standard error, exit 2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsageError {
    /// No arguments: the help, on standard error.
    NoArguments,
    UnknownCommand(Vec<u8>),
    UnknownFlag(Vec<u8>),
    NeedsValue(&'static str),
    GivenTwice(&'static str),
    NotALevelNo,
    NotATick,
    NotAPath,
    Unexpected(Vec<u8>),
}

impl UsageError {
    /// The bytes written to standard error, `\n` included.
    pub fn message(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let quoted = |out: &mut Vec<u8>, head: &str, arg: &[u8]| {
            out.extend_from_slice(head.as_bytes());
            out.push(b'"');
            out.extend_from_slice(arg);
            out.extend_from_slice(b"\"\n");
        };
        match self {
            Self::NoArguments => out.extend_from_slice(super::HELP.as_bytes()),
            Self::UnknownCommand(arg) => quoted(&mut out, "error: unknown command ", arg),
            Self::UnknownFlag(arg) => {
                out.extend_from_slice(b"error: unknown flag ");
                out.extend_from_slice(arg);
                out.push(b'\n');
            }
            Self::Unexpected(arg) => quoted(&mut out, "error: unexpected argument ", arg),
            other => out.extend_from_slice(format!("error: {other}\n").as_bytes()),
        }
        out
    }
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoArguments => f.write_str("no arguments"),
            Self::UnknownCommand(a) => write!(f, "unknown command \"{}\"", a.escape_ascii()),
            Self::UnknownFlag(a) => write!(f, "unknown flag {}", a.escape_ascii()),
            Self::NeedsValue(flag) => write!(f, "{flag} needs a value"),
            Self::GivenTwice(flag) => write!(f, "{flag} given twice"),
            Self::NotALevelNo => f.write_str("--level: not a LevelNo"),
            Self::NotATick => f.write_str("--at: not a tick"),
            Self::NotAPath => f.write_str("--out: not a path"),
            Self::Unexpected(a) => write!(f, "unexpected argument \"{}\"", a.escape_ascii()),
        }
    }
}

/// ASCII digits only, at least one; the value saturates at `u64::MAX`.
fn digits(text: &[u8]) -> Option<u64> {
    if text.is_empty() || !text.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(text.iter().fold(0u64, |n, d| {
        n.saturating_mul(10).saturating_add(u64::from(d - b'0'))
    }))
}

/// `--level`: digits, value a `LevelNo`. Saturation keeps every long value out of `1..=2`.
fn level(text: &[u8]) -> Result<LevelNo, UsageError> {
    digits(text)
        .and_then(LevelNo::new)
        .ok_or(UsageError::NotALevelNo)
}

/// Contract section 2, "Parsing, in this order".
pub fn parse<A: AsRef<[u8]>>(args: &[A]) -> Result<Parsed, UsageError> {
    let Some((first, rest)) = args.split_first() else {
        return Err(UsageError::NoArguments);
    };
    if args.iter().any(|a| matches!(a.as_ref(), b"--help" | b"-h")) {
        return Ok(Parsed::Help);
    }
    let first = first.as_ref();
    let command =
        Command::named(first).ok_or_else(|| UsageError::UnknownCommand(first.to_vec()))?;
    let mut options = Options::default();
    let mut seen: Vec<Flag> = Vec::with_capacity(3);
    let mut rest = rest.iter().map(AsRef::as_ref);
    while let Some(arg) = rest.next() {
        if !arg.starts_with(b"--") {
            return Err(UsageError::Unexpected(arg.to_vec()));
        }
        let flag = Flag::named(arg)
            .filter(|f| command.takes(*f))
            .ok_or_else(|| UsageError::UnknownFlag(arg.to_vec()))?;
        let value = rest.next().ok_or(UsageError::NeedsValue(flag.name()))?;
        if seen.contains(&flag) {
            return Err(UsageError::GivenTwice(flag.name()));
        }
        seen.push(flag);
        match flag {
            Flag::Level => options.level = level(value)?,
            Flag::At => options.at = Some(digits(value).ok_or(UsageError::NotATick)?),
            Flag::Out => {
                if value.is_empty() {
                    return Err(UsageError::NotAPath);
                }
                use std::os::unix::ffi::OsStrExt;
                options.out = Some(PathBuf::from(std::ffi::OsStr::from_bytes(value)));
            }
        }
    }
    Ok(Parsed::Run(Invocation { command, options }))
}
