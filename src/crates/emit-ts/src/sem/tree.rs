//! The typed tree both emitters print from.
//!
//! The IR resolves names but gives expressions no types, so `x.size` is a
//! lookup on a value. A printer cannot work that way: `size` of a text, a list
//! and a map are three different target expressions. [`super::infer`] turns IR
//! expressions into this tree, in which every node has a [`Type`] and every
//! core-library call is a [`CoreOp`] on a receiver of known type.

use spec_lang_ir::{BinOp, CmpOp, ConstId, FnId, RuleId, StateId, TypeId};
use spec_lang_syntax::Span;

/// The type of a value, as far as a printer needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Unit,
    Bool,
    /// Exact and unbounded.
    Int,
    /// An `Int` known to fit a machine word: a size, a byte. A backend may hold
    /// it in its native number.
    Size,
    Text,
    /// A declared alias, record or sum.
    Named(TypeId),
    Option(Box<Type>),
    List(Box<Type>),
    Set(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
    /// Every state of the app, in a test or a law.
    World,
    Trace(Vec<RuleId>),
    Step(Vec<RuleId>),
    Event(Vec<RuleId>),
    Reply,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TExpr {
    pub kind: TKind,
    pub ty: Type,
    pub span: Span,
}

/// A core-library operation on a receiver whose type is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreOp {
    /// `text.size`: code points.
    TextSize,
    /// `text.trim()`.
    TextTrim,
    /// `text.bytes`: UTF-8.
    TextBytes,
    /// `text.contains(part)`.
    TextContains,
    /// `text.starts_with(prefix)`.
    TextStartsWith,
    /// `xs.size` of a list, set or map.
    Size,
    /// `set.list`: the set's elements, ascending.
    ToList,
    /// `map.keys`, ascending.
    Keys,
    /// `map.values`, in ascending key order.
    Values,
    /// `map.get(k)`: an `Option`.
    Get,
    Filter,
    Map,
    All,
    Any,
    /// `xs.sort(key)`: stable, ascending by key.
    Sort,
    /// `texts.join(sep)`.
    Join,
    /// `x in xs`: the receiver is the collection, the argument the element or key.
    In,
}

