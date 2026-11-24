//! Typed expressions and statements to C++.

use std::collections::HashMap;

use spec_lang_diag::Diag;
use spec_lang_emit_rust::front::facts::{children, stmt_exprs};
use spec_lang_emit_rust::front::typed::*;
use spec_lang_ir::{CmpOp, DeclInfo, StateId, TypeDef};

use super::{Gen, camel, ident};
use crate::ast::{Expr, Stmt, Ty};

pub type R<X> = Result<X, Diag>;

/// How a local is reached.
#[derive(Clone, Debug)]
pub enum Local {
    /// By this expression.
    Value(Expr),
    /// Through this pointer.
    Ptr(Expr),
    /// A collection materialised as pointers to the elements of a state.
    PtrList(Expr),
    /// A collection of which only the size is ever used: the size itself.
    Count(Expr),
}

/// The scope of one declaration's lowering.
#[derive(Debug)]
pub struct Env<'g, 'f, 'p> {
    pub g: &'g Gen<'f, 'p>,
    pub decl: &'p DeclInfo,
    pub locals: HashMap<String, Local>,
    /// Owned locals used exactly once: their one use moves.
    pub movable: Vec<String>,
    /// The world: `w`.
    pub world: Expr,
    /// The lines `out.line` sends: `out`.
    pub out: Expr,
    /// Product code: a claim that is not discharged at build time has nowhere to fail.
    pub product: bool,
    /// `m[k]` is reached through this expression (the entry strategy).
    pub entry: Option<(StateId, String, Expr)>,
    /// A rule with guards returns `std::expected`.
    pub fallible: bool,
    /// Inside a law checked on read: `m[k]` is `m.at(k)`, and the caller wraps the law in `rt::holds`,
    /// so a missing key makes the law false.
    pub missing_is_false: bool,
    /// Anchors made while lowering, for the file to register.
    pub anchors: Vec<spec_lang_emit_rust::provenance::Origin>,
}

impl<'g, 'f, 'p> Env<'g, 'f, 'p> {
    pub fn new(g: &'g Gen<'f, 'p>, decl: &'p DeclInfo) -> Self {
        Env {
            g,
            decl,
            locals: HashMap::new(),
            movable: Vec::new(),
            world: Expr::id("w"),
            out: Expr::id("out"),
            product: true,
            entry: None,
            fallible: false,
            anchors: Vec::new(),
            missing_is_false: false,
        }
    }

    /// The text of a readable anchor comment; the file registers its origin.
    pub fn anchor(&mut self, origin: spec_lang_emit_rust::provenance::Origin) -> String {
        let text = crate::format::anchor_text(&origin);
        self.anchors.push(origin);
        text
    }

    pub fn bind(&mut self, name: &str, how: Local) {
        self.locals.insert(name.to_string(), how);
    }

    pub fn value(&mut self, name: &str) {
        self.bind(name, Local::Value(Expr::id(ident(name))));
    }

    fn no(&self, e: &TExpr, what: impl std::fmt::Display) -> Diag {
        self.g.unsupported(e.span, self.decl, what)
    }

    fn state_name(&self, s: StateId) -> String {
        ident(&self.g.f.prog.state(s).info.name)
    }

    /// An expression whose value is stored (a field, an insert, an argument that moves): a local used
    /// once moves.
    pub fn owned(&mut self, e: &TExpr) -> R<Expr> {
        if let K::Local(n) = &e.kind
            && self.movable.contains(n)
            && !self.g.trivial(&e.ty)
        {
            return Ok(self.expr(e)?.moved());
        }
        self.expr(e)
    }

