//! Expressions, statements, places and patterns of the LIR. Every
//! expression carries its [`LTy`]; every name is resolved; every library
//! operation is a [`Prim`]. `README.md` gives each node's evaluation order.

use crate::prim::Prim;
use crate::ty::{DTy, LTy};
use spec_lang_ir::{ConstId, FnId, HandlerId, RuleId, StateId, TypeId};
use spec_lang_syntax::Span;

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: LTy,
    pub span: Span,
}

/// `&&`, `||`, `implies`: left to right, stop when decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Logic {
    And,
    Or,
    Implies,
}

impl Logic {
    pub fn name(self) -> &'static str {
        match self {
            Logic::And => "and",
            Logic::Or => "or",
            Logic::Implies => "implies",
        }
    }
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Unit,
    Bool(bool),
    Int(i128),
    Text(String),
    /// `"a {x} b"`: literal parts, and values shown with `show` (`README.md`).
    Interp(Vec<Part>),
    Local(String),
    /// A constant: evaluated once, on first use, in no scope.
    Const(ConstId),
    /// The state's current value.
    State(StateId),
    /// A sum variant without fields, as a value.
    Variant {
        ty: TypeId,
        tag: u32,
    },
    /// `Some(x)`.
    Some(Box<Expr>),
    /// `none`.
    None,
    /// A checked constructor of a record (`tag` none) or a sum variant. `None`
    /// arguments take the field's default. Order: the given arguments left to
    /// right, then per field in order: its default if not given (evaluated in
    /// no scope), then `conform` to the field's declared type.
    Construct {
        ty: TypeId,
        tag: Option<u32>,
        args: Vec<Option<Expr>>,
    },
    /// A `fn` call: arguments left to right; then the callee's prologue.
    CallFn {
        func: FnId,
        args: Vec<Option<Expr>>,
    },
    /// A rule call. From a test (outside any rule) it is one atomic step;
    /// inside a rule it joins the caller's step.
    CallRule {
        rule: RuleId,
        args: Vec<Option<Expr>>,
    },
    /// A call of a library handler's operation (`out.line(text)`): the
    /// runtime of the library performs it. In tests, the handler's pretend
    /// ([`crate::Handler`]) runs it; its state commits and rolls back with
    /// the world.
    Effect {
        handler: HandlerId,
        op: u32,
        name: String,
        args: Vec<Expr>,
    },
    /// An operation read as a value, without a call: the test handler `out.text`.
    EffectRead {
        handler: HandlerId,
        op: u32,
        name: String,
    },
    Prim {
        op: Prim,
        args: Vec<Expr>,
    },
    Lambda(Box<Lambda>),
    /// Call a function value.
    CallLambda {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    /// A record's field by position.
    Field {
        recv: Box<Expr>,
        ty: TypeId,
        index: u32,
        name: String,
    },
    /// A field of a sum value by name: its position in each variant, `None`
    /// where the variant has no such field (stops E-TYPE-MISMATCH there).
    VariantField {
        recv: Box<Expr>,
        ty: TypeId,
        name: String,
        by_tag: Vec<Option<u32>>,
    },
    /// `w.tasks` on a world.
    WorldState {
        recv: Box<Expr>,
        state: StateId,
    },
    /// `s.event.no`: an argument of a traced call by name; its position per
    /// rule, `None` where the rule has no such parameter (stops E-TYPE-MISMATCH).
    EventArg {
        recv: Box<Expr>,
        name: String,
        by_rule: Vec<(RuleId, Option<u32>)>,
    },
    /// `r.with(f: v)`: a copy with fields changed, each checked by `conform`.
    With {
        recv: Box<Expr>,
        ty: TypeId,
        changes: Vec<(u32, Expr)>,
    },
    /// `x.is(pattern)`: bindings do not escape.
    Is {
        value: Box<Expr>,
        pat: Pat,
    },
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        els: Box<Expr>,
    },
    Logic {
        op: Logic,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// `a < b <= c`: each link evaluates its right side only when every
    /// earlier link held; false at the first that does not. A link is
    /// `op(left, right)`, except `x in c`, which is `op(c, x)`
    /// ([`Prim::is_containment`]).
    Compare {
        first: Box<Expr>,
        links: Vec<(Prim, Expr)>,
    },
    /// `value else fallback`: a failure of `value` gives `fallback`. With
    /// `unwrap` (the value is an `Option` and the source allows it), `none`
    /// gives `fallback` too and `Some(v)` gives `v`.
    Else {
        value: Box<Expr>,
        fallback: Box<Expr>,
        unwrap: bool,
    },
    /// `fail Name(data)`: arguments left to right, then the failure.
    Fail {
        name: String,
        args: Vec<Expr>,
    },
    /// Arms in order; the first whose pattern matches. No arm: stops E-RUNTIME.
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<Arm>,
    },
    /// A block as a value: its `return`, or `()` when it ends without one.
    Block(Block),
    /// `?name`: reaching it blocks the check (`F-BLOCKED`).
    Hole(String),
    Tuple(Vec<Expr>),
    List(Vec<Expr>),
    SetOf(Vec<Expr>),
    /// Entries left to right; a later equal key replaces an earlier one.
    MapOf(Vec<(Expr, Expr)>),
    Range {
        lo: Box<Expr>,
        hi: Box<Expr>,
        inclusive: bool,
    },
}

