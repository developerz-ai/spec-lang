//! The declaration tables of a resolved program.

use crate::expr::{Block, Expect, Expr, Lambda};
use crate::ids::*;
use crate::library::{AppField, Handler, HandlerOp, Library, NoteUse, Trigger};
use crate::ty::Ty;
use spec_lang_syntax::{Location, SourceMap, Span};

/// A whole resolved program. Every table is indexed by its id type; every
/// declaration knows its module and span.
#[derive(Debug)]
pub struct Program {
    /// Every file read; spans index into it.
    pub sources: SourceMap,
    /// `modules[0]` is the built-in `core` module (no file).
    pub modules: Vec<Module>,
    pub types: Vec<TypeDecl>,
    pub consts: Vec<ConstDecl>,
    pub states: Vec<StateDecl>,
    pub fns: Vec<FnDecl>,
    pub rules: Vec<RuleDecl>,
    pub tests: Vec<TestDecl>,
    pub laws: Vec<LawDecl>,
    pub apps: Vec<AppDecl>,
    pub lambdas: Vec<Lambda>,
    /// The built-in `Option[T] = Some(value: T) | none`: tag 0 is `Some`, tag 1 `none`.
    pub option_type: TypeId,
    /// The built-in `Result[T] = Ok(value: T) | Err(error)`: tag 0 is `Ok`, tag 1 `Err`.
    /// A trace step's `reply` is one; `Err` holds the failure.
    pub result_type: TypeId,
    /// Rule-set types (`type Change = a | b;`) named inside a `Trace[...]`,
    /// which [`crate::Ty::Trace`] has already flattened into rules.
    pub rule_set_uses: Vec<TypeId>,
    /// Every library a `use` loaded, in load order ([`crate::library`]).
    pub libraries: Vec<Library>,
    /// Every handler of a loaded library, shared by the libraries that offer it.
    pub handlers: Vec<Handler>,
}

/// The tag of `Some` in [`Program::option_type`].
pub const SOME_TAG: u32 = 0;
/// The tag of `none` in [`Program::option_type`].
pub const NONE_TAG: u32 = 1;
/// The tag of `Ok` in [`Program::result_type`].
pub const OK_TAG: u32 = 0;
/// The tag of `Err` in [`Program::result_type`].
pub const ERR_TAG: u32 = 1;

impl Program {
    pub fn module(&self, id: ModuleId) -> &Module {
        &self.modules[id.index()]
    }
    pub fn ty(&self, id: TypeId) -> &TypeDecl {
        &self.types[id.index()]
    }
    pub fn constant(&self, id: ConstId) -> &ConstDecl {
        &self.consts[id.index()]
    }
    pub fn state(&self, id: StateId) -> &StateDecl {
        &self.states[id.index()]
    }
    pub fn func(&self, id: FnId) -> &FnDecl {
        &self.fns[id.index()]
    }
    pub fn rule(&self, id: RuleId) -> &RuleDecl {
        &self.rules[id.index()]
    }
    pub fn test(&self, id: TestId) -> &TestDecl {
        &self.tests[id.index()]
    }
    pub fn law(&self, id: LawId) -> &LawDecl {
        &self.laws[id.index()]
    }
    pub fn app(&self, id: AppId) -> &AppDecl {
        &self.apps[id.index()]
    }
    pub fn lambda(&self, id: LambdaId) -> &Lambda {
        &self.lambdas[id.index()]
    }

    /// What every declaration has, for any declaration.
    pub fn info(&self, d: DeclRef) -> &DeclInfo {
        match d {
            DeclRef::Type(i) => &self.ty(i).info,
            DeclRef::Const(i) => &self.constant(i).info,
            DeclRef::State(i) => &self.state(i).info,
            DeclRef::Fn(i) => &self.func(i).info,
            DeclRef::Rule(i) => &self.rule(i).info,
            DeclRef::Test(i) => &self.test(i).info,
            DeclRef::Law(i) => &self.law(i).info,
            DeclRef::App(i) => &self.app(i).info,
        }
    }

    /// A declaration by qualified name (`todo.board.done`) or by a dotted
    /// suffix of it (`board.done`, `done`) when exactly one matches.
    pub fn find_decl(&self, name: &str) -> Result<DeclRef, Vec<String>> {
        let suffix = format!(".{name}");
        let hits: Vec<DeclRef> = self
            .modules
            .iter()
            .skip(1)
            .flat_map(|m| m.decls.iter().copied())
            .filter(|d| {
                let q = self.qualified(self.info(*d));
                q == name || q.ends_with(&suffix)
            })
            .collect();
        match hits.as_slice() {
            [one] => Ok(*one),
            many => Err(many.iter().map(|d| self.qualified(self.info(*d))).collect()),
        }
    }

