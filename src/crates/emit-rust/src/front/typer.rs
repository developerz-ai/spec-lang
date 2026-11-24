//! IR to the typed tree. The IR resolves names but has no expression types;
//! this pass finds them, bidirectionally: an expected type flows down into
//! literals and lambdas, a found type flows up. Refinements are explicit in
//! the output: a value enters a refined type through [`K::Narrow`] with a
//! [`Proof`], and leaves it through [`K::Widen`].
//!
//! Anything outside the subset the backends print is an `E-UNSUPPORTED`
//! diagnostic at its span, never a guess.

use std::collections::HashMap;

use spec_lang_diag::Diag;
use spec_lang_ir::{
    self as ir, BinOp, Callee, CmpOp, Effect, ExprKind, InterpPart, LambdaBody, MutOp, Pattern, PlaceRoot, PlaceStep,
    Program, StmtKind, Ty, TypeDef, TypeId, UnOp,
};
use spec_lang_syntax::Span;

use super::typed::*;

pub type R<X> = Result<X, Diag>;

/// Types expressions of one declaration.
#[derive(Debug)]
pub struct Typer<'p> {
    pub prog: &'p Program,
    /// The declaration being typed, dotted, for diagnostics.
    pub decl: String,
    scopes: Vec<HashMap<String, T>>,
    /// The result type of the enclosing fn or rule, for `return`.
    pub result: T,
}

impl<'p> Typer<'p> {
    pub fn new(prog: &'p Program, decl: String) -> Self {
        Typer { prog, decl, scopes: vec![HashMap::new()], result: T::Unit }
    }