#[derive(Clone, Debug)]
pub enum Part {
    Lit(String),
    /// Text as it is; any other value as `show` prints it.
    Show(Expr),
}

/// A lambda. Captured locals are copied when the lambda is made.
#[derive(Clone, Debug)]
pub struct Lambda {
    pub params: Vec<(String, LTy)>,
    pub captures: Vec<(String, LTy)>,
    pub result: LTy,
    pub body: LambdaBody,
}

#[derive(Clone, Debug)]
pub enum LambdaBody {
    Expr(Expr),
    Block(Block),
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pat: Pat,
    pub value: Expr,
}

/// Statements; `ty` is what a `return` inside gives (the block's value type).
#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub ty: LTy,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    /// `let`/`var`; with `check`, the value must `conform` to it.
    Let {
        name: String,
        mutable: bool,
        ty: LTy,
        check: Option<DTy>,
        value: Expr,
    },
    /// Evaluate the value, then the place's keys; `conform` to `place.check`;
    /// write. A map key that is the last step adds or replaces; any other
    /// missing key or index fails Missing.
    Assign {
        place: Place,
        value: Expr,
    },
    /// `place.push(x)` and the like: evaluate the arguments, read the place,
    /// apply `op` (whose first argument is the place's value), then write as
    /// [`StmtKind::Assign`] does, `conform` included.
    Mutate {
        place: Place,
        op: Prim,
        args: Vec<Expr>,
    },
    Expr(Expr),
    Return(Option<Expr>),
    /// In a test: false fails the check (`F-TEST`).
    Assert(Expr),
    /// `iter` is a `List`, evaluated once before the loop.
    For {
        var: String,
        ty: LTy,
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
    /// A test step `subject -> value;` or `subject -> fail E;`.
    Expect {
        subject: Expr,
        expect: Expected,
    },
}

/// What a test step or a `@cases` row expects.
#[derive(Clone, Debug)]
pub enum Expected {
    Value(Expr),
    /// A refusal by name; with arguments, its data must be equal too (test
    /// steps; a `@cases` row compares the name only).
    Fail {
        name: String,
        args: Vec<Expr>,
    },
}

/// A place a statement writes.
#[derive(Clone, Debug)]
pub struct Place {
    pub root: PlaceRoot,
    pub path: Vec<PlaceStep>,
    /// The type of the value at the end of the path.
    pub ty: LTy,
    /// For a state place: the declared type the written value must meet
    /// (`None` when the path ends in no declared type, as the engine does).
    pub check: Option<DTy>,
}

#[derive(Clone, Debug)]
pub enum PlaceRoot {
    State(StateId),
    Local(String),
}

#[derive(Clone, Debug)]
pub enum PlaceStep {
    Field { ty: TypeId, index: u32 },
    MapKey(Expr),
    ListIndex(Expr),
}

/// A pattern compiled to steps run in order against the scrutinee: the first
/// failing test means no match; bindings are made as they come.
#[derive(Clone, Debug, Default)]
pub struct Pat {
    pub steps: Vec<PatStep>,
}

#[derive(Clone, Debug)]
pub enum PatStep {
    /// The value at `path` is this variant of `ty` (for a reply: `Ok` is
    /// tag 0, `Err` tag 1 of the built-in `Result`).
    Tag {
        path: Vec<PathStep>,
        ty: TypeId,
        tag: u32,
    },
    /// The event at `path` is a call of `rule`.
    Event {
        path: Vec<PathStep>,
        rule: RuleId,
    },
    /// The value at `path` equals `value` (evaluated now).
    Equals {
        path: Vec<PathStep>,
        value: Expr,
    },
    /// Never matches (a field the variant does not have).
    Never,
    Bind {
        name: String,
        path: Vec<PathStep>,
        ty: LTy,
    },
}

/// One step from the scrutinee into its parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathStep {
    /// Field `index` of variant `tag` of `ty` (`Some`'s value, a reply's payload).
    VariantField {
        ty: TypeId,
        tag: u32,
        index: u32,
    },
    TupleItem(u32),
    /// Argument `index` of a call of `rule`.
    EventArg {
        rule: RuleId,
        index: u32,
    },
}

impl Expr {
    pub fn new(kind: ExprKind, ty: LTy, span: Span) -> Self {
        Expr { kind, ty, span }
    }
}