    pub fn expr(&mut self, e: &TExpr) -> R<Expr> {
        Ok(match &e.kind {
            K::Unit => return Err(self.no(e, "a unit value")),
            K::Bool(b) => Expr::Bool(*b),
            K::Int(n) => Expr::Int(*n),
            K::Text(s) => Expr::str(s.clone()),
            K::Interp(parts) => {
                let mut args = Vec::new();
                for p in parts {
                    args.push(match p {
                        Part::Lit(s) => Expr::str(s.clone()),
                        Part::Expr(x) => self.shown(x)?,
                    });
                }
                Expr::call("rt::text::concat", args)
            }
            K::Local(n) => match self.locals.get(n) {
                Some(Local::Value(x) | Local::Count(x) | Local::PtrList(x)) => x.clone(),
                Some(Local::Ptr(p)) => p.clone().deref(),
                None => return Err(self.no(e, format!("the local `{n}` is not in scope here"))),
            },
            K::Const(_) => return Err(self.no(e, "a const")),
            K::State(s) => self.world.clone().member(&self.state_name(*s)),
            K::Variant { ty, tag } => {
                if *ty == self.g.f.prog.option_type {
                    Expr::id("std::nullopt")
                } else {
                    Expr::id(format!("{}::{}", self.g.type_path(*ty), variant_name(self.g, *ty, *tag)))
                }
            }
            K::Some(x) => Expr::Brace(Some(self.g.cty(&e.ty)), vec![self.owned(x)?]),
            K::Record { ty, fields } => {
                let Some(decls) = spec_lang_emit_rust::front::record_fields(self.g.f.prog, *ty) else {
                    return Err(self.no(e, "a record of a type that is not a record"));
                };
                let mut out = Vec::new();
                for (f, v) in decls.iter().zip(fields) {
                    out.push((ident(&f.name), self.owned(v)?));
                }
                Expr::Designated(Ty::Name(self.g.type_path(*ty)), out)
            }
            K::CallFn { f, args } => {
                let d = self.g.f.prog.func(*f);
                let name = format!("{}::{}", self.g.module_of(&d.info), ident(&d.info.name));
                let args = args.iter().map(|a| self.expr(a)).collect::<R<Vec<_>>>()?;
                Expr::call(name, args)
            }
            K::CallRule { rule, args } => {
                let d = self.g.f.prog.rule(*rule);
                let name = format!("{}::{}", self.g.module_of(&d.info), ident(&d.info.name));
                let mut all = vec![self.world.clone(), self.out.clone()];
                for a in args {
                    all.push(self.owned(a)?);
                }
                Expr::call(name, all)
            }
            K::OutLine(x) => {
                let v = self.expr(x)?;
                let how = if matches!(v, Expr::Str(_)) { "emplace_back" } else { "push_back" };
                self.out.clone().method(how, vec![v])
            }
            K::OutText => Expr::call("rt::test::text", vec![self.out.clone()]),
            K::Field(x, i) => {
                let T::Named(id) = x.ty else { return Err(self.no(e, "a field of a value that is not a record")) };
                let Some(fields) = spec_lang_emit_rust::front::record_fields(self.g.f.prog, id) else {
                    return Err(self.no(e, "a field of a value that is not a record"));
                };
                let name = ident(&fields[*i].name);
                match self.expr(x)? {
                    Expr::Un("*", p) => Expr::Arrow(p, name),
                    v => v.member(&name),
                }
            }
            K::Core(op, ops) => self.core(e, *op, ops)?,
            K::Lambda { .. } => return Err(self.no(e, "a lambda outside a collection method")),
            K::Index { map, key } => {
                if let (Some((s, k, with)), K::State(ms), K::Local(kn)) = (&self.entry, &map.kind, &key.kind)
                    && s == ms
                    && k == kn
                {
                    return Ok(with.clone());
                }
                if self.product && !self.missing_is_false {
                    return Err(Diag::error(
                        "E-UNSUPPORTED",
                        e.span,
                        "a map read with no guard that the key is in the map: `Missing` has nowhere to go",
                    )
                    .with_decl(self.g.path(self.decl))
                    .with_note("add `@guard(k in m else E)` before the body"));
                }
                let m = self.expr(map)?;
                let k = self.expr(key)?;
                m.method("at", vec![k])
            }
            K::Not(x) => self.expr(x)?.negate(),
            K::Neg(x) => Expr::Un("-", Box::new(self.expr(x)?)),
            K::Bin(op, a, b) => {
                let (a, b) = (self.expr(a)?, self.expr(b)?);
                match op {
                    Bin::Add => Expr::bin("+", a, b),
                    Bin::Sub => Expr::bin("-", a, b),
                    Bin::Mul => Expr::bin("*", a, b),
                    Bin::And => Expr::bin("&&", a, b),
                    Bin::Or => Expr::bin("||", a, b),
                    Bin::Implies => Expr::bin("||", a.negate(), b),
                }
            }
            K::Compare(head, links) => self.compare(head, links)?,
            K::Tuple(xs) => {
                let xs = xs.iter().map(|x| self.expr(x)).collect::<R<Vec<_>>>()?;
                Expr::Brace(Some(self.g.cty(&e.ty)), xs)
            }
            K::List(xs) | K::SetOf(xs) => {
                let xs = xs.iter().map(|x| self.owned(x)).collect::<R<Vec<_>>>()?;
                Expr::Brace(Some(self.g.cty(&e.ty)), xs)
            }
            K::MapOf(pairs) => {
                let mut xs = Vec::new();
                for (k, v) in pairs {
                    xs.push(Expr::Brace(None, vec![self.expr(k)?, self.owned(v)?]));
                }
                Expr::Brace(Some(self.g.cty(&e.ty)), xs)
            }
            K::If { cond, then, els } => {
                Expr::Cond(Box::new(self.expr(cond)?), Box::new(self.expr(then)?), Box::new(self.expr(els)?))
            }
            K::Match { scrutinee, arms } => self.match_expr(e, scrutinee, arms)?,
            K::Widen(x) => {
                let v = self.expr(x)?;
                match &x.ty {
                    T::Byte => Expr::Cast(Ty::name("std::int64_t"), Box::new(v)),
                    T::Named(id) => match self.g.alias_base(*id) {
                        Some(T::Int) => v.method("value", vec![]),
                        Some(T::Text) => v.method("text", vec![]),
                        _ => return Err(self.no(e, "widening this type")),
                    },
                    _ => return Err(self.no(e, "widening this type")),
                }
            }
            K::Narrow { expr: x, proof } => {
                let T::Named(id) = e.ty else { return Err(self.no(e, "narrowing to a type that is not named")) };
                let path = self.g.type_path(id);
                match proof {
                    Proof::Static(_) => Expr::call(format!("{path}::proven"), vec![self.expr(x)?]),
                    Proof::Runtime if self.product => {
                        return Err(Diag::error(
                            "E-UNSUPPORTED",
                            e.span,
                            format!(
                                "a {} claim that is not shown at build time, where a failure has nowhere to go",
                                self.g.f.prog.ty(id).info.name
                            ),
                        )
                        .with_decl(self.g.path(self.decl)));
                    }
                    Proof::Runtime => {
                        // In a test: the literal helper of support.hpp checks the claim and fails the test.
                        let _ = path;
                        Expr::call(
                            format!("support::{}", super::snake(&self.g.f.prog.ty(id).info.name)),
                            vec![self.expr(x)?],
                        )
                    }
                }
            }
            K::StepPart(x, p) => {
                let v = self.expr(x)?;
                match p {
                    StepPart::Before => v.member("before"),
                    StepPart::After => v.member("after"),
                    StepPart::Event => v.member("event"),
                    StepPart::Reply => v,
                }
            }
            K::WorldState(w, s) => {
                let name = self.state_name(*s);
                self.expr(w)?.member(&name)
            }
            K::EventArg { event, name } => Expr::call(format!("event_{name}"), vec![self.expr(event)?]),
            K::IsEvent { event, rule } => {
                let r = self.g.f.prog.rule(*rule);
                Expr::CallT(
                    Box::new(Expr::id("std::holds_alternative")),
                    vec![Ty::name(format!("{}::{}", self.g.module_of(&r.info), camel(&r.info.name)))],
                    vec![self.expr(event)?],
                )
            }
            K::IsFail(x) => self.expr(x)?.member("refused"),
        })
    }