    pub fn bind(&mut self, name: &str, ty: T) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.scopes.pop();
    }

    fn local(&self, name: &str) -> Option<&T> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    pub fn unsupported(&self, span: Span, what: impl std::fmt::Display) -> Diag {
        Diag::unsupported(span, what).with_decl(self.decl.clone())
    }

    fn mismatch(&self, span: Span, expected: &T, found: &T) -> Diag {
        Diag::error("E-TYPE-MISMATCH", span, "a value is not of the type the place needs")
            .with_decl(self.decl.clone())
            .with_expected(type_name(self.prog, expected), type_name(self.prog, found))
    }

    // ---- types ----

    pub fn ty(&self, ty: &Ty, span: Span) -> R<T> {
        Ok(match ty {
            Ty::Unit => T::Unit,
            Ty::Bool => T::Bool,
            Ty::Int => T::Int,
            Ty::Text => T::Text,
            Ty::Named(id) => match &self.prog.ty(*id).def {
                TypeDef::Rules(rules) => T::Event(rules.clone()),
                _ => T::Named(*id),
            },
            Ty::Option(t) => T::Option(Box::new(self.ty(t, span)?)),
            Ty::List(t) => T::List(Box::new(self.ty(t, span)?)),
            Ty::Set(t) => T::Set(Box::new(self.ty(t, span)?)),
            Ty::Map(k, v) => T::Map(Box::new(self.ty(k, span)?), Box::new(self.ty(v, span)?)),
            Ty::Tuple(ts) => T::Tuple(ts.iter().map(|t| self.ty(t, span)).collect::<R<_>>()?),
            Ty::Trace(rules) => T::Trace(rules.clone()),
            Ty::Range { .. } => return Err(self.unsupported(span, "a range type outside a named type")),
            Ty::Where(..) => return Err(self.unsupported(span, "a `where` claim outside a named type")),
        })
    }

    /// The base of a refined alias (`Title` gives `Text`), with its claim.
    pub fn alias_base(&self, id: TypeId) -> Option<(T, Option<&'p ir::Claim>)> {
        alias_base(self.prog, id)
    }

    // ---- expressions ----

    /// Types `e`; when `want` is given, the result has exactly that type.
    pub fn expr(&mut self, e: &ir::Expr, want: Option<&T>) -> R<TExpr> {
        let found = self.infer(e, want)?;
        match want {
            Some(w) => self.coerce(found, w),
            None => Ok(found),
        }
    }

    /// Makes `e` of type `want`, widening or narrowing a refinement.
    pub fn coerce(&self, e: TExpr, want: &T) -> R<TExpr> {
        if &e.ty == want {
            return Ok(e);
        }
        let span = e.span;
        match (&e.ty, want) {
            (T::Byte, T::Int) => return Ok(widen(e, T::Int)),
            (T::Named(id), base) => {
                if let Some((b, _)) = self.alias_base(*id)
                    && &b == base
                {
                    return Ok(widen(e, b));
                }
            }
            _ => {}
        }
        if let T::Named(id) = want
            && let Some((base, claim)) = self.alias_base(*id)
        {
            let inner = self.coerce(e, &base)?;
            let proof = match claim {
                None => Proof::Static("the type has no claim".into()),
                Some(c) => self.prove(&inner, c, *id)?,
            };
            return Ok(TExpr { kind: K::Narrow { expr: Box::new(inner), proof }, ty: want.clone(), span });
        }
        Err(self.mismatch(span, want, &e.ty))
    }

    /// Discharges a claim at build time where a fact in the program shows it,
    /// else leaves it to a check when the value is made.
    fn prove(&self, value: &TExpr, claim: &ir::Claim, ty: TypeId) -> R<Proof> {
        let name = &self.prog.ty(ty).info.name;
        if let Some(bound) = int_claim_lower_bound(&claim.expr)
            && let Some(lo) = lower_bound(self.prog, value)
        {
            if lo >= bound {
                return Ok(Proof::Static(format!("{name}'s claim self >= {bound} holds: the value is at least {lo}")));
            }
            if let K::Int(_) = value.kind {
                return Err(Diag::error("E-CLAIM", value.span, format!("the literal is not a {name}"))
                    .with_decl(self.decl.clone())
                    .with_expected(format!("an Int >= {bound}"), lo.to_string()));
            }
        }
        Ok(Proof::Runtime)
    }

    fn infer(&mut self, e: &ir::Expr, want: Option<&T>) -> R<TExpr> {
        let span = e.span;
        let mk = |kind: K, ty: T| Ok(TExpr { kind, ty, span });
        match &e.kind {
            ExprKind::Unit => mk(K::Unit, T::Unit),
            ExprKind::Bool(b) => mk(K::Bool(*b), T::Bool),
            ExprKind::Int(n) => {
                if want == Some(&T::Byte) && (0..=255).contains(n) {
                    return mk(K::Int(*n), T::Byte);
                }
                mk(K::Int(*n), T::Int)
            }
            ExprKind::Text(s) => mk(K::Text(s.clone()), T::Text),
            ExprKind::Interp(parts) => {
                let mut out = Vec::new();
                for p in parts {
                    out.push(match p {
                        InterpPart::Lit(s) => Part::Lit(s.clone()),
                        InterpPart::Expr(x) => Part::Expr(self.expr(x, None)?),
                    });
                }
                mk(K::Interp(out), T::Text)
            }
            ExprKind::Local(name) => {
                let ty = self
                    .local(name)
                    .cloned()
                    .ok_or_else(|| self.unsupported(span, format!("the local `{name}` has no known type")))?;
                mk(K::Local(name.clone()), ty)
            }
            ExprKind::Const(id) => {
                let c = self.prog.constant(*id);
                let ty = match &c.ty {
                    Some(t) => self.ty(t, span)?,
                    None => return Err(self.unsupported(span, "a const without a written type")),
                };
                mk(K::Const(*id), ty)
            }
            ExprKind::State(id) => {
                let ty = self.ty(&self.prog.state(*id).ty, span)?;
                mk(K::State(*id), ty)
            }
            ExprKind::Variant { ty, tag } => {
                let t = if *ty == self.prog.option_type {
                    match want {
                        Some(w @ T::Option(_)) => w.clone(),
                        _ => return Err(self.unsupported(span, "`none` where no Option type is expected")),
                    }
                } else {
                    T::Named(*ty)
                };
                mk(K::Variant { ty: *ty, tag: *tag }, t)
            }
            ExprKind::Call { callee, args } => self.call(*callee, args, want, span),
            ExprKind::Prop { recv, name } => self.prop(recv, name, span),
            ExprKind::Method { recv, name, args } => self.method(recv, name, args, want, span),
            ExprKind::Index { recv, key } => {
                let map = self.expr(recv, None)?;
                let T::Map(k, v) = map.ty.clone() else {
                    return Err(self.unsupported(span, "indexing a value that is not a Map"));
                };
                let key = self.expr(key, Some(&k))?;
                mk(K::Index { map: Box::new(map), key: Box::new(key) }, *v)
            }
            ExprKind::Is { value, pattern } => self.is(value, pattern, span),
            ExprKind::Unary(op, x) => match op {
                UnOp::Not => {
                    let x = self.expr(x, Some(&T::Bool))?;
                    mk(K::Not(Box::new(x)), T::Bool)
                }
                UnOp::Neg => {
                    let x = self.expr(x, Some(&T::Int))?;
                    mk(K::Neg(Box::new(x)), T::Int)
                }
                _ => Err(self.unsupported(span, "this unary operator")),
            },
            ExprKind::Binary(op, a, b) => {
                let (bin, ty) = match op {
                    BinOp::Add => (Bin::Add, T::Int),
                    BinOp::Sub => (Bin::Sub, T::Int),
                    BinOp::Mul => (Bin::Mul, T::Int),
                    BinOp::And => (Bin::And, T::Bool),
                    BinOp::Or => (Bin::Or, T::Bool),
                    BinOp::Implies => (Bin::Implies, T::Bool),
                    _ => return Err(self.unsupported(span, "this binary operator")),
                };
                let a = self.expr(a, Some(&ty))?;
                let b = self.expr(b, Some(&ty))?;
                mk(K::Bin(bin, Box::new(a), Box::new(b)), ty)
            }
            ExprKind::Compare(first, links) => self.compare(first, links, span),
            ExprKind::Tuple(items) => {
                let wants: Vec<Option<T>> = match want {
                    Some(T::Tuple(ts)) if ts.len() == items.len() => ts.iter().cloned().map(Some).collect(),
                    _ => vec![None; items.len()],
                };
                let mut out = Vec::new();
                for (x, w) in items.iter().zip(wants) {
                    out.push(self.expr(x, w.as_ref())?);
                }
                let ty = T::Tuple(out.iter().map(|x| x.ty.clone()).collect());
                mk(K::Tuple(out), ty)
            }
            ExprKind::List(items) => {
                let elem = match want {
                    Some(T::List(e)) => Some((**e).clone()),
                    _ => None,
                };
                let out = self.items(items, elem.as_ref(), span)?;
                let ty = T::List(Box::new(elem.or_else(|| out.first().map(|x| x.ty.clone())).unwrap_or(T::Unit)));
                mk(K::List(out), ty)
            }
            ExprKind::SetOf(items) => {
                let elem = match want {
                    Some(T::Set(e)) => Some((**e).clone()),
                    _ => None,
                };
                let out = self.items(items, elem.as_ref(), span)?;
                let Some(elem) = elem.or_else(|| out.first().map(|x| x.ty.clone())) else {
                    return Err(self.unsupported(span, "an empty `Set()` where no Set type is expected"));
                };
                mk(K::SetOf(out), T::Set(Box::new(elem)))
            }
            ExprKind::MapOf(pairs) => {
                let Some(T::Map(k, v)) = want else {
                    return Err(self.unsupported(span, "a `Map(...)` where no Map type is expected"));
                };
                let mut out = Vec::new();
                for (a, b) in pairs {
                    out.push((self.expr(a, Some(k))?, self.expr(b, Some(v))?));
                }
                mk(K::MapOf(out), T::Map(k.clone(), v.clone()))
            }
            ExprKind::If { cond, then, els } => {
                let cond = self.expr(cond, Some(&T::Bool))?;
                let then = self.expr(then, want)?;
                let els = self.expr(els, Some(&then.ty))?;
                let ty = then.ty.clone();
                mk(K::If { cond: Box::new(cond), then: Box::new(then), els: Box::new(els) }, ty)
            }
            ExprKind::Match { scrutinee, arms } => {
                let scrutinee = self.expr(scrutinee, None)?;
                let mut out: Vec<(Pat, TExpr)> = Vec::new();
                let mut ty = want.cloned();
                for arm in arms {
                    self.push();
                    let pat = self.pattern(&arm.pattern, &scrutinee.ty, arm.span)?;
                    let value = self.expr(&arm.value, ty.as_ref());
                    self.pop();
                    let value = value?;
                    ty.get_or_insert_with(|| value.ty.clone());
                    out.push((pat, value));
                }
                let ty = ty.unwrap_or(T::Unit);
                mk(K::Match { scrutinee: Box::new(scrutinee), arms: out }, ty)
            }
            ExprKind::OutText => mk(K::OutText, T::Text),
            ExprKind::Lambda(_) => Err(self.unsupported(span, "a lambda outside a collection method")),
            ExprKind::Block(_) => Err(self.unsupported(span, "a block used as a value inside an expression")),
            ExprKind::Fail(_) => Err(self.unsupported(span, "`fail` inside an expression; use a @guard")),
            ExprKind::Else { .. } => Err(self.unsupported(span, "`else` on a value")),
            ExprKind::With { .. } => Err(self.unsupported(span, "`.with(...)`")),
            ExprKind::Range { .. } => Err(self.unsupported(span, "a range value")),
            ExprKind::CallValue { .. } => Err(self.unsupported(span, "calling a lambda held in a local")),
            ExprKind::Hole(name) => Err(self.unsupported(span, format!("the hole `?{name}`: nothing to emit"))),
        }
    }

    fn items(&mut self, items: &[ir::Expr], elem: Option<&T>, _span: Span) -> R<Vec<TExpr>> {
        let mut out: Vec<TExpr> = Vec::new();
        for x in items {
            let want = elem.cloned().or_else(|| out.first().map(|f| f.ty.clone()));
            out.push(self.expr(x, want.as_ref())?);
        }
        Ok(out)
    }

    fn call(&mut self, callee: Callee, args: &[Option<ir::Expr>], want: Option<&T>, span: Span) -> R<TExpr> {
        let mk = |kind: K, ty: T| Ok(TExpr { kind, ty, span });
        match callee {
            Callee::Fn(f) => {
                let decl = self.prog.func(f);
                let mut out = Vec::new();
                for (p, a) in decl.params.iter().zip(args) {
                    let pty = self.ty(&p.ty, p.span)?;
                    out.push(self.arg(a.as_ref(), p.default.as_ref(), &pty, span)?);
                }
                let ty = match &decl.result {
                    Some(t) => self.ty(t, decl.info.span)?,
                    None => T::Unit,
                };
                mk(K::CallFn { f, args: out }, ty)
            }
            Callee::Rule(r) => {
                let decl = self.prog.rule(r);
                let mut out = Vec::new();
                for (p, a) in decl.params.iter().zip(args) {
                    let pty = self.ty(&p.ty, p.span)?;
                    out.push(self.arg(a.as_ref(), p.default.as_ref(), &pty, span)?);
                }
                let ty = match &decl.result {
                    Some(t) => self.ty(t, decl.info.span)?,
                    None => T::Unit,
                };
                mk(K::CallRule { rule: r, args: out }, ty)
            }
            Callee::Record(ty) => {
                let TypeDef::Record(fields) = &self.prog.ty(ty).def else {
                    return Err(self.unsupported(span, "constructing a type that is not a record"));
                };
                let mut out = Vec::new();
                for (f, a) in fields.iter().zip(args) {
                    let fty = self.ty(&f.ty, f.span)?;
                    out.push(self.arg(a.as_ref(), f.default.as_ref(), &fty, span)?);
                }
                mk(K::Record { ty, fields: out }, T::Named(ty))
            }
            Callee::Variant(ty, tag) if ty == self.prog.option_type && tag == spec_lang_ir::SOME_TAG => {
                let inner_want = match want {
                    Some(T::Option(t)) => Some((**t).clone()),
                    _ => None,
                };
                let Some(Some(x)) = args.first() else {
                    return Err(self.unsupported(span, "`Some` without its value"));
                };
                let x = self.expr(x, inner_want.as_ref())?;
                let ty = T::Option(Box::new(x.ty.clone()));
                mk(K::Some(Box::new(x)), ty)
            }
            Callee::Variant(..) => Err(self.unsupported(span, "a sum variant with fields")),
            Callee::Effect(Effect::OutLine) => {
                let Some(Some(x)) = args.first() else {
                    return Err(self.unsupported(span, "`out.line` without its text"));
                };
                let x = self.expr(x, Some(&T::Text))?;
                mk(K::OutLine(Box::new(x)), T::Unit)
            }
        }
    }

    /// An argument, or the parameter's default when it is not given.
    fn arg(&mut self, given: Option<&ir::Expr>, default: Option<&ir::Expr>, ty: &T, span: Span) -> R<TExpr> {
        match (given, default) {
            (Some(x), _) | (None, Some(x)) => self.expr(x, Some(ty)),
            (None, None) => Err(self.unsupported(span, "a missing argument without a default")),
        }
    }

    fn prop(&mut self, recv: &ir::Expr, name: &str, span: Span) -> R<TExpr> {
        let r = self.expr(recv, None)?;
        self.prop_of(r, name, span)
    }

    fn prop_of(&mut self, r: TExpr, name: &str, span: Span) -> R<TExpr> {
        let mk = |kind: K, ty: T| Ok(TExpr { kind, ty, span });
        let core = |op: Op, r: TExpr, ty: T| mk(K::Core(op, vec![r]), ty);
        match (&r.ty, name) {
            (T::Named(id), _) => {
                let id = *id;
                match &self.prog.ty(id).def {
                    TypeDef::Record(fields) => {
                        let Some((i, f)) = fields.iter().enumerate().find(|(_, f)| f.name == name) else {
                            return Err(self.unsupported(span, format!("no field `{name}`")));
                        };
                        let ty = self.ty(&f.ty, f.span)?;
                        mk(K::Field(Box::new(r), i), ty)
                    }
                    TypeDef::Alias(_) => {
                        let Some((base, _)) = self.alias_base(id) else {
                            return Err(self.unsupported(span, "an alias without a base"));
                        };
                        let r = widen(r, base);
                        self.prop_of(r, name, span)
                    }
                    _ => Err(self.unsupported(span, format!("`.{name}` on this type"))),
                }
            }
            (T::Text, "size") => core(Op::Size, r, T::Int),
            (T::Text, "bytes") => core(Op::Bytes, r, T::List(Box::new(T::Byte))),
            (T::List(_) | T::Set(_) | T::Map(..), "size") => core(Op::Size, r, T::Int),
            (T::Set(e), "list") => {
                let ty = T::List(e.clone());
                core(Op::List, r, ty)
            }
            (T::Map(k, _), "keys") => {
                let ty = T::Set(k.clone());
                core(Op::Keys, r, ty)
            }
            (T::Map(_, v), "values") => {
                let ty = T::List(v.clone());
                core(Op::Values, r, ty)
            }
            (T::Trace(_), "states") => core(Op::States, r, T::List(Box::new(T::World))),
            (T::Trace(rules), "steps") => {
                let ty = T::List(Box::new(T::Step(rules.clone())));
                core(Op::Steps, r, ty)
            }
            (T::Step(rules), part) => {
                let (p, ty) = match part {
                    "before" => (StepPart::Before, T::World),
                    "after" => (StepPart::After, T::World),
                    "event" => (StepPart::Event, T::Event(rules.clone())),
                    "reply" => (StepPart::Reply, T::Reply),
                    _ => return Err(self.unsupported(span, format!("`.{part}` of a step"))),
                };
                mk(K::StepPart(Box::new(r), p), ty)
            }
            (T::World, state) => {
                let found: Vec<_> = self.prog.states.iter().enumerate().filter(|(_, s)| s.info.name == state).collect();
                let [(i, s)] = found.as_slice() else {
                    return Err(self.unsupported(span, format!("`.{state}` names no single state of the world")));
                };
                let ty = self.ty(&s.ty, s.info.span)?;
                mk(K::WorldState(Box::new(r), spec_lang_ir::StateId(*i as u32)), ty)
            }
            (T::Event(rules), param) => {
                let mut ty = None;
                for rule in rules {
                    let decl = self.prog.rule(*rule);
                    let Some(p) = decl.params.iter().find(|p| p.name == param) else {
                        return Err(
                            self.unsupported(span, format!("rule {} has no parameter `{param}`", decl.info.name))
                        );
                    };
                    let t = self.ty(&p.ty, p.span)?;
                    if ty.as_ref().is_some_and(|x| x != &t) {
                        return Err(self.unsupported(span, format!("`{param}` has two types across the trace's rules")));
                    }
                    ty = Some(t);
                }
                let ty = ty.unwrap_or(T::Unit);
                mk(K::EventArg { event: Box::new(r), name: param.to_string() }, ty)
            }
            _ => Err(self.unsupported(span, format!("`.{name}` on {}", type_name(self.prog, &r.ty)))),
        }
    }

    fn method(&mut self, recv: &ir::Expr, name: &str, args: &[ir::Arg], want: Option<&T>, span: Span) -> R<TExpr> {
        let r = self.expr(recv, None)?;
        let r = match &r.ty {
            T::Named(id) if matches!(self.prog.ty(*id).def, TypeDef::Alias(_)) => {
                let base = self.alias_base(*id).map(|(b, _)| b).unwrap_or(T::Unit);
                widen(r, base)
            }
            _ => r,
        };
        let mk = |op: Op, operands: Vec<TExpr>, ty: T| Ok(TExpr { kind: K::Core(op, operands), ty, span });
        let elem = match &r.ty {
            T::List(e) | T::Set(e) => Some((**e).clone()),
            _ => None,
        };
        match (name, &r.ty, args) {
            ("trim", T::Text, []) => mk(Op::Trim, vec![r], T::Text),
            ("contains" | "starts_with", T::Text, [x]) => {
                let x = self.expr(&x.value, Some(&T::Text))?;
                let op = if name == "contains" { Op::TextContains } else { Op::StartsWith };
                mk(op, vec![r, x], T::Bool)
            }
            ("join", T::List(e), [sep]) if **e == T::Text => {
                let sep = self.expr(&sep.value, Some(&T::Text))?;
                mk(Op::Join, vec![r, sep], T::Text)
            }
            ("get", T::Map(k, v), [key]) => {
                let key = self.expr(&key.value, Some(k))?;
                let ty = T::Option(v.clone());
                mk(Op::Get, vec![r, key], ty)
            }
            ("all" | "any" | "filter" | "map" | "sort", _, [f]) => {
                let Some(elem) = elem else {
                    return Err(self.unsupported(span, format!("`.{name}` on {}", type_name(self.prog, &r.ty))));
                };
                let body_want = match name {
                    "all" | "any" | "filter" => Some(T::Bool),
                    "map" => match want {
                        Some(T::List(e)) => Some((**e).clone()),
                        _ => None,
                    },
                    _ => None,
                };
                let lambda = self.lambda(&f.value, &elem, body_want.as_ref())?;
                let (op, ty) = match name {
                    "all" => (Op::All, T::Bool),
                    "any" => (Op::Any, T::Bool),
                    "filter" => {
                        (Op::Filter, if matches!(r.ty, T::Set(_)) { r.ty.clone() } else { T::List(Box::new(elem)) })
                    }
                    "map" => (Op::Map, T::List(Box::new(lambda.ty.clone()))),
                    _ => (Op::Sort, T::List(Box::new(elem))),
                };
                mk(op, vec![r, lambda], ty)
            }
            _ => Err(self.unsupported(span, format!("the method `.{name}` on {}", type_name(self.prog, &r.ty)))),
        }
    }

    /// A one-parameter lambda over `elem`; its type is its body's type.
    fn lambda(&mut self, e: &ir::Expr, elem: &T, want: Option<&T>) -> R<TExpr> {
        let ExprKind::Lambda(id) = &e.kind else {
            return Err(self.unsupported(e.span, "a collection method whose argument is not a lambda"));
        };
        let l = self.prog.lambda(*id);
        let [param] = l.params.as_slice() else {
            return Err(self.unsupported(l.span, "a lambda with more than one parameter"));
        };
        let LambdaBody::Expr(body) = &l.body else {
            return Err(self.unsupported(l.span, "a lambda with a block body"));
        };
        self.push();
        self.bind(param, elem.clone());
        let body = self.expr(body, want);
        self.pop();
        let body = body?;
        let ty = body.ty.clone();
        Ok(TExpr { kind: K::Lambda { param: param.clone(), body: Box::new(body) }, ty, span: e.span })
    }

    fn compare(&mut self, first: &ir::Expr, links: &[(CmpOp, ir::Expr)], span: Span) -> R<TExpr> {
        if let [(CmpOp::In, right)] = links {
            let left = self.expr(first, None)?;
            let coll = self.expr(right, None)?;
            let elem_ty = match &coll.ty {
                T::Set(e) | T::List(e) => (**e).clone(),
                T::Map(k, _) => (**k).clone(),
                t => return Err(self.unsupported(span, format!("`in` on {}", type_name(self.prog, t)))),
            };
            let elem = self.coerce(left, &elem_ty)?;
            return Ok(TExpr { kind: K::Core(Op::Contains, vec![elem, coll]), ty: T::Bool, span });
        }
        if links.iter().any(|(op, _)| *op == CmpOp::In) {
            return Err(self.unsupported(span, "`in` inside a comparison chain"));
        }
        let sources: Vec<&ir::Expr> = std::iter::once(first).chain(links.iter().map(|(_, e)| e)).collect();
        let mut operands = sources.iter().map(|e| self.expr(e, None)).collect::<R<Vec<_>>>()?;
        // A literal beside a byte is a byte.
        if operands.iter().any(|o| o.ty == T::Byte) {
            for o in &mut operands {
                if let K::Int(n) = o.kind
                    && (0..=255).contains(&n)
                {
                    o.ty = T::Byte;
                }
            }
        }
        // The common type: the first operand that is neither a literal nor refined,
        // else the first that is not a literal; beside a literal a refined type
        // widens to its base rather than the literal being narrowed.
        let is_lit = |o: &TExpr| matches!(o.kind, K::Int(_) | K::Text(_));
        let refined = |o: &TExpr| matches!(o.ty, T::Named(id) if alias_base(self.prog, id).is_some());
        let target = operands
            .iter()
            .find(|o| !is_lit(o) && !refined(o))
            .or_else(|| operands.iter().find(|o| !is_lit(o)))
            .or(operands.first())
            .map(|o| o.ty.clone())
            .unwrap_or(T::Int);
        let target = match &target {
            T::Named(id) if operands.iter().any(is_lit) => self.alias_base(*id).map(|(b, _)| b).unwrap_or(target),
            _ => target,
        };
        let operands = operands.into_iter().map(|o| self.coerce(o, &target)).collect::<R<Vec<_>>>()?;
        let mut operands = operands.into_iter();
        let Some(head) = operands.next() else {
            return Err(self.unsupported(span, "an empty comparison"));
        };
        let out = links.iter().map(|(op, _)| *op).zip(operands).collect();
        Ok(TExpr { kind: K::Compare(Box::new(head), out), ty: T::Bool, span })
    }

    fn is(&mut self, value: &ir::Expr, pattern: &Pattern, span: Span) -> R<TExpr> {
        let v = self.expr(value, None)?;
        match (pattern, &v.ty) {
            (Pattern::Call { rule, .. }, T::Event(_)) => {
                Ok(TExpr { kind: K::IsEvent { event: Box::new(v), rule: *rule }, ty: T::Bool, span })
            }
            (Pattern::Variant { ty, tag, args }, T::Reply)
                if *ty == self.prog.result_type
                    && *tag == spec_lang_ir::ERR_TAG
                    && args.iter().all(|a| matches!(a.pattern, Pattern::Wild)) =>
            {
                Ok(TExpr { kind: K::IsFail(Box::new(v)), ty: T::Bool, span })
            }
            _ => {
                let pat = self.pattern(pattern, &v.ty, span)?;
                let arms = vec![
                    (pat, TExpr { kind: K::Bool(true), ty: T::Bool, span }),
                    (Pat::Wild, TExpr { kind: K::Bool(false), ty: T::Bool, span }),
                ];
                Ok(TExpr { kind: K::Match { scrutinee: Box::new(v), arms }, ty: T::Bool, span })
            }
        }
    }

    fn pattern(&mut self, p: &Pattern, ty: &T, span: Span) -> R<Pat> {
        match p {
            Pattern::Wild => Ok(Pat::Wild),
            Pattern::Bind(name) => {
                self.bind(name, ty.clone());
                Ok(Pat::Bind(name.clone()))
            }
            Pattern::Variant { ty: vt, tag, args } => {
                let field_tys: Vec<T> = if *vt == self.prog.option_type {
                    match ty {
                        T::Option(inner) => vec![(**inner).clone()],
                        _ => return Err(self.unsupported(span, "an Option pattern on a value that is not an Option")),
                    }
                } else {
                    match &self.prog.ty(*vt).def {
                        TypeDef::Sum(vs) => match vs.get(*tag as usize) {
                            Some(v) => v.fields.iter().map(|f| self.ty(&f.ty, f.span)).collect::<R<_>>()?,
                            None => Vec::new(),
                        },
                        _ => Vec::new(),
                    }
                };
                let mut out = Vec::new();
                for (a, t) in args.iter().zip(field_tys.iter()) {
                    out.push(self.pattern(&a.pattern, t, span)?);
                }
                Ok(Pat::Variant { ty: *vt, tag: *tag, args: out })
            }
            _ => Err(self.unsupported(span, "this pattern")),
        }
    }

    // ---- statements ----

    pub fn block(&mut self, b: &ir::Block) -> R<Vec<TStmt>> {
        self.push();
        let out = b.stmts.iter().map(|s| self.stmt(s)).collect::<R<Vec<_>>>();
        self.pop();
        out
    }

    /// A body written as `= expr` or `= { ... }`.
    pub fn body(&mut self, e: &ir::Expr) -> R<Vec<TStmt>> {
        match &e.kind {
            ExprKind::Block(b) => self.block(b),
            _ => {
                let result = self.result.clone();
                let x = self.expr(e, Some(&result))?;
                Ok(vec![TStmt { kind: S::Return(Some(x)), span: e.span }])
            }
        }
    }

    fn stmt(&mut self, s: &ir::Stmt) -> R<TStmt> {
        let span = s.span;
        let kind = match &s.kind {
            StmtKind::Let { name, mutable: false, ty, value } => {
                let want = match ty {
                    Some(t) => Some(self.ty(t, span)?),
                    None => None,
                };
                let value = self.expr(value, want.as_ref())?;
                self.bind(name, value.ty.clone());
                S::Let { name: name.clone(), value }
            }
            StmtKind::Let { .. } => return Err(self.unsupported(span, "`var`")),
            StmtKind::Assign { place, value } => {
                let PlaceRoot::State(state) = place.root else {
                    return Err(self.unsupported(span, "assigning a local"));
                };
                let mut ty = self.ty(&self.prog.state(state).ty, span)?;
                let mut path = Vec::new();
                for step in &place.path {
                    match (step, ty.clone()) {
                        (PlaceStep::Index(k), T::Map(kt, vt)) => {
                            path.push(PathStep::Index(self.expr(k, Some(&kt))?));
                            ty = *vt;
                        }
                        (PlaceStep::Field(f), T::Named(id)) => {
                            let TypeDef::Record(fields) = &self.prog.ty(id).def else {
                                return Err(self.unsupported(span, "a field of a type that is not a record"));
                            };
                            let Some((i, field)) = fields.iter().enumerate().find(|(_, x)| &x.name == f) else {
                                return Err(self.unsupported(span, format!("no field `{f}`")));
                            };
                            path.push(PathStep::Field(i));
                            ty = self.ty(&field.ty, field.span)?;
                        }
                        _ => return Err(self.unsupported(span, "this place")),
                    }
                }
                let value = self.expr(value, Some(&ty))?;
                S::Assign { state, path, value }
            }
            StmtKind::Mutate { place, op: MutOp::Remove, args } => {
                let (PlaceRoot::State(state), true, [key]) = (&place.root, place.path.is_empty(), args.as_slice())
                else {
                    return Err(self.unsupported(span, "`remove` on anything but a state map"));
                };
                let T::Map(k, _) = self.ty(&self.prog.state(*state).ty, span)? else {
                    return Err(self.unsupported(span, "`remove` on a state that is not a Map"));
                };
                S::Remove { state: *state, key: self.expr(key, Some(&k))? }
            }
            StmtKind::Mutate { .. } => return Err(self.unsupported(span, "`push` and `insert`")),
            StmtKind::Expr(e) => S::Expr(self.expr(e, None)?),
            StmtKind::Return(v) => {
                let result = self.result.clone();
                S::Return(match v {
                    Some(v) => Some(self.expr(v, Some(&result))?),
                    None => None,
                })
            }
            StmtKind::Assert(e) => S::Assert(self.expr(e, Some(&T::Bool))?),
            StmtKind::For { var, iter, body } => {
                let iter = self.expr(iter, None)?;
                let Some(elem) = iter.ty.elem().cloned() else {
                    return Err(self.unsupported(span, "`for` over a value that is not a List or Set"));
                };
                self.push();
                self.bind(var, elem);
                let body = self.block(body);
                self.pop();
                S::For { var: var.clone(), iter, body: body? }
            }
            StmtKind::While { .. } => return Err(self.unsupported(span, "`while`")),
            StmtKind::If { cond, then, els } => {
                let cond = self.expr(cond, Some(&T::Bool))?;
                let then = self.block(then)?;
                let els = match els {
                    Some(b) => self.block(b)?,
                    None => Vec::new(),
                };
                S::If { cond, then, els }
            }
            StmtKind::Expect { subject, expect } => {
                let call = self.expr(subject, None)?;
                if !matches!(call.kind, K::CallRule { .. }) {
                    return Err(self.unsupported(span, "an expectation on something that is not a rule call"));
                }
                match expect {
                    ir::Expect::Value(v) => {
                        let value = self.expr(v, Some(&call.ty))?;
                        S::ExpectValue { call, value }
                    }
                    ir::Expect::Fail(f) if f.args.is_empty() => S::ExpectFail { call, error: f.name.clone() },
                    ir::Expect::Fail(_) => return Err(self.unsupported(span, "an expected failure with data")),
                }
            }
        };
        Ok(TStmt { kind, span })
    }
}