    /// `path:line:col` of a span.
    pub fn location(&self, span: Span) -> Location {
        self.sources.location(span)
    }

    /// The source text a span covers.
    pub fn snippet(&self, span: Span) -> &str {
        self.sources.slice(span)
    }

    /// A module by its dotted name.
    pub fn find_module(&self, name: &str) -> Option<&Module> {
        self.modules.iter().find(|m| m.name == name)
    }

    pub fn handler(&self, id: HandlerId) -> &Handler {
        &self.handlers[id.index()]
    }

    /// The operation an effect call or read names.
    pub fn effect_op(&self, e: crate::expr::Effect) -> Option<&HandlerOp> {
        self.handlers.get(e.handler.index())?.ops.get(e.op as usize)
    }

    /// `out.line`: an effect as written in source.
    pub fn effect_name(&self, e: crate::expr::Effect) -> String {
        let h = self.handler(e.handler);
        format!("{}.{}", h.name, self.effect_op(e).map(|o| o.name.as_str()).unwrap_or("?"))
    }

    /// A loaded library by name.
    pub fn library(&self, name: &str) -> Option<&Library> {
        self.libraries.iter().find(|l| l.name == name)
    }

    /// Every effect label a loaded library's handler adds (`screen.write`):
    /// what a `foreign` declaration may state.
    pub fn world_labels(&self) -> std::collections::BTreeSet<String> {
        self.handlers.iter().flat_map(|h| h.labels.iter().cloned()).collect()
    }

    /// `module.name` of a declaration, for reports.
    pub fn qualified(&self, info: &DeclInfo) -> String {
        format!("{}.{}", self.module(info.module).name, info.name)
    }
}

/// One `.slg` file. Declaration order carries no meaning but is kept for
/// reports and printers.
#[derive(Debug)]
pub struct Module {
    pub id: ModuleId,
    /// Dotted: `todo.model`.
    pub name: String,
    /// The file as given to the loader; empty for `core`.
    pub path: String,
    pub uses: Vec<UseItem>,
    /// Libraries named by `use` ([`crate::library`]).
    pub vocabularies: Vec<String>,
    pub decls: Vec<DeclRef>,
    /// The library whose `contract.slg` this module is; `None` for the project's own.
    pub library: Option<String>,
}

/// What one `use` bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UseItem {
    /// `use a.b;` binds `b` to module `a.b`.
    Module { alias: String, module: ModuleId, span: Span },
    /// `use a.b.c;` binds `c` to a declaration of `a.b` (and, for a sum type, its variants).
    Decl { name: String, decl: DeclRef, span: Span },
    /// `use cli;`: a library.
    Vocabulary { name: String, span: Span },
}

/// What every declaration has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclInfo {
    pub module: ModuleId,
    pub name: String,
    /// The whole declaration, notes included.
    pub span: Span,
    /// Its name.
    pub name_span: Span,
    pub is_pub: bool,
    /// `@why("...")`.
    pub why: Option<String>,
}

#[derive(Debug)]
pub struct TypeDecl {
    pub info: DeclInfo,
    pub def: TypeDef,
}

#[derive(Debug)]
pub enum TypeDef {
    /// `type Title = Text where is_title(self);`, `type SeatNo = 1..=200;`.
    Alias(Ty),
    /// `type Task(no: TaskNo, status: Status = Open);`.
    Record(Vec<Field>),
    /// `type Status = Open | Done;`, `type Seat = Free | Held(by: Id[User])`.
    Sum(Vec<Variant>),
    /// `type Change = board.add | board.done;`: a set of rules, for `Trace[Change]`.
    Rules(Vec<RuleId>),
}

#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub ty: Ty,
    /// Evaluated in the type's module when the field is not given.
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug)]
pub struct ConstDecl {
    pub info: DeclInfo,
    pub ty: Option<Ty>,
    pub value: Expr,
}

#[derive(Debug)]
pub struct StateDecl {
    pub info: DeclInfo,
    pub ty: Ty,
    /// `None`: starts empty (a `Map`, `Set`, `List` or `Option`).
    pub init: Option<Expr>,
}