    /// A part of an interpolated text, as something `rt::text::concat` appends.
    fn shown(&mut self, x: &TExpr) -> R<Expr> {
        let v = self.expr(x)?;
        Ok(match &x.ty {
            T::Named(id) => match self.g.alias_base(*id) {
                Some(T::Int) => v.method("value", vec![]),
                Some(T::Text) => v.method("text", vec![]),
                _ => return Err(self.no(x, "showing a value of this type in a text")),
            },
            T::Byte => Expr::Cast(Ty::name("std::int64_t"), Box::new(v)),
            T::Int | T::Text => v,
            _ => return Err(self.no(x, "showing a value of this type in a text")),
        })
    }

    fn compare(&mut self, head: &TExpr, links: &[(CmpOp, TExpr)]) -> R<Expr> {
        // `size == 0`, `size >= 1`: emptiness, which needs no count.
        if let [(op, rhs)] = links
            && let K::Core(Op::Size, xs) = &head.kind
            && let K::Int(n) = rhs.kind
            && let [x] = xs.as_slice()
        {
            let empty = match (op, n) {
                (CmpOp::Eq, 0) | (CmpOp::Lt, 1) | (CmpOp::Le, 0) => Some(true),
                (CmpOp::Ne, 0) | (CmpOp::Ge, 1) | (CmpOp::Gt, 0) => Some(false),
                _ => None,
            };
            if let Some(empty) = empty
                && !matches!(&x.kind, K::Local(l) if matches!(self.locals.get(l), Some(Local::Count(_))))
            {
                let v = self.collection(x)?.method("empty", vec![]);
                return Ok(if empty { v } else { v.negate() });
            }
        }
        let typed: Vec<&TExpr> = std::iter::once(head).chain(links.iter().map(|(_, r)| r)).collect();
        let mut operands = vec![self.expr(head)?];
        for (_, r) in links {
            operands.push(self.expr(r)?);
        }
        // `lo <= x <= hi`.
        let mut out: Option<Expr> = None;
        for (i, (op, _)) in links.iter().enumerate() {
            let sym = match op {
                CmpOp::Eq => "==",
                CmpOp::Ne => "!=",
                CmpOp::Lt => "<",
                CmpOp::Le => "<=",
                CmpOp::Gt => ">",
                CmpOp::Ge => ">=",
                CmpOp::In => return Err(self.no(head, "`in` in a chain")),
            };
            let link = match self.size_link(*op, typed[i], typed[i + 1])? {
                Some(fast) => fast,
                None => Expr::bin(sym, operands[i].clone(), operands[i + 1].clone()),
            };
            out = Some(match out {
                None => link,
                Some(prev) => Expr::bin("&&", prev, link),
            });
        }
        out.ok_or_else(|| self.no(head, "an empty comparison"))
    }