fn widen(e: TExpr, to: T) -> TExpr {
    let span = e.span;
    TExpr { kind: K::Widen(Box::new(e)), ty: to, span }
}

/// The base of a refined alias (`Title` gives `Text`), with its claim.
pub fn alias_base(prog: &Program, id: TypeId) -> Option<(T, Option<&ir::Claim>)> {
    let TypeDef::Alias(ty) = &prog.ty(id).def else {
        return None;
    };
    let (base, claim) = match ty {
        Ty::Where(b, c) => (b.as_ref(), Some(c.as_ref())),
        t => (t, None),
    };
    let base = match base {
        Ty::Int => T::Int,
        Ty::Text => T::Text,
        Ty::Bool => T::Bool,
        Ty::Named(n) => T::Named(*n),
        _ => return None,
    };
    Some((base, claim))
}

/// `self >= n` or `self > n` as a claim on an `Int`: the least value it admits.
pub fn int_claim_lower_bound(e: &ir::Expr) -> Option<i128> {
    let ExprKind::Compare(a, links) = &e.kind else {
        return None;
    };
    let [(op, b)] = links.as_slice() else {
        return None;
    };
    match (&a.kind, op, &b.kind) {
        (ExprKind::Local(s), CmpOp::Ge, ExprKind::Int(n)) if s == "self" => Some(*n),
        (ExprKind::Local(s), CmpOp::Gt, ExprKind::Int(n)) if s == "self" => Some(n + 1),
        _ => None,
    }
}