/// A parameter of a `fn`, `rule`, `test` or `law`.
#[derive(Debug)]
pub struct Param {
    pub name: String,
    pub ty: Ty,
    /// Evaluated in the declaring module, with earlier parameters in scope.
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug)]
pub struct FnDecl {
    pub info: DeclInfo,
    pub params: Vec<Param>,
    pub result: Option<Ty>,
    /// The hole `?foreign` when [`FnDecl::foreign`] is set.
    pub body: Expr,
    pub cases: Vec<Case>,
    /// `@uses(...)` as written (`keys.read`).
    pub uses: Vec<String>,
    /// `= foreign c "raylib#IsKeyDown";`: the body lives outside spec-lang.
    pub foreign: Option<Foreign>,
}

/// `foreign <lang> "<symbol>"`: a body only a backend can run (card,
/// Functions: evidence starts at `assumed`). The interpreter blocks a check
/// that reaches one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Foreign {
    /// `c`, `rust`, `ts`, as written.
    pub lang: String,
    /// The text, as written: `raylib#IsKeyDown`.
    pub symbol: String,
    /// `foreign c "raylib#IsKeyDown"`.
    pub span: Span,
}

impl std::fmt::Display for Foreign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "foreign {} \"{}\"", self.lang, self.symbol)
    }
}

/// One `@cases` row: arguments in parameter order (`None`: the default) and
/// what the call must give.
#[derive(Debug)]
pub struct Case {
    pub args: Vec<Option<Expr>>,
    pub expect: Expect,
    /// The whole `args -> value` text, for reports.
    pub span: Span,
}

#[derive(Debug)]
pub struct RuleDecl {
    pub info: DeclInfo,
    pub params: Vec<Param>,
    pub result: Option<Ty>,
    /// Every `@guard` clause, in written order: the first that does not hold
    /// fails the call with its error, before the body runs.
    pub guards: Vec<Guard>,
    /// The hole `?foreign` when [`RuleDecl::foreign`] is set.
    pub body: Expr,
    /// `@uses(...)` as written (`tasks.write`); recorded, not checked.
    pub uses: Vec<String>,
    /// `@on(...)` with a library's trigger: `cli("add")`, `tick(60Hz)`.
    pub triggers: Vec<Trigger>,
    /// `= foreign c "raylib#DrawCube";`.
    pub foreign: Option<Foreign>,
}

/// `@guard(cond else Error)`.
#[derive(Debug)]
pub struct Guard {
    pub cond: Expr,
    pub error: String,
    pub span: Span,
}

#[derive(Debug)]
pub struct TestDecl {
    pub info: DeclInfo,
    /// Generated from their types when the test has no `@cases`: a property.
    pub params: Vec<Param>,
    pub given: Vec<Given>,
    pub cases: Vec<Case>,
    pub body: Block,
}

/// One `@given` item, applied left to right.
#[derive(Debug)]
pub enum Given {
    /// `state -> value`.
    State { state: StateId, value: Expr, span: Span },
    /// Another test's end world.
    Test { test: TestId, span: Span },
}

#[derive(Debug)]
pub struct LawDecl {
    pub info: DeclInfo,
    /// Every parameter; exactly one has type [`Ty::Trace`].
    pub params: Vec<Param>,
    pub body: Expr,
}

impl LawDecl {
    /// The trace parameter's index and the rules it names.
    pub fn trace(&self) -> Option<(usize, &[RuleId])> {
        self.params.iter().enumerate().find_map(|(i, p)| match &p.ty {
            Ty::Trace(rules) => Some((i, rules.as_slice())),
            _ => None,
        })
    }
}

/// `app Name(rules: [...])`, or an app with a library's shape, with its notes.
#[derive(Debug)]
pub struct AppDecl {
    pub info: DeclInfo,
    /// The rules it exposes.
    pub rules: Vec<RuleId>,
    /// `@target(rust)`.
    pub target: Option<String>,
    /// Notes of its libraries, in written order: `@bin("todo")`, `@window(960, 540, "Skyhop")`.
    pub notes: Vec<NoteUse>,
    /// The library whose app shape it has (`game`), if any.
    pub shape: Option<String>,
    /// The shape's parameters, in the shape's order: `start: world.start`.
    pub fields: Vec<AppField>,
}

impl AppDecl {
    /// Every declaration the app names: its rules, and its fields.
    pub fn named(&self) -> Vec<DeclRef> {
        let mut out: Vec<DeclRef> = self.rules.iter().map(|r| DeclRef::Rule(*r)).collect();
        out.extend(self.fields.iter().map(|f| f.decl));
        out
    }

    /// The library note `name`, if written.
    pub fn note(&self, name: &str) -> Option<&NoteUse> {
        self.notes.iter().find(|n| n.name == name)
    }

    /// The declaration the shape parameter `name` names.
    pub fn field(&self, name: &str) -> Option<DeclRef> {
        self.fields.iter().find(|f| f.name == name).map(|f| f.decl)
    }
}