    /// `t.size <= n`, `n >= t.size`, `t.size >= 1` on a Text: decided from the bytes where they decide it
    /// (a code point is one to four bytes), counting code points only past that.
    fn size_link(&mut self, op: CmpOp, a: &TExpr, b: &TExpr) -> R<Option<Expr>> {
        fn text_size(e: &TExpr) -> Option<&TExpr> {
            match &e.kind {
                K::Core(Op::Size, xs) if xs.len() == 1 && xs[0].ty == T::Text => Some(&xs[0]),
                _ => None,
            }
        }
        let int = |e: &TExpr| match e.kind {
            K::Int(n) => Some(n),
            _ => None,
        };
        let (x, n, op) = match (text_size(a), int(b), text_size(b), int(a)) {
            (Some(x), Some(n), _, _) => (x, n, op),
            (_, _, Some(x), Some(n)) => {
                let flipped = match op {
                    CmpOp::Le => CmpOp::Ge,
                    CmpOp::Lt => CmpOp::Gt,
                    CmpOp::Ge => CmpOp::Le,
                    CmpOp::Gt => CmpOp::Lt,
                    o => o,
                };
                (x, n, flipped)
            }
            _ => return Ok(None),
        };
        let v = self.expr(x)?;
        Ok(match (op, n) {
            (CmpOp::Le, n) => Some(Expr::call("rt::text::size_at_most", vec![v, Expr::Int(n)])),
            (CmpOp::Lt, n) => Some(Expr::call("rt::text::size_at_most", vec![v, Expr::Int(n - 1)])),
            (CmpOp::Ge, 1) | (CmpOp::Gt, 0) => Some(v.method("empty", vec![]).negate()),
            _ => None,
        })
    }

    /// A collection or text as something with `.size()`/`.empty()` or a range.
    fn collection(&mut self, x: &TExpr) -> R<Expr> {
        if x.ty == T::Text {
            return self.expr(x);
        }
        self.expr(x)
    }

