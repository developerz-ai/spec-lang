//! The declaration tables of a lowered program. Ids are the IR's: a
//! `TypeId` here names the same type as in `spec_lang_ir::Program`.

use crate::manifest::Manifest;
use crate::node::{Block, Expected, Expr};
use crate::ty::{ClaimId, DTy, LTy};
use spec_lang_ir::{ConstId, FnId, HandlerId, LawId, ModuleId, RuleId, StateId, TestId, TypeId};
use spec_lang_syntax::{SourceMap, Span};

/// A whole program, lowered.
#[derive(Debug)]
pub struct Lir {
    pub modules: Vec<Module>,
    pub types: Vec<TypeDecl>,
    /// Every `where` claim, as a function of `self`.
    pub claims: Vec<Claim>,
    pub consts: Vec<Const>,
    pub states: Vec<State>,
    pub fns: Vec<Func>,
    pub rules: Vec<Rule>,
    pub tests: Vec<Test>,
    pub laws: Vec<Law>,
    /// Every check, in the order and with the names `spec-lang test` uses.
    pub checks: Vec<Check>,
    /// One manifest per `app`.
    pub apps: Vec<Manifest>,
    /// Every handler of a loaded library, by `HandlerId`.
    pub handlers: Vec<Handler>,
    /// The built-in `Option[T] = Some(value: T) | none`.
    pub option_type: TypeId,
    /// The built-in `Result = Ok(value) | Err(error)` of trace replies.
    pub result_type: TypeId,
    /// Every library a `use` loaded, in load order: where a printer finds `real/<lang>/`.
    pub libraries: Vec<Library>,
    /// Every file read, so a printer can name `path:line` for a span.
    pub sources: SourceMap,
}

impl Lir {
    pub fn ty(&self, id: TypeId) -> &TypeDecl {
        &self.types[id.index()]
    }
    pub fn claim(&self, id: ClaimId) -> &Claim {
        &self.claims[id.index()]
    }
    pub fn constant(&self, id: ConstId) -> &Const {
        &self.consts[id.index()]
    }
    pub fn state(&self, id: StateId) -> &State {
        &self.states[id.index()]
    }
    pub fn func(&self, id: FnId) -> &Func {
        &self.fns[id.index()]
    }
    pub fn rule(&self, id: RuleId) -> &Rule {
        &self.rules[id.index()]
    }
    pub fn test(&self, id: TestId) -> &Test {
        &self.tests[id.index()]
    }
    pub fn law(&self, id: LawId) -> &Law {
        &self.laws[id.index()]
    }
    pub fn handler(&self, id: HandlerId) -> &Handler {
        &self.handlers[id.index()]
    }
}

/// A loaded library: its name and the directory that holds its `vocab.slg` and `real/`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Library {
    pub name: String,
    pub dir: String,
}

/// One source module, `modules[0]` the built-in `core`.
#[derive(Debug)]
pub struct Module {
    pub id: ModuleId,
    pub name: String,
    pub path: String,
    /// Vocabularies it `use`s, by name; passed through, never interpreted.
    pub vocabularies: Vec<String>,
}

/// What every declaration has.
#[derive(Clone, Debug)]
pub struct Info {
    pub module: ModuleId,
    pub name: String,
    /// `module.name`.
    pub qualified: String,
    pub is_pub: bool,
    pub why: Option<String>,
    pub span: Span,
}

#[derive(Debug)]
pub struct TypeDecl {
    pub info: Info,
    /// `Option` and `Result`: declared by the language, not the program.
    pub builtin: bool,
    pub def: TypeDef,
}

#[derive(Debug)]
pub enum TypeDef {
    /// `type Title = Text where is_title(self)`: no runtime form of its own;
    /// a value of it is a value of `of` that conforms.
    Alias {
        of: DTy,
    },
    Record {
        fields: Vec<Field>,
    },
    Sum {
        variants: Vec<Variant>,
    },
    /// `type Change = a | b;`: a set of rules, the type of their events.
    RuleSet {
        rules: Vec<RuleId>,
    },
}

#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub ty: LTy,
    pub check: DTy,
    /// A closed expression evaluated when the field is not given.
    pub default: Option<Expr>,
}

#[derive(Debug)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
}

/// A claim `T where p`: a `Bool` function of `self`. Conforming runs the base
/// check first, then the claim: true holds; false, a failure or a non-`Bool`
/// does not.
#[derive(Debug)]
pub struct Claim {
    pub module: ModuleId,
    pub self_ty: LTy,
    pub body: Expr,
    /// The claim as written, for messages.
    pub text: String,
}

#[derive(Debug)]
pub struct Const {
    pub info: Info,
    pub ty: LTy,
    /// The written type: checked by `spec-lang test`'s check stage, not at run time.
    pub declared: Option<DTy>,
    pub value: Expr,
}

#[derive(Debug)]
pub struct State {
    pub info: Info,
    pub ty: LTy,
    /// Every initial value and every write is checked against it.
    pub check: DTy,
    /// `None`: starts empty (`Map()`, `Set()`, `[]` or `none`).
    pub init: Option<Expr>,
}

#[derive(Debug)]
pub struct Param {
    pub name: String,
    pub ty: LTy,
    pub check: DTy,
    /// Evaluated when the argument is absent, with the earlier parameters in scope.
    pub default: Option<Expr>,
}

/// The body of a `fn` or rule.
#[derive(Debug)]
pub enum Body {
    Expr(Expr),
    /// `foreign <lang> "<library>#<symbol>"`: an external call the LIR names
    /// and never runs. The evaluator blocks a check that reaches it (`B-FOREIGN`).
    Extern(Extern),
}

