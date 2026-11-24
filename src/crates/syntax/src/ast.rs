//! The typed syntax tree. One node per grammar rule that carries meaning;
//! precedence levels collapse into [`ExprKind`]. Every node has a [`Span`].

use crate::span::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// `NAME ("." NAME)*`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    pub segments: Vec<Ident>,
    pub span: Span,
}

impl Path {
    /// The dotted text: `todo.model.Task`.
    pub fn dotted(&self) -> String {
        self.segments.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(".")
    }
}

/// `file: use* decl*`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub uses: Vec<Use>,
    pub decls: Vec<Decl>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Use {
    pub path: Path,
    pub span: Span,
}

/// `note* "pub"? path NAME generics? params? result? tail`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decl {
    pub notes: Vec<Note>,
    pub is_pub: bool,
    pub kind: Path,
    pub name: Ident,
    pub generics: Option<Vec<Param>>,
    pub params: Option<Vec<Param>>,
    pub result: Option<ResultType>,
    pub tail: Tail,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultSep {
    /// `: T`, as in `state x: T`.
    Colon,
    /// `-> T`, as in `fn f() -> T`.
    Arrow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultType {
    pub sep: ResultSep,
    pub ty: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tail {
    /// `;`: declared, no body.
    Semi,
    /// `{ decl* }`.
    Children(Vec<Decl>),
    /// `= body`.
    Body(Body),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    Value(Value),
    Machine(Block),
    Foreign { lang: Ident, symbol: TextLit },
}

/// `value: expr ";" | blocky`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Expr(Expr),
    Block(Block),
    Match(Match),
}

impl Value {
    pub fn span(&self) -> Span {
        match self {
            Value::Expr(e) => e.span,
            Value::Block(b) => b.span,
            Value::Match(m) => m.span,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub scrutinee: Expr,
    pub arms: Vec<Arm>,
    pub span: Span,
}

/// `arm: bar "=>" value`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arm {
    pub pattern: Expr,
    pub value: Value,
    pub span: Span,
}

/// `note: "@" path ("(" args? ")")?`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub path: Path,
    pub args: Option<Vec<Arg>>,
    pub span: Span,
}

/// `param: note* NAME (":" expr)? ("=" expr)?`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub notes: Vec<Note>,
    pub name: Ident,
    pub ty: Option<Expr>,
    pub default: Option<Expr>,
    pub span: Span,
}

/// `arg: (NAME ":")? expr`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arg {
    pub name: Option<Ident>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StmtKind {
    Let { mutable: bool, name: Ident, ty: Option<Expr>, value: Value },
    Return(Option<Value>),
    Assert(Expr),
    Expr(Expr),
    Assign { place: Expr, value: Value },
    For { var: Ident, iter: Expr, body: Block },
    While { cond: Expr, body: Block },
    If(IfStmt),
    Match(Match),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IfStmt {
    pub cond: Expr,
    pub then: Block,
    pub els: Option<Else>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Else {
    If(Box<IfStmt>),
    Block(Block),
}

/// A text literal as written: `prefix"raw"`. Escapes and `{interpolation}`
/// are not processed here; `raw_offset` is the byte offset of `raw` in the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextLit {
    pub prefix: Option<String>,
    pub raw: String,
    pub raw_offset: u32,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Pos,
    Not,
    Own,
    Ref,
    Mut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Band,
    Bor,
    Bxor,
    Shl,
    Shr,
    /// `|`: sums, unions of rule names, patterns.
    Bar,
    Where,
    Implies,
    And,
    Or,
    RangeIncl,
    RangeExcl,
    PlusMinus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    In,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LambdaBody {
    Expr(Box<Expr>),
    Block(Block),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExprKind {
    Name(String),
    /// A number with its unit suffix, as written.
    Num(String),
    Text(TextLit),
    /// `?name`.
    Hole(Ident),
    /// `( args? )`: grouping, a tuple, or a record of named values.
    Group {
        args: Vec<Arg>,
        trailing_comma: bool,
    },
    /// `[ args? ]`.
    List(Vec<Arg>),
    Field(Box<Expr>, Ident),
    Call(Box<Expr>, Vec<Arg>),
    Index(Box<Expr>, Vec<Arg>),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// `x²`, `x³`.
    Sup(Box<Expr>, u8),
    /// `a < b <= c`: a chain of comparisons.
    Compare(Box<Expr>, Vec<(CmpOp, Expr)>),
    /// `params => body`.
    Lambda(Box<Expr>, LambdaBody),
    /// `a -> b`.
    Arrow(Box<Expr>, Box<Expr>),
    /// `a else b`.
    Else(Box<Expr>, Box<Expr>),
    /// `if c then a else b`.
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
    /// `fail e`.
    Fail(Box<Expr>),
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Expr { kind, span }
    }

    /// The bare name, if this expression is one.
    pub fn as_name(&self) -> Option<&str> {
        match &self.kind {
            ExprKind::Name(n) => Some(n),
            _ => None,
        }
    }
}