    fn core(&mut self, e: &TExpr, op: Op, ops: &[TExpr]) -> R<Expr> {
        let first = ops.first().ok_or_else(|| self.no(e, "a core operation without its receiver"))?;
        Ok(match op {
            Op::Size => {
                if let K::Local(n) = &first.kind
                    && let Some(Local::Count(c)) = self.locals.get(n)
                {
                    return Ok(c.clone());
                }
                let v = self.expr(first)?;
                if first.ty == T::Text {
                    Expr::call("rt::text::size", vec![v])
                } else {
                    Expr::call("rt::length", vec![v])
                }
            }
            Op::Trim => Expr::call("rt::text::trim", vec![self.expr(first)?]),
            Op::Bytes => Expr::call("std::string_view", vec![self.expr(first)?]),
            Op::Keys => Expr::bin("|", self.expr(first)?, Expr::id("std::views::keys")),
            Op::Values => Expr::bin("|", self.expr(first)?, Expr::id("std::views::values")),
            Op::Contains => {
                let [x, coll] = ops else { return Err(self.no(e, "`in` without two operands")) };
                let c = self.expr(coll)?;
                let x = self.expr(x)?;
                match coll.ty {
                    T::List(_) => Expr::call("std::ranges::contains", vec![c, x]),
                    _ => c.method("contains", vec![x]),
                }
            }
            Op::All | Op::Any | Op::Filter | Op::Map => {
                let [r, l] = ops else { return Err(self.no(e, "a collection method without its lambda")) };
                let range = self.expr(r)?;
                let elem = r.ty.elem().cloned().unwrap_or(T::Unit);
                let lambda = self.lambda(l, &elem, &range)?;
                match op {
                    Op::All => Expr::call("std::ranges::all_of", vec![range, lambda]),
                    Op::Any => Expr::call("std::ranges::any_of", vec![range, lambda]),
                    Op::Filter => Expr::bin("|", range, Expr::call("std::views::filter", vec![lambda])),
                    _ => Expr::bin("|", range, Expr::call("std::views::transform", vec![lambda])),
                }
            }
            Op::Sort => {
                let [r, l] = ops else { return Err(self.no(e, "`sort` without its key")) };
                if matches!(r.ty, T::Set(_)) && is_identity(l) {
                    // A Set iterates in ascending order already: sorting by the element is the identity.
                    return self.expr(r);
                }
                return Err(self.no(e, "`sort` outside a `let`"));
            }
            Op::TextContains | Op::StartsWith => {
                let [t, part] = ops else { return Err(self.no(e, "a text method without its operand")) };
                let method = if op == Op::TextContains { "contains" } else { "starts_with" };
                Expr::call("std::string_view", vec![self.expr(t)?]).method(method, vec![self.expr(part)?])
            }
            // A std::set iterates in ascending order: its list is itself.
            Op::List => self.expr(first)?,
            Op::Join => {
                let [r, sep] = ops else { return Err(self.no(e, "`join` without its separator")) };
                Expr::call("rt::text::join", vec![self.expr(r)?, self.expr(sep)?])
            }
            Op::Get => {
                let [m, k] = ops else { return Err(self.no(e, "`get` without its key")) };
                Expr::call("rt::get", vec![self.expr(m)?, self.expr(k)?])
            }
            Op::States => self.expr(first)?.method("states", vec![]),
            Op::Steps => self.expr(first)?.member("steps"),
        })
    }

    /// A one-parameter lambda over the elements of `range`.
    fn lambda(&mut self, l: &TExpr, elem: &T, _range: &Expr) -> R<Expr> {
        let K::Lambda { param, body } = &l.kind else { return Err(self.no(l, "a collection method without a lambda")) };
        let ty = if *elem == T::Byte { Ty::name("unsigned char") } else { Ty::Auto.cref() };
        let saved = self.locals.remove(param);
        self.value(param);
        let b = self.expr(body);
        self.locals.remove(param);
        if let Some(s) = saved {
            self.locals.insert(param.clone(), s);
        }
        Ok(Expr::Lambda {
            capture: "&",
            params: vec![(ty, ident(param))],
            ret: None,
            body: vec![Stmt::Return(Some(b?))],
        })
    }