#[derive(Clone, Debug)]
pub struct Extern {
    /// `c`, `rust`, ... as written.
    pub lang: String,
    /// The part before `#` (`raylib`); empty when the symbol has no `#`.
    pub library: String,
    /// The part after `#` (`DrawCube`).
    pub symbol: String,
    /// The world effects it performs, from `@uses` (`screen.write`).
    pub effects: Vec<String>,
}

/// A `fn`: prologue = fill defaults in order (earlier parameters in scope),
/// `conform` every parameter in order; then the body; then `conform` the result.
#[derive(Debug)]
pub struct Func {
    pub info: Info,
    pub params: Vec<Param>,
    pub result: LTy,
    pub result_check: Option<DTy>,
    pub body: Body,
    /// `@uses` as written.
    pub uses: Vec<String>,
}

/// A rule: a world transition. Prologue as a `fn`; then each guard in order:
/// false, or a failure while evaluating it, fails with the guard's error;
/// then the body; then `conform` the result. Called from outside a rule it
/// is atomic: on any failure the world and the effects it buffered are
/// restored; on success the buffered effects are sent. Called inside a rule
/// it joins that rule's step.
#[derive(Debug)]
pub struct Rule {
    pub info: Info,
    pub params: Vec<Param>,
    pub result: LTy,
    pub result_check: Option<DTy>,
    pub guards: Vec<Guard>,
    pub body: Body,
    pub uses: Vec<String>,
    /// `@on(...)` as data: `cli("add")`, `tick(60Hz)`.
    pub triggers: Vec<crate::manifest::Note>,
}

#[derive(Debug)]
pub struct Guard {
    pub cond: Expr,
    pub error: String,
}

#[derive(Debug)]
pub struct Test {
    pub info: Info,
    /// Generated from their declared types when the test is a property.
    pub params: Vec<Param>,
    pub given: Vec<Given>,
    pub body: Block,
}

/// One `@given` item, applied left to right to the initial world.
#[derive(Debug)]
pub enum Given {
    /// Evaluate in no scope, `conform` to the state's type, set it.
    State { state: StateId, value: Expr },
    /// The end world of another test (run once, remembered).
    Test(TestId),
}

#[derive(Debug)]
pub struct Law {
    pub info: Info,
    pub params: Vec<Param>,
    /// Which parameter is the trace.
    pub trace: usize,
    /// The rules the trace calls.
    pub traced: Vec<RuleId>,
    /// The rules that may build a reached start world: every rule of the
    /// traced rules' modules that cannot reach a foreign body.
    pub start_rules: Vec<RuleId>,
    pub body: Expr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckKind {
    Case,
    Test,
    Property,
    Law,
}

impl CheckKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CheckKind::Case => "case",
            CheckKind::Test => "test",
            CheckKind::Property => "property",
            CheckKind::Law => "law",
        }
    }
}

/// One check, named as `spec-lang test` names it: the declaration, and for a
/// `@cases` row ` case[i]`.
#[derive(Debug)]
pub struct Check {
    pub name: String,
    pub module: String,
    pub kind: CheckKind,
    /// The declaration, qualified.
    pub decl: String,
    pub what: CheckWhat,
}

#[derive(Debug)]
pub enum CheckWhat {
    /// One `@cases` row: call the target with the arguments (`None`: the
    /// default) and compare with what is expected.
    Case {
        target: CaseTarget,
        args: Vec<Option<Expr>>,
        expect: Expected,
    },
    /// A test without parameters: its `@given` world, then its steps.
    Test(TestId),
    /// A test with parameters, run with generated arguments.
    Property(TestId),
    Law(spec_lang_ir::LawId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaseTarget {
    Fn(FnId),
    /// A parameterised test with `@cases`: the row's values are its parameters.
    Test(TestId),
}

/// A library handler: a group of operations a library's runtime performs
/// (`out.line`), and what tests run instead (its pretend).
#[derive(Debug)]
pub struct Handler {
    pub name: String,
    pub library: String,
    /// The effect labels it adds (`out.write`).
    pub labels: Vec<String>,
    pub ops: Vec<HandlerOp>,
    pub pretend: Option<Pretend>,
}

/// Who may use an operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpKind {
    /// Called by rules: it changes the world outside.
    Rule,
    /// Called by any body.
    Fn,
    /// Read by tests only.
    Test,
}

impl OpKind {
    pub fn as_str(self) -> &'static str {
        match self {
            OpKind::Rule => "rule",
            OpKind::Fn => "fn",
            OpKind::Test => "test",
        }
    }
}

#[derive(Debug)]
pub struct HandlerOp {
    pub name: String,
    pub kind: OpKind,
    /// `None`: read as a value (`out.text`).
    pub params: Option<Vec<Param>>,
    pub result: LTy,
    pub uses: Vec<String>,
}

/// What tests run for a handler: state (locals the arms may change, kept
/// between calls, committed and rolled back with the world, fresh in every
/// test), and one arm per operation.
#[derive(Debug)]
pub struct Pretend {
    pub name: String,
    pub state: Vec<Param>,
    /// Per operation, in the handler's order; `None`: not implemented
    /// (reaching it stops `E-UNSUPPORTED`).
    pub arms: Vec<Option<PretendArm>>,
}

/// An arm: the operation's arguments `conform` to its parameters, are
/// bound to these names beside the state, and the body runs; the state's
/// locals as they end are the new state; the body's value is the reply.
#[derive(Debug)]
pub struct PretendArm {
    pub params: Vec<(String, LTy)>,
    pub body: Expr,
}
