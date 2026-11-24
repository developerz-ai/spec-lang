//! Resolved expressions, statements and patterns.
//!
//! Names are resolved: a local is [`ExprKind::Local`], a state is
//! [`ExprKind::State`], a call to a `fn`, `rule`, record or variant is
//! [`ExprKind::Call`] with its arguments already in parameter order.
//! What is still dynamic, because the IR has no expression types, is member
//! access on a value: [`ExprKind::Prop`] (`x.size`, `t.status`) and
//! [`ExprKind::Method`] (`xs.filter(...)`) are looked up on the value.

use crate::ids::{LambdaId, ModuleId, RuleId, StateId, TypeId};
use crate::ty::Ty;
use spec_lang_syntax::Span;
pub use spec_lang_syntax::ast::{CmpOp, UnOp};

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Expr { kind, span }
    }
}

/// Binary operators that evaluate both sides, plus the three that short-circuit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    /// `/`: on `Int` an open choice (`E-OPEN`), see `docs/idea/28-standard-library.md`.
    Div,
    Mod,
    Pow,
    Band,
    Bor,
    Bxor,
    Shl,
    Shr,
    /// Left to right, stops when decided.
    And,
    Or,
    Implies,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Unit,
    Bool(bool),
    Int(i128),
    /// A text with escapes processed and no interpolation.
    Text(String),
    /// `"a {x} b"`.
    Interp(Vec<InterpPart>),
    /// A parameter, `let`/`var`, `for` variable, pattern binding, lambda
    /// parameter, `it`, or `self` in a claim.
    Local(String),
    Const(crate::ids::ConstId),
    /// Reads the state's current value.
    State(StateId),
    /// A variant with no fields used as a value (`Open`, `none`).
    Variant {
        ty: TypeId,
        tag: u32,
    },
    /// A call whose callee is known: arguments are in parameter order, `None`
    /// where the parameter's default applies.
    Call {
        callee: Callee,
        args: Vec<Option<Expr>>,
    },
    /// A call of a value (a lambda held in a local).
    CallValue {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    /// `recv.name(args)`: a core library method, looked up on the value.
    Method {
        recv: Box<Expr>,
        name: String,
        args: Vec<Arg>,
    },
    /// `recv.name`: a record field, a core property (`size`, `keys`, `values`,
    /// `bytes`, `steps`, ...), a state of a world, or an argument of an event.
    Prop {
        recv: Box<Expr>,
        name: String,
    },
    /// `recv[key]`: fails `Missing` when absent.
    Index {
        recv: Box<Expr>,
        key: Box<Expr>,
    },
    /// `recv.with(f: v, ...)`.
    With {
        recv: Box<Expr>,
        fields: Vec<(String, Expr)>,
    },
    /// `value.is(pattern)`.
    Is {
        value: Box<Expr>,
        pattern: Box<Pattern>,
    },
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// `a < b <= c`: every link holds; stops at the first that does not.
    Compare(Box<Expr>, Vec<(CmpOp, Expr)>),
    Range {
        lo: Box<Expr>,
        hi: Box<Expr>,
        inclusive: bool,
    },
    Tuple(Vec<Expr>),
    List(Vec<Expr>),
    /// `Set(a, b)`.
    SetOf(Vec<Expr>),
    /// `Map(k -> v, ...)`.
    MapOf(Vec<(Expr, Expr)>),
    Lambda(LambdaId),
    /// `if c then a else b`.
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        els: Box<Expr>,
    },
    /// `value else fallback`: a failure of `value` gives `fallback`; when
    /// `unwrap_option`, a `none` does too and `Some(v)` gives `v`.
    Else {
        value: Box<Expr>,
        fallback: Box<Expr>,
        unwrap_option: bool,
    },
    Fail(Failure),
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<Arm>,
    },
    /// A block used as a value: its value is its `return`.
    Block(Block),
    /// The `out.text` test handler: every line `out.line` sent, each with `\n`.
    OutText,
    /// `?name`: runs until reached, then the test is blocked.
    Hole(String),
}

/// A part of an interpolated text.
#[derive(Clone, Debug, PartialEq)]
pub enum InterpPart {
    Lit(String),
    Expr(Expr),
}

/// The target of a [`ExprKind::Call`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Callee {
    Fn(crate::ids::FnId),
    Rule(RuleId),
    /// A record type's constructor.
    Record(TypeId),
    /// A sum variant's constructor (`Some(x)`, `Held(by, until)`).
    Variant(TypeId, u32),
    /// An effect of a vocabulary.
    Effect(Effect),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// `out.line(text)` of the `cli` vocabulary: one line, sent after commit.
    OutLine,
}

/// An argument of a dynamic call.
#[derive(Clone, Debug, PartialEq)]
pub struct Arg {
    pub name: Option<String>,
    pub value: Expr,
}

/// `fail Name` or `fail Name(data...)`. An error name declares itself.
#[derive(Clone, Debug, PartialEq)]
pub struct Failure {
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    Let {
        name: String,
        mutable: bool,
        ty: Option<Ty>,
        value: Expr,
    },
    /// `place = value`.
    Assign {
        place: Place,
        value: Expr,
    },
    /// `place.push(x)`, `place.insert(...)`, `place.remove(k)`: a statement
    /// that changes a place and returns nothing.
    Mutate {
        place: Place,
        op: MutOp,
        args: Vec<Expr>,
    },
    Expr(Expr),
    Return(Option<Expr>),
    Assert(Expr),
    For {
        var: String,
        iter: Expr,
        body: Block,
    },
    While {
        cond: Expr,
        body: Block,
    },
    If {
        cond: Expr,
        then: Block,
        els: Option<Block>,
    },
    /// In a test: `subject -> value;` or `subject -> fail E;`.
    Expect {
        subject: Expr,
        expect: Expect,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutOp {
    Push,
    Insert,
    Remove,
}

/// What a test step or a case expects.
#[derive(Clone, Debug, PartialEq)]
pub enum Expect {
    Value(Expr),
    Fail(Failure),
}

/// A place a statement writes: a state or a `var`, then fields and indexes.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub root: PlaceRoot,
    pub path: Vec<PlaceStep>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlaceRoot {
    State(StateId),
    Local(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlaceStep {
    Field(String),
    Index(Expr),
}

/// `x => body`, `(a, b) => body`, or an argument that mentions `it`.
#[derive(Clone, Debug, PartialEq)]
pub struct Lambda {
    pub params: Vec<String>,
    pub body: LambdaBody,
    pub module: ModuleId,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LambdaBody {
    Expr(Expr),
    Block(Block),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arm {
    pub pattern: Pattern,
    pub value: Expr,
    pub span: Span,
}

/// Patterns of `match` and `.is(...)`.
#[derive(Clone, Debug, PartialEq)]
pub enum Pattern {
    /// `_`.
    Wild,
    /// A name not in scope: binds it.
    Bind(String),
    /// A literal, or a name in scope: compares equal.
    Value(Expr),
    /// `Held(by, _)`, `Some(x)`, `Open`, and a trace reply's `Ok(v)`, `Err(_)`.
    Variant {
        ty: TypeId,
        tag: u32,
        args: Vec<PatArg>,
    },
    Tuple(Vec<Pattern>),
    /// An event in a trace: `board.clear()`. Arguments not written match anything.
    Call {
        rule: RuleId,
        args: Vec<PatArg>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatArg {
    pub name: Option<String>,
    pub pattern: Pattern,
}