    /// `match` as a value: a test of one pattern, or an immediately invoked switch.
    fn match_expr(&mut self, e: &TExpr, scrutinee: &TExpr, arms: &[(Pat, TExpr)]) -> R<Expr> {
        if let [(p, TExpr { kind: K::Bool(true), .. }), (Pat::Wild, TExpr { kind: K::Bool(false), .. })] = arms {
            let s = self.expr(scrutinee)?;
            return match p {
                Pat::Variant { ty, tag, args } if args.iter().all(|a| matches!(a, Pat::Wild)) => {
                    if *ty == self.g.f.prog.option_type {
                        let has = s.method("has_value", vec![]);
                        Ok(if *tag == spec_lang_ir::SOME_TAG { has } else { has.negate() })
                    } else {
                        let v = Expr::id(format!("{}::{}", self.g.type_path(*ty), variant_name(self.g, *ty, *tag)));
                        Ok(Expr::bin("==", s, v))
                    }
                }
                _ => Err(self.no(e, "this pattern in `.is(...)`")),
            };
        }
        let body = self.match_stmts(e, scrutinee, arms, &mut |_, v| Ok(Stmt::Return(Some(v))))?;
        Ok(Expr::Call(Box::new(Expr::Lambda { capture: "&", params: vec![], ret: None, body }), vec![]))
    }

    /// `match` as statements: each arm ends in what `arm` makes of its value.
    pub fn match_stmts(
        &mut self,
        e: &TExpr,
        scrutinee: &TExpr,
        arms: &[(Pat, TExpr)],
        arm: &mut dyn FnMut(&mut Self, Expr) -> R<Stmt>,
    ) -> R<Vec<Stmt>> {
        let s = self.expr(scrutinee)?;
        match &scrutinee.ty {
            T::Option(_) => {
                let mut none = None;
                let mut some = None;
                for (p, v) in arms {
                    match p {
                        Pat::Variant { tag, args, .. } if *tag == spec_lang_ir::NONE_TAG && args.is_empty() => {
                            none = Some(v)
                        }
                        Pat::Variant { tag, args, .. } if *tag == spec_lang_ir::SOME_TAG => {
                            some = Some((args.first(), v))
                        }
                        _ => return Err(self.no(e, "this pattern on an Option")),
                    }
                }
                let (Some(none), Some((bind, some))) = (none, some) else {
                    return Err(self.no(e, "a match on an Option without both arms"));
                };
                let n = self.expr(none)?;
                let mut then = vec![arm(self, n)?];
                if let Some(Pat::Bind(x)) = bind {
                    self.bind(x, Local::Value(s.clone().deref()));
                }
                let v = self.expr(some)?;
                let mut rest = vec![arm(self, v)?];
                let mut out = vec![Stmt::If {
                    cond: s.method("has_value", vec![]).negate(),
                    then: std::mem::take(&mut then),
                    els: vec![],
                }];
                out.append(&mut rest);
                Ok(out)
            }
            T::Named(ty) if matches!(self.g.f.prog.ty(*ty).def, TypeDef::Sum(_)) => {
                let mut cases = Vec::new();
                for (p, v) in arms {
                    let Pat::Variant { ty, tag, args } = p else {
                        return Err(self.no(e, "a wildcard arm on a sum: name every variant"));
                    };
                    if !args.is_empty() {
                        return Err(self.no(e, "a variant with fields"));
                    }
                    let label = Expr::id(format!("{}::{}", self.g.type_path(*ty), variant_name(self.g, *ty, *tag)));
                    let v = self.expr(v)?;
                    cases.push((label, vec![arm(self, v)?]));
                }
                Ok(vec![Stmt::Switch { on: s, cases }, Stmt::Expr(Expr::call("std::unreachable", vec![]))])
            }
            _ => Err(self.no(e, "a match on this type")),
        }
    }
}

fn is_identity(l: &TExpr) -> bool {
    matches!(&l.kind, K::Lambda { param, body } if matches!(&body.kind, K::Local(n) if n == param))
}

pub fn variant_name(g: &Gen<'_, '_>, ty: spec_lang_ir::TypeId, tag: u32) -> String {
    match &g.f.prog.ty(ty).def {
        TypeDef::Sum(vs) => vs.get(tag as usize).map(|v| v.name.clone()).unwrap_or_default(),
        _ => String::new(),
    }
}

/// How many times a local is read in a body.
pub fn uses(body: &[TStmt], name: &str) -> usize {
    let mut n = 0;
    for s in body {
        stmt_exprs(s, &mut |e| count(e, name, &mut n));
    }
    n
}

pub fn count(e: &TExpr, name: &str, n: &mut usize) {
    if matches!(&e.kind, K::Local(x) if x == name) {
        *n += 1;
    }
    children(e, &mut |c| count(c, name, n));
}
