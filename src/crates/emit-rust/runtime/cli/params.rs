//! The shape of a derived parameter table (docs/cli-vocabulary.md, "Derived,
//! never written"). The table itself is generated per program.

use super::UsageError;

/// The parameter types one program's rules take, as the command line reads them.
pub trait ValueType: Copy + 'static {
    /// One parsed argument of any of these types.
    type Value;
    /// The type's name as the spec writes it, for "not a ...".
    fn name(self) -> &'static str;
    /// Parses and checks the type's claim at the edge; the argument moves into the value.
    fn parse(self, text: String) -> Option<Self::Value>;
}

/// A parameter without a default: `<name>`, by position.
#[derive(Clone, Copy, Debug)]
pub struct Positional<T: 'static> {
    pub name: &'static str,
    pub value: T,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlagKind<T> {
    /// `Bool = false`: `--name`, at most once.
    Switch,
    /// `Option[T] = none`, or any other default that is not a set: `--name <value>`, at most once.
    Single(T),
    /// `Set[T]` with a default: `--name <value>`, repeatable; repeats collapse.
    Set(T),
}

/// A parameter with a default: `--name`.
#[derive(Clone, Copy, Debug)]
pub struct Flag<T: 'static> {
    pub name: &'static str,
    pub kind: FlagKind<T>,
}

/// One `@on(cli("name"))` rule: its parameters, and how they become its event.
#[derive(Debug)]
pub struct CommandSpec<T: ValueType, C: 'static> {
    pub name: &'static str,
    pub positionals: &'static [Positional<T>],
    pub flags: &'static [Flag<T>],
    pub bind: fn(Parsed<T::Value>) -> Result<C, UsageError>,
}

/// The arguments of one command, each already of its parameter's type.
#[derive(Debug)]
pub struct Parsed<V> {
    values: Vec<(&'static str, V)>,
    switches: Vec<&'static str>,
}

impl<V> Default for Parsed<V> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            switches: Vec::new(),
        }
    }
}

impl<V> Parsed<V> {
    pub(super) fn push(&mut self, name: &'static str, value: V) {
        self.values.push((name, value));
    }

    pub(super) fn switch_on(&mut self, name: &'static str) {
        self.switches.push(name);
    }

    pub fn has(&self, name: &str) -> bool {
        self.values.iter().any(|(given, _)| *given == name)
    }

    pub fn switch(&self, name: &str) -> bool {
        self.switches.contains(&name)
    }

    /// The first value given under `name`, moved out.
    pub fn take(&mut self, name: &str) -> Option<V> {
        let at = self.values.iter().position(|(given, _)| *given == name)?;
        Some(self.values.remove(at).1)
    }

    /// Every value given under `name`, moved out, in the order given.
    pub fn take_all(&mut self, name: &str) -> Vec<V> {
        let (taken, kept) = std::mem::take(&mut self.values)
            .into_iter()
            .partition(|(given, _)| *given == name);
        self.values = kept;
        taken.into_iter().map(|(_, value)| value).collect()
    }
}

/// The parser has already reported a missing positional; this keeps a binder total without a panic.
pub fn required<T>(value: Option<T>, name: &'static str) -> Result<T, UsageError> {
    value.ok_or(UsageError::Missing(name))
}

/// An `Int` argument: ASCII digits only, one or more; leading zeros allowed.
/// A number beyond the word saturates to `i64::MAX`, which no store holds
/// (the store carries at most 2^53), so a rule that looks it up finds nothing.
pub fn int(text: &str) -> Option<i64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(text.bytes().fold(0i64, |n, b| n.saturating_mul(10).saturating_add(i64::from(b - b'0'))))
}
