//! A small typed Ruby tree: what the Ruby backend builds and prints.
//! It covers the Ruby this backend emits, not the language.

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Nil,
    True,
    False,
    SelfRef,
    Int(i128),
    /// A double-quoted string; the printer escapes it.
    Str(String),
    Sym(String),
    /// `"a #{x} b"`.
    Interp(Vec<Part>),
    /// A local or a bare method name.
    Ident(String),
    /// `A::B::C`.
    Const(Vec<String>),
    /// `recv.name(args) { block }`; without a receiver, `name(args)`.
    Call {
        recv: Option<Box<Expr>>,
        name: String,
        args: Vec<Arg>,
        block: Option<Block>,
    },
    /// `recv[key]`.
    Index(Box<Expr>, Box<Expr>),
    Array(Vec<Expr>),
    /// `{ k => v }`; a symbol key prints as `k: v`.
    Hash(Vec<(Expr, Expr)>),
    /// `Set[a, b]`.
    SetLit(Vec<Expr>),
    Not(Box<Expr>),
    Neg(Box<Expr>),
    Bin(&'static str, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    /// `->(a, b) { body }`.
    Lambda(Vec<Param>, Box<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Lit(String),
    Expr(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Pos(Expr),
    /// `name: value`, written `name:` when the value is the local of that name.
    Key(String, Expr),
    /// `**hash`.
    Splat2(Expr),
}

/// `{ |a, b| body }` for one expression, `do |a, b| ... end` otherwise.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    Req(String),
    /// `name:`.
    Key(String),
    /// `name: default`.
    KeyOpt(String, Expr),
    /// `**name`.
    Rest2(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    Assign(String, Expr),
    /// `NAME = value`.
    ConstAssign(String, Expr),
    Return(Option<Expr>),
    /// `raise Class`.
    Raise(Expr),
    If {
        cond: Expr,
        then: Vec<Stmt>,
        els: Option<Vec<Stmt>>,
    },
    /// `stmt unless cond` / `stmt if cond`.
    Modifier {
        stmt: Box<Stmt>,
        keyword: &'static str,
        cond: Expr,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    /// `def name(params)`; `def self.name` when `singleton`.
    Def {
        singleton: bool,
        name: String,
        params: Vec<Param>,
        body: Vec<Stmt>,
    },
    Module {
        name: String,
        body: Vec<Stmt>,
    },
    Class {
        name: String,
        superclass: Option<Expr>,
        body: Vec<Stmt>,
    },
    /// `Name = Data.define(:a, :b) do ... end`; no block when `body` is empty.
    DataDefine {
        name: String,
        fields: Vec<String>,
        body: Vec<Stmt>,
    },
    /// `super(args)`.
    Super(Vec<Arg>),
    /// `case on; when a then x; ...; end`.
    Case {
        on: Expr,
        whens: Vec<(Expr, Expr)>,
    },
    Comment(String),
    Blank,
}

pub fn ident(n: &str) -> Expr {
    Expr::Ident(n.to_string())
}

pub fn konst(path: &str) -> Expr {
    Expr::Const(path.split("::").map(str::to_string).collect())
}

pub fn sym(n: &str) -> Expr {
    Expr::Sym(n.to_string())
}

pub fn text(s: &str) -> Expr {
    Expr::Str(s.to_string())
}

/// `recv.name(args)`.
pub fn call(recv: Expr, name: &str, args: Vec<Expr>) -> Expr {
    Expr::Call {
        recv: Some(Box::new(recv)),
        name: name.to_string(),
        args: args.into_iter().map(Arg::Pos).collect(),
        block: None,
    }
}

/// `recv.name(args)` with keyword arguments allowed.
pub fn call_args(recv: Expr, name: &str, args: Vec<Arg>) -> Expr {
    Expr::Call { recv: Some(Box::new(recv)), name: name.to_string(), args, block: None }
}

/// `recv.name(args) { |params| body }`.
pub fn call_block(recv: Expr, name: &str, args: Vec<Expr>, params: &[&str], body: Vec<Stmt>) -> Expr {
    Expr::Call {
        recv: Some(Box::new(recv)),
        name: name.to_string(),
        args: args.into_iter().map(Arg::Pos).collect(),
        block: Some(Block { params: params.iter().map(|p| p.to_string()).collect(), body }),
    }
}

pub fn bin(op: &'static str, a: Expr, b: Expr) -> Expr {
    Expr::Bin(op, Box::new(a), Box::new(b))
}

/// `!e`, folding a comparison, a double negation, or a predicate with an opposite.
pub fn not(e: Expr) -> Expr {
    match e {
        Expr::Not(inner) => *inner,
        Expr::Bin("==", a, b) => Expr::Bin("!=", a, b),
        Expr::Bin("!=", a, b) => Expr::Bin("==", a, b),
        other => Expr::Not(Box::new(other)),
    }
}

pub fn key(name: &str, value: Expr) -> Arg {
    Arg::Key(name.to_string(), value)
}
