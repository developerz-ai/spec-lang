//! An app's manifest: plain data a library's runtime reads. Nothing here
//! knows a library: notes, triggers and shape parameters pass through as written.

use spec_lang_ir::{FnId, RuleId};

/// A library's note or trigger with constant arguments: `bin("todo")`,
/// `store(file(".todo.json"))`, `cli("add")`, `tick(60Hz)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// The library that declares it (`cli`).
    pub library: String,
    pub name: String,
    /// Every parameter, in declared order.
    pub args: Vec<NoteArg>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteArg {
    pub param: String,
    pub value: NoteValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoteValue {
    Int(i128),
    /// As written, escapes not processed.
    Text(String),
    /// A whole number with a unit: `60Hz`.
    Quantity {
        value: i128,
        unit: String,
    },
    /// A variant of a library type: `file(".todo.json")`.
    Variant {
        name: String,
        args: Vec<NoteArg>,
    },
}

/// A declaration an app parameter names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Named {
    Rule(RuleId),
    Fn(FnId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppParam {
    pub name: String,
    pub decls: Vec<Named>,
}

/// One `app`.
#[derive(Clone, Debug)]
pub struct Manifest {
    pub name: String,
    pub qualified: String,
    pub module: String,
    /// `@target(x)`: which printer builds it.
    pub target: Option<String>,
    /// The library whose app shape it has, if any.
    pub shape: Option<String>,
    /// Its libraries' notes, in written order.
    pub notes: Vec<Note>,
    /// `rules: [...]`, then the shape's parameters in the shape's order.
    pub params: Vec<AppParam>,
    /// Every rule a named declaration can reach through calls, the named
    /// ones included, in id order. Their parameters, types, defaults,
    /// triggers and notes are in [`crate::Lir::rules`].
    pub rules: Vec<RuleId>,
}
