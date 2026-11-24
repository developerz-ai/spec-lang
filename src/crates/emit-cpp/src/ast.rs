//! A small typed C++ target tree and its printer. The printer writes plain,
//! unwrapped C++; `clang-format` sets the final layout (see [`crate::format`]).
//! Expressions carry precedence, so parentheses appear only where C++ needs them.

use std::fmt::Write;

/// A type as C++ spells it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ty {
    /// A (possibly qualified) name: `std::int64_t`, `model::Task`.
    Name(String),
    /// `name<args...>`.
    Generic(String, Vec<Ty>),
    Const(Box<Ty>),
    Ref(Box<Ty>),
    Ptr(Box<Ty>),
    Void,
    Auto,
}

impl Ty {
    pub fn name(s: impl Into<String>) -> Ty {
        Ty::Name(s.into())
    }
    pub fn generic(s: impl Into<String>, args: Vec<Ty>) -> Ty {
        Ty::Generic(s.into(), args)
    }
    /// `const T&`.
    pub fn cref(self) -> Ty {
        Ty::Ref(Box::new(Ty::Const(Box::new(self))))
    }
    /// `const T`.
    pub fn konst(self) -> Ty {
        Ty::Const(Box::new(self))
    }
    pub fn mref(self) -> Ty {
        Ty::Ref(Box::new(self))
    }
    pub fn cptr(self) -> Ty {
        Ty::Ptr(Box::new(Ty::Const(Box::new(self))))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    /// A (possibly qualified) name.
    Id(String),
    Int(i128),
    Bool(bool),
    /// A string literal; escaped by the printer.
    Str(String),
    Call(Box<Expr>, Vec<Expr>),
    /// `f<T...>(args)`.
    CallT(Box<Expr>, Vec<Ty>, Vec<Expr>),
    Member(Box<Expr>, String),
    Arrow(Box<Expr>, String),
    /// A prefix operator: `!`, `-`, `*`, `&`.
    Un(&'static str, Box<Expr>),
    Bin(&'static str, Box<Expr>, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
    /// `[&](params) -> ret { body }`.
    Lambda {
        capture: &'static str,
        params: Vec<(Ty, String)>,
        ret: Option<Ty>,
        body: Vec<Stmt>,
    },
    /// `T{.a = x, .b = y}`.
    Designated(Ty, Vec<(String, Expr)>),
    /// `T{a, b}`, or `{a, b}` without a type.
    Brace(Option<Ty>, Vec<Expr>),
    /// `static_cast<T>(e)`.
    Cast(Ty, Box<Expr>),
    /// `(e)`: a macro argument whose commas are not inside parentheses.
    Paren(Box<Expr>),
}

impl Expr {
    pub fn id(s: impl Into<String>) -> Expr {
        Expr::Id(s.into())
    }
    pub fn str(s: impl Into<String>) -> Expr {
        Expr::Str(s.into())
    }
    pub fn call(f: impl Into<String>, args: Vec<Expr>) -> Expr {
        Expr::Call(Box::new(Expr::Id(f.into())), args)
    }
    pub fn method(self, name: &str, args: Vec<Expr>) -> Expr {
        Expr::Call(Box::new(Expr::Member(Box::new(self), name.into())), args)
    }
    pub fn member(self, name: &str) -> Expr {
        Expr::Member(Box::new(self), name.into())
    }
    pub fn bin(op: &'static str, a: Expr, b: Expr) -> Expr {
        Expr::Bin(op, Box::new(a), Box::new(b))
    }
    pub fn negate(self) -> Expr {
        match self {
            Expr::Un("!", x) => *x,
            Expr::Bin(op @ ("==" | "!=" | "<" | "<=" | ">" | ">="), a, b) => {
                let flipped = match op {
                    "==" => "!=",
                    "!=" => "==",
                    "<" => ">=",
                    "<=" => ">",
                    ">" => "<=",
                    _ => "<",
                };
                Expr::Bin(flipped, a, b)
            }
            x => Expr::Un("!", Box::new(x)),
        }
    }
    pub fn deref(self) -> Expr {
        match self {
            Expr::Un("&", x) => *x,
            x => Expr::Un("*", Box::new(x)),
        }
    }
    pub fn moved(self) -> Expr {
        Expr::call("std::move", vec![self])
    }

    /// As a macro argument: parenthesised when a comma in it is not inside parentheses.
    pub fn macro_arg(self) -> Expr {
        fn bare_comma(e: &Expr) -> bool {
            match e {
                Expr::Designated(..) | Expr::Brace(..) | Expr::CallT(..) => true,
                Expr::Member(x, _) | Expr::Arrow(x, _) | Expr::Un(_, x) => bare_comma(x),
                Expr::Call(f, _) => bare_comma(f),
                Expr::Bin(_, a, b) => bare_comma(a) || bare_comma(b),
                Expr::Cond(c, a, b) => bare_comma(c) || bare_comma(a) || bare_comma(b),
                _ => false,
            }
        }
        if bare_comma(&self) { Expr::Paren(Box::new(self)) } else { self }
    }

    fn prec(&self) -> u8 {
        match self {
            Expr::Bin(op, ..) => bin_prec(op),
            Expr::Cond(..) => 1,
            Expr::Un(..) => 14,
            Expr::Lambda { .. } => 16,
            _ => 16,
        }
    }
}

fn bin_prec(op: &str) -> u8 {
    match op {
        "|" => 6,
        "||" => 3,
        "&&" => 4,
        "==" | "!=" => 8,
        "<" | "<=" | ">" | ">=" => 9,
        "+" | "-" => 11,
        "*" | "/" | "%" => 12,
        "=" => 1,
        _ => 2,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    Return(Option<Expr>),
    /// `T name = init;` or `T name;`.
    Var {
        ty: Ty,
        name: String,
        init: Option<Expr>,
    },
    If {
        cond: Expr,
        then: Vec<Stmt>,
        els: Vec<Stmt>,
    },
    /// `for (T name : range) { body }`.
    For {
        ty: Ty,
        name: String,
        range: Expr,
        body: Vec<Stmt>,
    },
    /// `for (init; cond; step) { body }` over an integer counter.
    Count {
        name: String,
        ty: Ty,
        from: Expr,
        below: Expr,
        body: Vec<Stmt>,
    },
    /// `switch (on) { case a: body ... }` with every case ending in its own `return`.
    Switch {
        on: Expr,
        cases: Vec<(Expr, Vec<Stmt>)>,
    },
    Block(Vec<Stmt>),
    /// `// text`.
    Comment(String),
    /// `static const T name = init;`
    Static {
        ty: Ty,
        name: String,
        init: Expr,
    },
    /// `if (auto name = init) { then }`.
    IfInit {
        name: String,
        init: Expr,
        then: Vec<Stmt>,
    },
    /// `if constexpr (requires { value.member; }) { then } else { els }`.
    IfHas {
        value: String,
        member: String,
        then: Vec<Stmt>,
        els: Vec<Stmt>,
    },
}

/// A function parameter.
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub ty: Ty,
    pub name: String,
    pub default: Option<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Func {
    pub doc: Vec<String>,
    /// `[[nodiscard]]`, `constexpr`, `inline`, `static`, ...
    pub prefix: Vec<&'static str>,
    pub ret: Ty,
    /// Qualified for an out-of-line definition (`Title::make`).
    pub name: String,
    pub params: Vec<Param>,
    /// `const`, `noexcept`, `= default`.
    pub suffix: Vec<&'static str>,
    /// `None`: a declaration.
    pub body: Option<Vec<Stmt>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Member {
    Field {
        ty: Ty,
        name: String,
        init: Option<Expr>,
    },
    Func(Func),
    /// `explicit Name(params) : inits {}`.
    Ctor {
        explicit: bool,
        constexpr: bool,
        name: String,
        params: Vec<Param>,
        inits: Vec<(String, Expr)>,
    },
    Access(&'static str),
    Comment(String),
    Using {
        name: String,
        ty: Ty,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Decl {
    PragmaOnce,
    /// `#include <x>` (system) or `#include "x"`.
    Include {
        path: String,
        system: bool,
    },
    Namespace(String, Vec<Decl>),
    Comment(String),
    Blank,
    Enum {
        doc: Vec<String>,
        name: String,
        underlying: Ty,
        variants: Vec<String>,
    },
    Struct {
        doc: Vec<String>,
        keyword: &'static str,
        name: String,
        members: Vec<Member>,
    },
    Func(Func),
    /// `inline constexpr T name = init;` and friends.
    Var {
        doc: Vec<String>,
        prefix: Vec<&'static str>,
        ty: Ty,
        name: String,
        init: Expr,
    },
    /// `using name = ty;`
    Using {
        name: String,
        ty: Ty,
    },
    /// A provenance marker comment: `// @@slg N`, removed after formatting.
    Marker(u32),
    /// A line of the preprocessor or another form this tree does not model, verbatim.
    Line(String),
}

// ---- printing ----

pub fn ty(t: &Ty) -> String {
    match t {
        Ty::Name(n) => n.clone(),
        Ty::Generic(n, args) => format!("{n}<{}>", args.iter().map(ty).collect::<Vec<_>>().join(", ")),
        Ty::Const(x) => format!("const {}", ty(x)),
        Ty::Ref(x) => format!("{}&", ty(x)),
        Ty::Ptr(x) => format!("{}*", ty(x)),
        Ty::Void => "void".into(),
        Ty::Auto => "auto".into(),
    }
}

pub fn str_lit(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                let _ = write!(out, "\\x{:02x}\"\"", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn expr(e: &Expr) -> String {
    match e {
        Expr::Id(n) => n.clone(),
        Expr::Int(n) => n.to_string(),
        Expr::Bool(b) => b.to_string(),
        Expr::Str(s) => str_lit(s),
        Expr::Call(f, args) => format!("{}({})", postfix(f), list(args)),
        Expr::CallT(f, ts, args) => {
            format!("{}<{}>({})", postfix(f), ts.iter().map(ty).collect::<Vec<_>>().join(", "), list(args))
        }
        Expr::Member(x, m) => format!("{}.{m}", postfix(x)),
        Expr::Arrow(x, m) => format!("{}->{m}", postfix(x)),
        Expr::Un(op, x) => {
            let inner = if x.prec() < 14 { format!("({})", expr(x)) } else { expr(x) };
            format!("{op}{inner}")
        }
        Expr::Bin(op, a, b) => {
            let p = bin_prec(op);
            let l = if a.prec() < p { format!("({})", expr(a)) } else { expr(a) };
            let r = if b.prec() <= p && !(*op == "=") { format!("({})", expr(b)) } else { expr(b) };
            // Mixed && and || always get parentheses, as -Wparentheses asks.
            let l = match (&**a, *op) {
                (Expr::Bin("&&", ..), "||") if !l.starts_with('(') => format!("({l})"),
                _ => l,
            };
            let r = match (&**b, *op) {
                (Expr::Bin("&&", ..), "||") if !r.starts_with('(') => format!("({r})"),
                _ => r,
            };
            format!("{l} {op} {r}")
        }
        Expr::Cond(c, a, b) => format!("{} ? {} : {}", wrap(c, 2), wrap(a, 2), wrap(b, 1)),
        Expr::Lambda { capture, params, ret, body } => {
            let ps = params.iter().map(|(t, n)| format!("{} {n}", ty(t))).collect::<Vec<_>>().join(", ");
            let r = ret.as_ref().map(|t| format!(" -> {}", ty(t))).unwrap_or_default();
            let mut s = format!("[{capture}]({ps}){r} {{\n");
            for st in body {
                stmt(st, &mut s);
            }
            s.push('}');
            s
        }
        Expr::Designated(t, fields) => format!(
            "{}{{{}}}",
            ty(t),
            fields.iter().map(|(n, v)| format!(".{n} = {}", expr(v))).collect::<Vec<_>>().join(", ")
        ),
        Expr::Brace(t, xs) => format!("{}{{{}}}", t.as_ref().map(ty).unwrap_or_default(), list(xs)),
        Expr::Cast(t, x) => format!("static_cast<{}>({})", ty(t), expr(x)),
        Expr::Paren(x) => format!("({})", expr(x)),
    }
}

fn wrap(e: &Expr, p: u8) -> String {
    if e.prec() < p { format!("({})", expr(e)) } else { expr(e) }
}

fn postfix(e: &Expr) -> String {
    wrap(e, 15)
}

fn list(xs: &[Expr]) -> String {
    xs.iter().map(expr).collect::<Vec<_>>().join(", ")
}

pub fn stmt(s: &Stmt, out: &mut String) {
    match s {
        Stmt::Expr(e) => {
            let _ = writeln!(out, "{};", expr(e));
        }
        Stmt::Return(None) => out.push_str("return;\n"),
        Stmt::Return(Some(e)) => {
            let _ = writeln!(out, "return {};", expr(e));
        }
        Stmt::Var { ty: t, name, init } => {
            let _ = match init {
                Some(i) => writeln!(out, "{} {name} = {};", ty(t), expr(i)),
                None => writeln!(out, "{} {name};", ty(t)),
            };
        }
        Stmt::If { cond, then, els } => {
            let _ = writeln!(out, "if ({}) {{", expr(cond));
            block(then, out);
            if els.is_empty() {
                out.push_str("}\n");
            } else if let [Stmt::If { .. }] = els.as_slice() {
                out.push_str("} else ");
                stmt(&els[0], out);
            } else {
                out.push_str("} else {\n");
                block(els, out);
                out.push_str("}\n");
            }
        }
        Stmt::For { ty: t, name, range, body } => {
            let _ = writeln!(out, "for ({} {name} : {}) {{", ty(t), expr(range));
            block(body, out);
            out.push_str("}\n");
        }
        Stmt::Count { name, ty: t, from, below, body } => {
            let _ = writeln!(out, "for ({} {name} = {}; {name} < {}; ++{name}) {{", ty(t), expr(from), expr(below));
            block(body, out);
            out.push_str("}\n");
        }
        Stmt::Switch { on, cases } => {
            let _ = writeln!(out, "switch ({}) {{", expr(on));
            for (c, body) in cases {
                let _ = writeln!(out, "case {}:", expr(c));
                block(body, out);
            }
            out.push_str("}\n");
        }
        Stmt::Block(b) => {
            out.push_str("{\n");
            block(b, out);
            out.push_str("}\n");
        }
        Stmt::Comment(c) => {
            let _ = writeln!(out, "// {c}");
        }
        Stmt::Static { ty: t, name, init } => {
            let _ = writeln!(out, "static const {} {name} = {};", ty(t), expr(init));
        }
        Stmt::IfInit { name, init, then } => {
            let _ = writeln!(out, "if (auto {name} = {}) {{", expr(init));
            block(then, out);
            out.push_str("}\n");
        }
        Stmt::IfHas { value, member, then, els } => {
            let _ = writeln!(out, "if constexpr (requires {{ {value}.{member}; }}) {{");
            block(then, out);
            out.push_str("} else {\n");
            block(els, out);
            out.push_str("}\n");
        }
    }
}

fn block(b: &[Stmt], out: &mut String) {
    for s in b {
        stmt(s, out);
    }
}

fn params(ps: &[Param]) -> String {
    ps.iter()
        .map(|p| match &p.default {
            Some(d) => format!("{} {} = {}", ty(&p.ty), p.name, expr(d)),
            None => format!("{} {}", ty(&p.ty), p.name),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn doc(lines: &[String], out: &mut String) {
    for l in lines {
        let _ = writeln!(out, "// {l}");
    }
}

pub fn func(f: &Func, out: &mut String) {
    doc(&f.doc, out);
    for p in &f.prefix {
        out.push_str(p);
        out.push(' ');
    }
    let _ = write!(out, "{} {}({})", ty(&f.ret), f.name, params(&f.params));
    for s in &f.suffix {
        out.push(' ');
        out.push_str(s);
    }
    match &f.body {
        None => out.push_str(";\n"),
        Some(b) => {
            out.push_str(" {\n");
            block(b, out);
            out.push_str("}\n");
        }
    }
}

pub fn decl(d: &Decl, out: &mut String) {
    match d {
        Decl::PragmaOnce => out.push_str("#pragma once\n"),
        Decl::Include { path, system: true } => {
            let _ = writeln!(out, "#include <{path}>");
        }
        Decl::Include { path, system: false } => {
            let _ = writeln!(out, "#include \"{path}\"");
        }
        Decl::Namespace(name, items) => {
            let _ = writeln!(out, "namespace {name} {{\n");
            for i in items {
                decl(i, out);
            }
            let label = if name.is_empty() { String::new() } else { format!(" {name}") };
            let _ = writeln!(out, "\n}}  // namespace{label}");
        }
        Decl::Comment(c) => {
            let _ = writeln!(out, "// {c}");
        }
        Decl::Blank => out.push('\n'),
        Decl::Enum { doc: d, name, underlying, variants } => {
            doc(d, out);
            let _ = writeln!(out, "enum class {name} : {} {{ {} }};", ty(underlying), variants.join(", "));
        }
        Decl::Struct { doc: d, keyword, name, members } => {
            doc(d, out);
            let _ = writeln!(out, "{keyword} {name} {{");
            for m in members {
                match m {
                    Member::Field { ty: t, name, init } => {
                        let _ = match init {
                            Some(i) => writeln!(out, "{} {name} = {};", ty(t), expr(i)),
                            None => writeln!(out, "{} {name};", ty(t)),
                        };
                    }
                    Member::Func(f) => func(f, out),
                    Member::Ctor { explicit, constexpr, name, params: ps, inits } => {
                        let mut pre = String::new();
                        if *explicit {
                            pre.push_str("explicit ");
                        }
                        if *constexpr {
                            pre.push_str("constexpr ");
                        }
                        let inits =
                            inits.iter().map(|(n, v)| format!("{n}({})", expr(v))).collect::<Vec<_>>().join(", ");
                        let _ = writeln!(out, "{pre}{name}({}) noexcept : {inits} {{}}", params(ps));
                    }
                    Member::Access(a) => {
                        let _ = writeln!(out, "\n{a}:");
                    }
                    Member::Comment(c) => {
                        let _ = writeln!(out, "// {c}");
                    }
                    Member::Using { name, ty: t } => {
                        let _ = writeln!(out, "using {name} = {};", ty(t));
                    }
                }
            }
            out.push_str("};\n");
        }
        Decl::Func(f) => func(f, out),
        Decl::Var { doc: d, prefix, ty: t, name, init } => {
            doc(d, out);
            for p in prefix {
                out.push_str(p);
                out.push(' ');
            }
            let _ = writeln!(out, "{} {name} = {};", ty(t), expr(init));
        }
        Decl::Using { name, ty: t } => {
            let _ = writeln!(out, "using {name} = {};", ty(t));
        }
        Decl::Marker(n) => {
            let _ = writeln!(out, "\n// @@slg {n}");
        }
        Decl::Line(l) => {
            out.push_str(l);
            out.push('\n');
        }
    }
}
