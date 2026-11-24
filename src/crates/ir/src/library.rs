//! Libraries a program loads: what `use <name>;` adds when no module of the
//! project has that name (decisions 008 and 009).
//!
//! A library is the directory `libs/<name>/`, found on the search path
//! ([`LibPath`]). The toolchain reads three files of it and knows nothing of
//! any library by name:
//!
//! * `vocab.slg`: the notes (`@allows(app) note bin(name: Text);`), triggers
//!   (`@allows(on) note cli(command: Text);`), handlers with their operations
//!   (`handler out { rule line(text: Text); test text -> Text; }`), types for
//!   note parameters, at most one app shape (`app Game(start: rule, ...)`),
//!   and `use other.handler;` to offer another library's handler;
//! * `contract.slg`: ordinary declarations, usually `foreign`, loaded as the
//!   module `<name>` when it declares anything;
//! * `pretend.slg`: `handler Name(state) = { op -> reply; ... }`, what the
//!   engine runs for each operation in tests.
//!
//! What a note, trigger or app shape means beyond its form belongs to the
//! library's runtime (`real/`), which reads the app as data.

use crate::expr::Expr;
use crate::ids::{DeclRef, HandlerId, ModuleId};
use crate::program::Param;
use crate::ty::Ty;
use spec_lang_syntax::Span;
use std::path::{Path, PathBuf};

/// Where `use <name>;` looks for `libs/<name>/`, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibPath {
    pub dirs: Vec<PathBuf>,
}

/// The environment variable that adds library roots: directories that hold
/// `<name>/vocab.slg`, separated like `PATH`.
pub const LIBS_ENV: &str = "SPEC_LANG_LIBS";

impl LibPath {
    /// The search path for a project at `project` (a directory or a file):
    /// first every `libs/` directory in the project's ancestors, nearest first
    /// (the project's own directory is not searched: its files are modules);
    /// then each directory of [`LIBS_ENV`]; then [`LibPath::builtin`].
    pub fn for_project(project: &Path) -> Self {
        let start = if project.is_file() { project.parent() } else { Some(project) };
        let mut dirs: Vec<PathBuf> = Vec::new();
        let absolute = start.map(|s| std::fs::canonicalize(s).unwrap_or_else(|_| s.to_path_buf()));
        for dir in absolute.iter().flat_map(|s| s.ancestors().skip(1)) {
            let libs = dir.join("libs");
            if libs.is_dir() {
                dirs.push(libs);
            }
        }
        let mut path = LibPath { dirs };
        path.dirs.extend(Self::from_env().dirs);
        path
    }

    /// The search path without a project: [`LIBS_ENV`], then [`LibPath::builtin`].
    pub fn from_env() -> Self {
        let mut dirs: Vec<PathBuf> =
            std::env::var_os(LIBS_ENV).map(|v| std::env::split_paths(&v).collect()).unwrap_or_default();
        dirs.push(Self::builtin());
        LibPath { dirs }
    }

    /// The `libs/` of the repository this toolchain was built from.
    /// Temporary: until packages come over git (decision 007), the libraries
    /// travel with the toolchain's checkout.
    pub fn builtin() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../libs")
    }

    /// The directory of library `name`: the first root holding `name/vocab.slg`.
    pub fn find(&self, name: &str) -> Option<PathBuf> {
        self.dirs.iter().map(|d| d.join(name)).find(|d| d.join("vocab.slg").is_file())
    }
}

/// One loaded library.
#[derive(Debug)]
pub struct Library {
    pub name: String,
    /// The directory it was loaded from, as found.
    pub dir: String,
    /// Its notes and triggers. A trigger is a note whose `allows` is `on`.
    pub notes: Vec<NoteSig>,
    /// The handlers a module that `use`s it may call: its own, then the ones
    /// it offers from other libraries (`use cli.out;`).
    pub handlers: Vec<HandlerId>,
    /// `app Game(start: rule, ...)`: the parameters an app takes in a module
    /// that uses this library.
    pub app: Option<AppShape>,
    /// `contract.slg`, when it declares anything: the module `<name>`.
    pub contract: Option<ModuleId>,
}

impl Library {
    /// A note or trigger by name that may stand on `kind` (`on` for a trigger).
    pub fn note(&self, name: &str) -> Option<&NoteSig> {
        self.notes.iter().find(|n| n.name == name)
    }
}

/// `@allows(app) note window(width: 1..=16384, height: 1..=16384, title: Text);`
#[derive(Clone, Debug)]
pub struct NoteSig {
    pub library: String,
    pub name: String,
    /// The kinds it may stand on; `on` for a trigger inside `@on(...)`.
    pub allows: Vec<String>,
    pub params: Vec<NoteParam>,
    pub span: Span,
}

impl NoteSig {
    /// `@window(width: 1..=16384, height: 1..=16384, title: Text)`, for messages.
    pub fn form(&self) -> String {
        let params: Vec<String> = self.params.iter().map(|p| format!("{}: {}", p.name, p.ty)).collect();
        if self.allows.iter().any(|a| a == "on") {
            format!("{}({})", self.name, params.join(", "))
        } else {
            format!("@{}({})", self.name, params.join(", "))
        }
    }
}

#[derive(Clone, Debug)]
pub struct NoteParam {
    pub name: String,
    pub ty: NoteTy,
}

