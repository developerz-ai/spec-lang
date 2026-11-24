//! The typed tree every backend prints from: the IR with a type on every
//! expression, core-library members resolved to [`Op`]s, defaults filled in,
//! and every change between a refined type and its base made explicit
//! ([`K::Widen`], [`K::Narrow`]).

use spec_lang_ir::{CmpOp, ConstId, FnId, RuleId, StateId, TypeId};
use spec_lang_syntax::Span;

/// A type as the backends see it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum T {
    Unit,
    Bool,
    Int,
    /// An element of `Text.bytes`: an `Int` from 0 to 255.
    Byte,
    Text,
    /// A declared record, sum or alias (a refined type such as `Title`).
    Named(TypeId),
    Option(Box<T>),
    List(Box<T>),
    Set(Box<T>),
    Map(Box<T>, Box<T>),
    Tuple(Vec<T>),
    /// The value of a rule call in a trace: a reply, or a refusal.
    Reply,
    /// A whole world: every state of the program.
    World,
    /// One call of a rule in a trace, as an event.
    Event(Vec<RuleId>),
    Step(Vec<RuleId>),
    Trace(Vec<RuleId>),
}

impl T {
    pub fn elem(&self) -> Option<&T> {
        match self {
            T::List(e) | T::Set(e) | T::Option(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TExpr {
    pub kind: K,
    pub ty: T,
    pub span: Span,
}

/// Core library operations, resolved on the receiver's type. The receiver is
/// the first operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// `Text.size`, `List.size`, `Set.size`, `Map.size`.
    Size,
    Trim,
    /// `Text.bytes`.
    Bytes,
    /// `Map.keys`, in ascending order.
    Keys,
    /// `Map.values`, in ascending key order.
    Values,
    /// `x in coll`: a set or list element, or a map key. Operands: element, collection.
    Contains,
    /// `xs.all(p)`.
    All,
    Any,
    Filter,
    Map,
    /// `xs.sort(key)`: stable, ascending by the key.
    Sort,
    /// `texts.join(sep)`.
    Join,
    /// `m.get(k)`: `Option` of the value.
    Get,
    /// `t.states`: every world of a trace, the first included.
    States,
    /// `t.steps`.
    Steps,
    /// `t.contains(x)` on `Text`: a substring.
    TextContains,
    /// `t.starts_with(x)` on `Text`.
    StartsWith,
    /// `s.list` on a `Set`: its elements, ascending.
    List,
}

#[derive(Clone, Debug, PartialEq)]
pub enum K {
    Unit,
    Bool(bool),
    Int(i128),
    Text(String),
    Interp(Vec<Part>),
    Local(String),
    Const(ConstId),
    State(StateId),
    /// A variant without fields (`Open`, `none`).
    Variant {
        ty: TypeId,
        tag: u32,
    },
    Some(Box<TExpr>),
    /// A record value: every field in declaration order, defaults filled in.
    Record {
        ty: TypeId,
        fields: Vec<TExpr>,
    },
    CallFn {
        f: FnId,
        args: Vec<TExpr>,
    },
    /// A rule called from a test: every parameter, defaults filled in.
    CallRule {
        rule: RuleId,
        args: Vec<TExpr>,
    },
    /// `out.line(text)`.
    OutLine(Box<TExpr>),
    /// `out.text` in a test.
    OutText,
    /// A record field, by index.
    Field(Box<TExpr>, usize),
    Core(Op, Vec<TExpr>),
    /// `x => body`; a lambda's parameter is a local.
    Lambda {
        param: String,
        body: Box<TExpr>,
    },
    /// `m[k]`: fails `Missing` when absent.
    Index {
        map: Box<TExpr>,
        key: Box<TExpr>,
    },
    Not(Box<TExpr>),
    Neg(Box<TExpr>),
    Bin(Bin, Box<TExpr>, Box<TExpr>),
    /// `a < b <= c`: every link holds.
    Compare(Box<TExpr>, Vec<(CmpOp, TExpr)>),
    Tuple(Vec<TExpr>),
    List(Vec<TExpr>),
    SetOf(Vec<TExpr>),
    MapOf(Vec<(TExpr, TExpr)>),
    If {
        cond: Box<TExpr>,
        then: Box<TExpr>,
        els: Box<TExpr>,
    },
    Match {
        scrutinee: Box<TExpr>,
        arms: Vec<(Pat, TExpr)>,
    },
    /// A refined value used as its base: `TaskNo` as `Int`, a `Byte` as `Int`.
    Widen(Box<TExpr>),
    /// A base value admitted to a refined type ([`T::Named`] of an alias).
    Narrow {
        expr: Box<TExpr>,
        proof: Proof,
    },
    /// `step.before`, `step.after`, `step.event`, `step.reply`.
    StepPart(Box<TExpr>, StepPart),
    /// A state of a world value: `w.tasks`.
    WorldState(Box<TExpr>, StateId),
    /// A parameter of the event of a step: `s.event.no`.
    EventArg {
        event: Box<TExpr>,
        name: String,
    },
    /// `s.event.is(board.clear())`.
    IsEvent {
        event: Box<TExpr>,
        rule: RuleId,
    },
    /// `s.reply.is(fail(_))`.
    IsFail(Box<TExpr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepPart {
    Before,
    After,
    Event,
    Reply,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bin {
    Add,
    Sub,
    Mul,
    And,
    Or,
    Implies,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Lit(String),
    Expr(TExpr),
}

/// Why a narrowing holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Proof {
    /// Discharged at build time; the text names the fact.
    Static(String),
    /// Checked when it runs; only allowed where a failure has somewhere to go (tests).
    Runtime,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pat {
    Wild,
    Bind(String),
    Variant { ty: TypeId, tag: u32, args: Vec<Pat> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TStmt {
    pub kind: S,
    pub span: Span,
}

/// One step of a place a statement writes.
#[derive(Clone, Debug, PartialEq)]
pub enum PathStep {
    Index(TExpr),
    Field(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub enum S {
    Let {
        name: String,
        value: TExpr,
    },
    /// `state(path) = value`.
    Assign {
        state: StateId,
        path: Vec<PathStep>,
        value: TExpr,
    },
    /// `state.remove(key)` on a map.
    Remove {
        state: StateId,
        key: TExpr,
    },
    Expr(TExpr),
    Return(Option<TExpr>),
    Assert(TExpr),
    For {
        var: String,
        iter: TExpr,
        body: Vec<TStmt>,
    },
    If {
        cond: TExpr,
        then: Vec<TStmt>,
        els: Vec<TStmt>,
    },
    /// `call -> value;` (a rule call in a test).
    ExpectValue {
        call: TExpr,
        value: TExpr,
    },
    /// `call -> fail Error;`.
    ExpectFail {
        call: TExpr,
        error: String,
    },
}