/// The least value an `Int` expression can have, from literals and the claims of refined types.
pub fn lower_bound(prog: &Program, e: &TExpr) -> Option<i128> {
    match &e.kind {
        K::Int(n) => Some(*n),
        K::Widen(x) => lower_bound(prog, x),
        K::Bin(Bin::Add, a, b) => Some(lower_bound(prog, a)? + lower_bound(prog, b)?),
        _ => match &e.ty {
            T::Named(id) => alias_base(prog, *id).and_then(|(_, c)| int_claim_lower_bound(&c?.expr)),
            _ => None,
        },
    }
}

/// A type as the spec writes it, for diagnostics.
pub fn type_name(prog: &Program, t: &T) -> String {
    match t {
        T::Unit => "()".into(),
        T::Bool => "Bool".into(),
        T::Int => "Int".into(),
        T::Byte => "Int (a byte)".into(),
        T::Text => "Text".into(),
        T::Named(id) => prog.ty(*id).info.name.clone(),
        T::Option(x) => format!("Option[{}]", type_name(prog, x)),
        T::List(x) => format!("List[{}]", type_name(prog, x)),
        T::Set(x) => format!("Set[{}]", type_name(prog, x)),
        T::Map(k, v) => format!("Map[{}, {}]", type_name(prog, k), type_name(prog, v)),
        T::Tuple(ts) => format!("({})", ts.iter().map(|x| type_name(prog, x)).collect::<Vec<_>>().join(", ")),
        T::Reply => "a reply".into(),
        T::World => "a world".into(),
        T::Event(_) => "an event".into(),
        T::Step(_) => "a step".into(),
        T::Trace(_) => "a trace".into(),
    }
}