/// The types a note parameter may have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoteTy {
    Text,
    Int,
    /// `1..=16384`, or with a unit, `1Hz..=1000Hz`: a whole number in range.
    Range {
        lo: i128,
        hi: i128,
        unit: Option<String>,
    },
    /// A sum declared by the library: `type Store = file(path: Text);`.
    Choice {
        name: String,
        variants: Vec<NoteVariant>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteVariant {
    pub name: String,
    pub params: Vec<(String, NoteTy)>,
}

impl std::fmt::Display for NoteTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoteTy::Text => write!(f, "Text"),
            NoteTy::Int => write!(f, "Int"),
            NoteTy::Range { lo, hi, unit } => {
                let u = unit.as_deref().unwrap_or("");
                write!(f, "{lo}{u}..={hi}{u}")
            }
            NoteTy::Choice { name, .. } => write!(f, "{name}"),
        }
    }
}

/// A use of a library's note or trigger, its arguments checked against the
/// [`NoteSig`]: `@bin("todo")`, `@on(cli("add"))`, `@on(tick(60Hz))`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteUse {
    pub library: String,
    pub name: String,
    /// In parameter order, every parameter given.
    pub args: Vec<NoteArg>,
    pub span: Span,
}

/// What calls a rule from outside (`@on(...)`): a trigger of a library.
pub type Trigger = NoteUse;

impl NoteUse {
    pub fn arg(&self, param: &str) -> Option<&NoteValue> {
        self.args.iter().find(|a| a.param == param).map(|a| &a.value)
    }

    /// The text argument `param`, if it is one.
    pub fn text(&self, param: &str) -> Option<&str> {
        match self.arg(param) {
            Some(NoteValue::Text(t)) => Some(t),
            _ => None,
        }
    }
}

impl std::fmt::Display for NoteUse {
    /// As written, arguments by position: `bin("todo")`, `store(file(".todo.json"))`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args: Vec<String> = self.args.iter().map(|a| a.value.to_string()).collect();
        write!(f, "{}({})", self.name, args.join(", "))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteArg {
    pub param: String,
    pub value: NoteValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoteValue {
    Int(i128),
    /// A whole number with a unit: `60Hz`.
    Num {
        value: i128,
        unit: String,
    },
    /// The text as written, without escapes processed.
    Text(String),
    /// A variant of a library type: `file(".todo.json")`.
    Variant {
        name: String,
        args: Vec<NoteArg>,
    },
}

impl std::fmt::Display for NoteValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoteValue::Int(n) => write!(f, "{n}"),
            NoteValue::Num { value, unit } => write!(f, "{value}{unit}"),
            NoteValue::Text(t) => write!(f, "\"{t}\""),
            NoteValue::Variant { name, args } if args.is_empty() => write!(f, "{name}"),
            NoteValue::Variant { name, args } => {
                let args: Vec<String> = args.iter().map(|a| a.value.to_string()).collect();
                write!(f, "{name}({})", args.join(", "))
            }
        }
    }
}

/// `app Game(start: rule, @on(tick) tick: rule, input: fn, ...)` in a vocabulary.
#[derive(Clone, Debug)]
pub struct AppShape {
    pub library: String,
    pub name: String,
    pub params: Vec<ShapeParam>,
}

#[derive(Clone, Debug)]
pub struct ShapeParam {
    pub name: String,
    /// `rule` or `fn`: the kind of declaration it names.
    pub kind: String,
    /// `@on(tick)`: the rule it names carries this trigger of the library.
    pub trigger: Option<String>,
}

/// One parameter of an app that has a library's shape: `tick: world.step`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppField {
    pub name: String,
    pub decl: DeclRef,
    pub span: Span,
}

/// `handler out { rule line(text: Text); test text -> Text; }`: a group of
/// effects a library adds, its operations, and its pretend.
#[derive(Debug)]
pub struct Handler {
    pub name: String,
    pub library: String,
    /// `@uses(out.write)` on the handler: the effect labels it adds. A
    /// `foreign` declaration may state these and only these.
    pub labels: Vec<String>,
    pub ops: Vec<HandlerOp>,
    /// What the engine runs; `None` when the library has no pretend for it.
    pub pretend: Option<Pretend>,
    pub span: Span,
}

#[derive(Debug)]
pub struct HandlerOp {
    pub name: String,
    pub kind: OpKind,
    /// `None`: read as a value (`out.text`); `Some`: called (`out.line(t)`).
    pub params: Option<Vec<Param>>,
    pub result: Option<Ty>,
    /// `@uses(...)` on the operation: the effects a call has.
    pub uses: Vec<String>,
    pub span: Span,
}

/// Who may use an operation, from the kind it is declared with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpKind {
    /// Called by rules only: it changes the world outside.
    Rule,
    /// Called by any body.
    Fn,
    /// Read by tests only: a test handler.
    Test,
}

/// `handler PretendOut(sent: mut List[Text] = []) = { out.line(text) -> ...; }`.
#[derive(Debug)]
pub struct Pretend {
    pub name: String,
    /// Its state: parameters with their starting values. A rule called from
    /// outside commits or rolls back this state with the world.
    pub state: Vec<Param>,
    /// Per operation of the handler, in its order: the parameter names the
    /// arm binds and its body. `None`: the pretend does not implement it.
    pub arms: Vec<Option<PretendArm>>,
    pub span: Span,
}

#[derive(Debug)]
pub struct PretendArm {
    pub params: Vec<String>,
    pub body: Expr,
    pub span: Span,
}
