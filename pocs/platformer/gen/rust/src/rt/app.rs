//! An app's manifest as Rust data: the app, its libraries' notes, its parameters and every rule it reaches, each
//! with its parameters' types and a call from plain data. A library's runtime (`crate::libs::<name>`) reads it;
//! nothing here knows a library.

use super::data::{Data, Ty};
use super::{Int, R};
use crate::machine::{Machine, World};

#[derive(Debug)]
pub struct App {
    pub name: &'static str,
    pub qualified: &'static str,
    pub module: &'static str,
    pub target: Option<&'static str>,
    /// The library whose app shape it has.
    pub shape: Option<&'static str>,
    /// Its libraries' notes, in written order.
    pub notes: &'static [Note],
    /// `rules: [...]`, then the shape's parameters in the shape's order.
    pub params: &'static [AppParam],
    /// Every rule a named declaration reaches.
    pub rules: &'static [Rule],
    /// The `fn`s its parameters name, then every `fn` of one parameter of a foreign parameter's type.
    pub fns: &'static [Func],
    /// Every foreign declaration of the program.
    pub externs: &'static [Extern],
    /// Every state, in world order.
    pub states: &'static [State],
    /// Operations of handlers read as values (`out.text`).
    pub reads: &'static [Read],
    /// The world as data, and back; `None` when the data is not a world of these types.
    pub world_data: fn(&World) -> Data,
    pub data_world: fn(&Data) -> Option<World>,
    /// Whether every law of the program holds on this world alone.
    pub lawful: fn(&World) -> bool,
}

/// A note or trigger: `@bin("todo")`, `cli("add")`, `tick(60Hz)`.
#[derive(Debug)]
pub struct Note {
    pub library: &'static str,
    pub name: &'static str,
    pub args: &'static [NoteArg],
}

#[derive(Debug)]
pub struct NoteArg {
    pub param: &'static str,
    pub value: NoteValue,
}

#[derive(Debug)]
pub enum NoteValue {
    Int(Int),
    /// As written, escapes not processed.
    Text(&'static str),
    Quantity(Int, &'static str),
    Variant(&'static str, &'static [NoteArg]),
}

impl Note {
    pub fn arg(&self, param: &str) -> Option<&NoteValue> {
        self.args.iter().find(|a| a.param == param).map(|a| &a.value)
    }

    pub fn text(&self, param: &str) -> Option<&'static str> {
        match self.arg(param) {
            Some(NoteValue::Text(t)) => Some(t),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct AppParam {
    pub name: &'static str,
    pub decls: &'static [Decl],
}

/// A declaration an app parameter names: an index into `App::rules` or `App::fns`.
#[derive(Clone, Copy, Debug)]
pub enum Decl {
    Rule(usize),
    Fn(usize),
}

#[derive(Debug)]
pub struct Param {
    pub name: &'static str,
    pub ty: &'static Ty,
    pub has_default: bool,
}

#[derive(Debug)]
pub struct Rule {
    pub name: &'static str,
    pub qualified: &'static str,
    pub params: &'static [Param],
    pub result: &'static Ty,
    pub triggers: &'static [Note],
    /// `@uses` as written.
    pub uses: &'static [&'static str],
    /// Whether it may write a state (`@uses` names a state's `write`).
    pub writes: bool,
    /// One atomic step from outside: arguments in parameter order, `None` for a default.
    pub call: fn(&mut Machine, Vec<Option<Data>>) -> R<Data>,
}

/// `foreign c "raylib#DrawCube"`.
#[derive(Debug)]
pub struct Extern {
    pub library: &'static str,
    pub symbol: &'static str,
    pub params: &'static [Param],
}

#[derive(Debug)]
pub struct Func {
    pub name: &'static str,
    pub qualified: &'static str,
    /// `@why`.
    pub why: Option<&'static str>,
    pub params: &'static [Param],
    pub result: &'static Ty,
    pub call: fn(&mut Machine, Vec<Option<Data>>) -> R<Data>,
}

#[derive(Debug)]
pub struct State {
    pub name: &'static str,
    pub qualified: &'static str,
    pub ty: &'static Ty,
}

#[derive(Debug)]
pub struct Read {
    pub library: &'static str,
    pub handler: &'static str,
    pub op: &'static str,
    pub read: fn(&mut Machine) -> R<Data>,
}

impl App {
    /// The first note of this name.
    pub fn note(&self, name: &str) -> Option<&Note> {
        self.notes.iter().find(|n| n.name == name)
    }

    /// The declarations an app parameter names.
    pub fn param(&self, name: &str) -> &'static [Decl] {
        self.params.iter().find(|p| p.name == name).map_or(&[], |p| p.decls)
    }

    /// A handler's read operation (`out.text`).
    pub fn read(&self, handler: &str, op: &str) -> Option<&Read> {
        self.reads.iter().find(|r| r.handler == handler && r.op == op)
    }
}

/// An argument of a call as its parameter's Rust type: a missing required one stops.
pub fn arg<T: super::data::AsData>(args: &mut std::vec::IntoIter<Option<Data>>, name: &str) -> R<T> {
    match args.next().flatten() {
        Some(d) => T::from_data(&d)
            .ok_or_else(|| super::stop("E-TYPE-MISMATCH", format!("argument `{name}` is not of its type"))),
        None => Err(super::stop("E-TYPE-ARITY", format!("missing argument `{name}`"))),
    }
}

/// An argument of a parameter with a default: `None` takes the default.
pub fn arg_or_default<T: super::data::AsData>(args: &mut std::vec::IntoIter<Option<Data>>, name: &str) -> R<Option<T>> {
    match args.next().flatten() {
        Some(d) => T::from_data(&d)
            .map(Some)
            .ok_or_else(|| super::stop("E-TYPE-MISMATCH", format!("argument `{name}` is not of its type"))),
        None => Ok(None),
    }
}