/// What a trace, a step, a world or an event gives by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceProp {
    States,
    Steps,
    Before,
    After,
    Event,
    Reply,
    /// A state of a world.
    State(StateId),
    /// A parameter of an event; every rule of the trace has it.
    Arg(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum TPart {
    Lit(String),
    Expr(TExpr),
}

#[derive(Clone, Debug, PartialEq)]
pub enum TPat {
    Wild,
    /// A variant with no data.
    Variant {
        ty: TypeId,
        tag: u32,
    },
    /// `none`.
    Nothing,
    /// `Some(name)` or `Some(_)`.
    Just(Option<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum TKind {
    Unit,
    Bool(bool),
    Int(i128),
    Text(String),
    Interp(Vec<TPart>),
    Local(String),
    Const(ConstId),
    /// The current value of a state.
    State(StateId),
    Variant {
        ty: TypeId,
        tag: u32,
    },
    /// `none`.
    Nothing,
    /// `Some(x)`.
    Just(Box<TExpr>),
    /// Arguments in parameter order; `None` where the default applies.
    CallFn {
        func: FnId,
        args: Vec<Option<TExpr>>,
    },
    /// In a test: one direct call of a rule.
    CallRule {
        rule: RuleId,
        args: Vec<Option<TExpr>>,
    },
    Record {
        ty: TypeId,
        args: Vec<Option<TExpr>>,
    },
    Field {
        recv: Box<TExpr>,
        name: String,
    },
    Core {
        op: CoreOp,
        recv: Box<TExpr>,
        args: Vec<TExpr>,
    },
    /// `map[key]`: fails `Missing` when absent.
    Index {
        recv: Box<TExpr>,
        key: Box<TExpr>,
    },
    With {
        recv: Box<TExpr>,
        fields: Vec<(String, TExpr)>,
    },
    Not(Box<TExpr>),
    Neg(Box<TExpr>),
    Binary(BinOp, Box<TExpr>, Box<TExpr>),
    /// One link of a comparison; a chain is an `And` of these.
    Compare(CmpOp, Box<TExpr>, Box<TExpr>),
    Tuple(Vec<TExpr>),
    List(Vec<TExpr>),
    SetOf(Vec<TExpr>),
    MapOf(Vec<(TExpr, TExpr)>),
    Lambda {
        params: Vec<(String, Type)>,
        body: Box<TExpr>,
    },
    If {
        cond: Box<TExpr>,
        then: Box<TExpr>,
        els: Box<TExpr>,
    },
    Match {
        scrutinee: Box<TExpr>,
        arms: Vec<(TPat, TExpr)>,
    },
    /// A base value entering a claimed type: the claim is checked here.
    Claim(Box<TExpr>),
    /// A `Size` used as an `Int`.
    Widen(Box<TExpr>),
    /// `out.text` in a test.
    OutText,
    Trace {
        recv: Box<TExpr>,
        prop: TraceProp,
    },
    /// `event.is(rule())`.
    IsRule {
        event: Box<TExpr>,
        rule: RuleId,
    },
    /// `reply.is(fail(_))` or `reply.is(fail(Name))`.
    IsRefused {
        reply: Box<TExpr>,
        error: Option<String>,
    },
    /// A block as a body: its value is its `return`.
    Block(Vec<TStmt>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TStmt {
    pub kind: TStmtKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TStmtKind {
    Let {
        name: String,
        mutable: bool,
        value: TExpr,
    },
    /// `state = value`.
    SetState {
        state: StateId,
        value: TExpr,
    },
    /// `state[key] = value`.
    PutEntry {
        state: StateId,
        key: TExpr,
        value: TExpr,
    },
    /// `state[key].field = value`.
    SetEntryField {
        state: StateId,
        key: TExpr,
        field: String,
        value: TExpr,
    },
    /// `state.remove(key)`.
    RemoveEntry {
        state: StateId,
        key: TExpr,
    },
    SetLocal {
        name: String,
        value: TExpr,
    },
    /// `out.line(text)`.
    OutLine(TExpr),
    /// `fail Name`.
    Fail(String),
    Expr(TExpr),
    Return(Option<TExpr>),
    Assert(TExpr),
    For {
        var: String,
        iter: TExpr,
        body: Vec<TStmt>,
    },
    While {
        cond: TExpr,
        body: Vec<TStmt>,
    },
    If {
        cond: TExpr,
        then: Vec<TStmt>,
        els: Option<Vec<TStmt>>,
    },
    /// `call -> value;`
    ExpectValue {
        call: TExpr,
        value: TExpr,
    },
    /// `call -> fail Name;`
    ExpectFail {
        call: TExpr,
        error: String,
    },
}

impl TExpr {
    pub fn new(kind: TKind, ty: Type, span: Span) -> Self {
        TExpr { kind, ty, span }
    }

    /// A literal or a local: evaluating it twice costs nothing.
    pub fn is_trivial(&self) -> bool {
        matches!(
            self.kind,
            TKind::Local(_) | TKind::Int(_) | TKind::Text(_) | TKind::Bool(_) | TKind::Variant { .. } | TKind::Nothing
        )
    }
}

/// Visit every expression of a statement list, outermost first.
pub fn walk_stmts(stmts: &[TStmt], f: &mut dyn FnMut(&TExpr)) {
    for s in stmts {
        match &s.kind {
            TStmtKind::Let { value, .. }
            | TStmtKind::SetState { value, .. }
            | TStmtKind::SetLocal { name: _, value }
            | TStmtKind::OutLine(value)
            | TStmtKind::Expr(value)
            | TStmtKind::Assert(value) => walk_expr(value, f),
            TStmtKind::PutEntry { key, value, .. } | TStmtKind::SetEntryField { key, value, .. } => {
                walk_expr(key, f);
                walk_expr(value, f);
            }
            TStmtKind::RemoveEntry { key, .. } => walk_expr(key, f),
            TStmtKind::Fail(_) => {}
            TStmtKind::Return(v) => {
                if let Some(v) = v {
                    walk_expr(v, f);
                }
            }
            TStmtKind::For { iter, body, .. } => {
                walk_expr(iter, f);
                walk_stmts(body, f);
            }
            TStmtKind::While { cond, body } => {
                walk_expr(cond, f);
                walk_stmts(body, f);
            }
            TStmtKind::If { cond, then, els } => {
                walk_expr(cond, f);
                walk_stmts(then, f);
                if let Some(els) = els {
                    walk_stmts(els, f);
                }
            }
            TStmtKind::ExpectValue { call, value } => {
                walk_expr(call, f);
                walk_expr(value, f);
            }
            TStmtKind::ExpectFail { call, .. } => walk_expr(call, f),
        }
    }
}

/// Visit an expression and everything inside it, outermost first.
pub fn walk_expr(e: &TExpr, f: &mut dyn FnMut(&TExpr)) {
    f(e);
    match &e.kind {
        TKind::Unit
        | TKind::Bool(_)
        | TKind::Int(_)
        | TKind::Text(_)
        | TKind::Local(_)
        | TKind::Const(_)
        | TKind::State(_)
        | TKind::Variant { .. }
        | TKind::Nothing
        | TKind::OutText => {}
        TKind::Interp(parts) => {
            for p in parts {
                if let TPart::Expr(x) = p {
                    walk_expr(x, f);
                }
            }
        }
        TKind::Just(x) | TKind::Not(x) | TKind::Neg(x) | TKind::Claim(x) | TKind::Widen(x) => walk_expr(x, f),
        TKind::CallFn { args, .. } | TKind::CallRule { args, .. } | TKind::Record { args, .. } => {
            for a in args.iter().flatten() {
                walk_expr(a, f);
            }
        }
        TKind::Field { recv, .. } | TKind::Trace { recv, .. } => walk_expr(recv, f),
        TKind::Core { recv, args, .. } => {
            walk_expr(recv, f);
            for a in args {
                walk_expr(a, f);
            }
        }
        TKind::Index { recv, key } => {
            walk_expr(recv, f);
            walk_expr(key, f);
        }
        TKind::With { recv, fields } => {
            walk_expr(recv, f);
            for (_, v) in fields {
                walk_expr(v, f);
            }
        }
        TKind::Binary(_, a, b) | TKind::Compare(_, a, b) => {
            walk_expr(a, f);
            walk_expr(b, f);
        }
        TKind::Tuple(xs) | TKind::List(xs) | TKind::SetOf(xs) => {
            for x in xs {
                walk_expr(x, f);
            }
        }
        TKind::MapOf(pairs) => {
            for (k, v) in pairs {
                walk_expr(k, f);
                walk_expr(v, f);
            }
        }
        TKind::Lambda { body, .. } => walk_expr(body, f),
        TKind::If { cond, then, els } => {
            walk_expr(cond, f);
            walk_expr(then, f);
            walk_expr(els, f);
        }
        TKind::Match { scrutinee, arms } => {
            walk_expr(scrutinee, f);
            for (_, v) in arms {
                walk_expr(v, f);
            }
        }
        TKind::IsRule { event, .. } => walk_expr(event, f),
        TKind::IsRefused { reply, .. } => walk_expr(reply, f),
        TKind::Block(stmts) => walk_stmts(stmts, f),
    }
}
