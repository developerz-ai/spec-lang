//! IR expressions to the typed tree: a small bidirectional pass.
//!
//! It knows the declared types (parameters, fields, states, results) and the
//! core library's signatures, and pushes an expected type down into literals.
//! Three coercions are made explicit, because a printer must act on them:
//! a base value entering a claimed type ([`TKind::Claim`]), a `Size` used as
//! an `Int` ([`TKind::Widen`]), and an `Int` literal used as a `Size`.
//! Anything it cannot type is an `E-UNSUPPORTED` or `E-TYPE-MISMATCH`
//! diagnostic; nothing is guessed.

use super::tree::*;
use spec_lang_diag::Diag;
use spec_lang_ir as ir;
use spec_lang_ir::{BinOp, CmpOp, Program, StateId, TypeDef, TypeId, UnOp};
use spec_lang_syntax::Span;

pub type R<T> = Result<T, Box<Diag>>;

/// What kind of body is being typed; decides which forms are legal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyKind {
    Fn,
    Rule,
    Test,
    Law,
}

#[derive(Debug)]
pub struct Infer<'p> {
    pub p: &'p Program,
    /// Every state of the app's world, in declaration order.
    pub world: &'p [StateId],
    /// The declaration being typed, dotted, for diagnostics.
    pub decl: String,
    pub kind: BodyKind,
    scope: Vec<(String, Type)>,
}

/// What a declared type is.
#[derive(Debug)]
pub enum Named<'p> {
    /// An alias; `claims` is true when it or its base carries a `where`.
    Alias {
        base: Type,
        claim: Option<&'p ir::Claim>,
    },
    Record(&'p [ir::Field]),
    /// A sum whose variants carry no data.
    Sum(&'p [ir::Variant]),
}

impl<'p> Infer<'p> {
    pub fn new(p: &'p Program, world: &'p [StateId], decl: String, kind: BodyKind) -> Self {
        Infer { p, world, decl, kind, scope: Vec::new() }
    }

    pub fn bind(&mut self, name: &str, ty: Type) {
        self.scope.push((name.to_string(), ty));
    }

    fn unbind(&mut self, depth: usize) {
        self.scope.truncate(depth);
    }

    fn lookup(&self, name: &str) -> Option<&Type> {
        self.scope.iter().rev().find(|(n, _)| n == name).map(|(_, t)| t)
    }

    pub fn unsupported<T>(&self, span: Span, what: impl std::fmt::Display) -> R<T> {
        Err(Box::new(
            Diag::unsupported(span, format!("{what} (emitters)"))
                .with_decl(self.decl.clone())
                .with_note("fix: rewrite with a form the emitters list as supported (src/crates/emit-ts/README.md), or extend the emitter"),
        ))
    }

    fn mismatch<T>(&self, span: Span, expected: &Type, found: &Type) -> R<T> {
        Err(Box::new(
            Diag::error("E-TYPE-MISMATCH", span, "a value is not of the type its place needs")
                .with_decl(self.decl.clone())
                .with_expected(self.show(expected), self.show(found))
                .with_note("fix: change the value, or the declared type of the place it flows into"),
        ))
    }

    /// A type in spec-lang terms.
    pub fn show(&self, t: &Type) -> String {
        match t {
            Type::Unit => "()".into(),
            Type::Bool => "Bool".into(),
            Type::Int | Type::Size => "Int".into(),
            Type::Text => "Text".into(),
            Type::Named(id) => self.p.ty(*id).info.name.clone(),
            Type::Option(t) => format!("Option[{}]", self.show(t)),
            Type::List(t) => format!("List[{}]", self.show(t)),
            Type::Set(t) => format!("Set[{}]", self.show(t)),
            Type::Map(k, v) => format!("Map[{}, {}]", self.show(k), self.show(v)),
            Type::Tuple(ts) => format!("({})", ts.iter().map(|t| self.show(t)).collect::<Vec<_>>().join(", ")),
            Type::World => "World".into(),
            Type::Trace(_) => "Trace".into(),
            Type::Step(_) => "Step".into(),
            Type::Event(_) => "Event".into(),
            Type::Reply => "Reply".into(),
        }
    }

    /// A declared type as a [`Type`].
    pub fn ty(&self, t: &ir::Ty, span: Span) -> R<Type> {
        Ok(match t {
            ir::Ty::Unit => Type::Unit,
            ir::Ty::Bool => Type::Bool,
            ir::Ty::Int => Type::Int,
            ir::Ty::Text => Type::Text,
            ir::Ty::Named(id) => match &self.p.ty(*id).def {
                TypeDef::Rules(_) => return self.unsupported(span, "a rule set used as a value type"),
                _ => Type::Named(*id),
            },
            ir::Ty::Option(t) => Type::Option(Box::new(self.ty(t, span)?)),
            ir::Ty::List(t) => Type::List(Box::new(self.ty(t, span)?)),
            ir::Ty::Set(t) => Type::Set(Box::new(self.ty(t, span)?)),
            ir::Ty::Map(k, v) => Type::Map(Box::new(self.ty(k, span)?), Box::new(self.ty(v, span)?)),
            ir::Ty::Tuple(ts) => Type::Tuple(ts.iter().map(|t| self.ty(t, span)).collect::<R<_>>()?),
            ir::Ty::Range { .. } => return self.unsupported(span, "a range type"),
            ir::Ty::Where(..) => return self.unsupported(span, "a `where` claim outside a `type` declaration"),
            ir::Ty::Trace(rules) => Type::Trace(rules.clone()),
        })
    }

    pub fn named(&self, id: TypeId) -> R<Named<'p>> {
        let decl = self.p.ty(id);
        match &decl.def {
            TypeDef::Alias(ir::Ty::Where(base, claim)) => {
                Ok(Named::Alias { base: self.ty(base, decl.info.span)?, claim: Some(claim.as_ref()) })
            }
            TypeDef::Alias(t) => Ok(Named::Alias { base: self.ty(t, decl.info.span)?, claim: None }),
            TypeDef::Record(fields) => Ok(Named::Record(fields)),
            TypeDef::Sum(variants) => {
                if variants.iter().any(|v| !v.fields.is_empty()) {
                    return self.unsupported(decl.info.span, "a sum type whose variants carry data");
                }
                Ok(Named::Sum(variants))
            }
            TypeDef::Rules(_) => self.unsupported(decl.info.span, "a rule set used as a value type"),
        }
    }

    /// The type under every alias: `Title` gives `Text`.
    pub fn base(&self, t: &Type) -> Type {
        let mut t = t.clone();
        while let Type::Named(id) = &t {
            match self.named(*id) {
                Ok(Named::Alias { base, .. }) => t = base,
                _ => break,
            }
        }
        t
    }

    /// True when a value of `from` must pass a claim to become a `to`.
    fn claim_between(&self, from: &Type, to: &Type) -> bool {
        let mut t = to.clone();
        while &t != from {
            let Type::Named(id) = &t else { return false };
            match self.named(*id) {
                Ok(Named::Alias { base, claim }) => {
                    if claim.is_some() {
                        return true;
                    }
                    t = base;
                }
                _ => return false,
            }
        }
        false
    }

    /// True when `from` is `to` or an alias chain above it.
    fn is_subtype(&self, from: &Type, to: &Type) -> bool {
        let mut t = from.clone();
        loop {
            if &t == to {
                return true;
            }
            let Type::Named(id) = &t else { return false };
            match self.named(*id) {
                Ok(Named::Alias { base, .. }) => t = base,
                _ => return false,
            }
        }
    }

    /// Make `e` a value of `want`, adding the coercion a printer must act on.
    pub fn fit(&self, e: TExpr, want: &Type) -> R<TExpr> {
        if &e.ty == want || self.is_subtype(&e.ty, want) {
            return Ok(e);
        }
        let span = e.span;
        match (want, &e.ty) {
            (Type::Int, Type::Size) => return Ok(TExpr::new(TKind::Widen(Box::new(e)), Type::Int, span)),
            (Type::Size, Type::Int) if matches!(e.kind, TKind::Int(_)) => {
                return Ok(TExpr::new(e.kind, Type::Size, span));
            }
            (Type::Option(inner), _) if matches!(e.kind, TKind::Just(_)) => {
                if let TKind::Just(x) = e.kind {
                    let x = self.fit(*x, inner)?;
                    return Ok(TExpr::new(TKind::Just(Box::new(x)), want.clone(), span));
                }
                return self.unsupported(span, "an option value");
            }
            _ => {}
        }
        let base = self.base(want);
        if matches!(want, Type::Named(_)) && base != *want {
            // A base value entering an alias: through the claim when there is one.
            let found = e.ty.clone();
            let inner = match self.fit(e, &base) {
                Ok(inner) => inner,
                Err(_) => return self.mismatch(span, want, &found),
            };
            if self.claim_between(&base, want) {
                return Ok(TExpr::new(TKind::Claim(Box::new(inner)), want.clone(), span));
            }
            return Ok(TExpr::new(inner.kind, want.clone(), span));
        }
        self.mismatch(span, want, &e.ty)
    }

    fn elem(&self, t: &Type) -> Option<Type> {
        match self.base(t) {
            Type::List(t) | Type::Set(t) => Some(*t),
            _ => None,
        }
    }

    fn state_ty(&self, id: StateId) -> R<Type> {
        let s = self.p.state(id);
        self.ty(&s.ty, s.info.span)
    }

    pub fn expr(&mut self, e: &ir::Expr, want: Option<&Type>) -> R<TExpr> {
        let got = self.expr_inner(e, want)?;
        match want {
            Some(w) => self.fit(got, w),
            None => Ok(got),
        }
    }

    fn args_for(&mut self, params: &[(Type, bool)], args: &[Option<ir::Expr>], span: Span) -> R<Vec<Option<TExpr>>> {
        if params.len() != args.len() {
            return self.unsupported(span, "a call whose arguments do not line up with the parameters");
        }
        let mut out = Vec::new();
        for ((ty, has_default), arg) in params.iter().zip(args) {
            match arg {
                Some(a) => out.push(Some(self.expr(a, Some(ty))?)),
                None if *has_default => out.push(None),
                None => return self.unsupported(span, "a call that leaves out a parameter with no default"),
            }
        }
        Ok(out)
    }

    fn param_types(&self, params: &[ir::Param]) -> R<Vec<(Type, bool)>> {
        params.iter().map(|p| Ok((self.ty(&p.ty, p.span)?, p.default.is_some()))).collect()
    }

    fn expr_inner(&mut self, e: &ir::Expr, want: Option<&Type>) -> R<TExpr> {
        use ir::ExprKind as K;
        let span = e.span;
        let mk = |kind, ty| Ok(TExpr::new(kind, ty, span));
        match &e.kind {
            K::Unit => mk(TKind::Unit, Type::Unit),
            K::Bool(b) => mk(TKind::Bool(*b), Type::Bool),
            K::Int(i) => {
                let small = want.is_some_and(|w| self.base(w) == Type::Size);
                mk(TKind::Int(*i), if small { Type::Size } else { Type::Int })
            }
            K::Text(s) => mk(TKind::Text(s.clone()), Type::Text),
            K::Interp(parts) => {
                let mut out = Vec::new();
                for p in parts {
                    out.push(match p {
                        ir::InterpPart::Lit(s) => TPart::Lit(s.clone()),
                        ir::InterpPart::Expr(x) => {
                            let x = self.expr(x, None)?;
                            if !matches!(self.base(&x.ty), Type::Int | Type::Size | Type::Text) {
                                return self.unsupported(x.span, "interpolating a value that is not a Text or an Int");
                            }
                            TPart::Expr(x)
                        }
                    });
                }
                mk(TKind::Interp(out), Type::Text)
            }
            K::Local(name) => match self.lookup(name) {
                Some(t) => mk(TKind::Local(name.clone()), t.clone()),
                None => self.unsupported(span, format!("the local `{name}`, whose type is not known here")),
            },
            K::Const(id) => {
                let c = self.p.constant(*id);
                match &c.ty {
                    Some(t) => mk(TKind::Const(*id), self.ty(t, c.info.span)?),
                    None => self.unsupported(span, "a const without a written type"),
                }
            }
            K::State(id) => mk(TKind::State(*id), self.state_ty(*id)?),
            K::Variant { ty, tag } if *ty == self.p.option_type => match want.map(|w| self.base(w)) {
                Some(Type::Option(inner)) => mk(TKind::Nothing, Type::Option(inner)),
                _ => self.unsupported(span, "`none` where its type is not known"),
            },
            K::Variant { ty, tag } => {
                self.named(*ty)?;
                mk(TKind::Variant { ty: *ty, tag: *tag }, Type::Named(*ty))
            }
            K::Call { callee, args } => self.call(*callee, args, want, span),
            K::CallValue { .. } => self.unsupported(span, "calling a value"),
            K::Method { recv, name, args } => self.method(recv, name, args, span),
            K::Prop { recv, name } => self.prop(recv, name, span),
            K::Index { recv, key } => {
                let recv = self.expr(recv, None)?;
                match self.base(&recv.ty) {
                    Type::Map(k, v) => {
                        let key = self.expr(key, Some(&k))?;
                        mk(TKind::Index { recv: Box::new(recv), key: Box::new(key) }, *v)
                    }
                    _ => self.unsupported(span, "indexing a value that is not a Map"),
                }
            }
            K::With { recv, fields } => {
                let recv = self.expr(recv, None)?;
                let Type::Named(id) = recv.ty.clone() else {
                    return self.unsupported(span, "`with` on a value that is not a record");
                };
                let Named::Record(decl) = self.named(id)? else {
                    return self.unsupported(span, "`with` on a value that is not a record");
                };
                let mut out = Vec::new();
                for (name, value) in fields {
                    let Some(f) = decl.iter().find(|f| &f.name == name) else {
                        return self.unsupported(value.span, format!("`with` of the unknown field `{name}`"));
                    };
                    let fty = self.ty(&f.ty, f.span)?;
                    out.push((name.clone(), self.expr(value, Some(&fty))?));
                }
                mk(TKind::With { recv: Box::new(recv), fields: out }, Type::Named(id))
            }
            K::Is { value, pattern } => self.is(value, pattern, span),
            K::Unary(op, x) => match op {
                UnOp::Not => {
                    let x = self.expr(x, Some(&Type::Bool))?;
                    mk(TKind::Not(Box::new(x)), Type::Bool)
                }
                UnOp::Neg => {
                    let x = self.expr(x, Some(&Type::Int))?;
                    mk(TKind::Neg(Box::new(x)), Type::Int)
                }
                _ => self.unsupported(span, "this unary operator"),
            },
            K::Binary(op, a, b) => self.binary(*op, a, b, span),
            K::Compare(first, links) => self.compare(first, links, span),
            K::Range { .. } => self.unsupported(span, "a range value"),
            K::Tuple(xs) => {
                let wants: Option<Vec<Type>> = match want.map(|w| self.base(w)) {
                    Some(Type::Tuple(ts)) if ts.len() == xs.len() => Some(ts),
                    _ => None,
                };
                let mut out = Vec::new();
                for (i, x) in xs.iter().enumerate() {
                    out.push(self.expr(x, wants.as_ref().and_then(|w| w.get(i)))?);
                }
                let ty = Type::Tuple(wants.unwrap_or_else(|| out.iter().map(|x| x.ty.clone()).collect()));
                mk(TKind::Tuple(out), ty)
            }
            K::List(xs) => {
                let inner = match want.map(|w| self.base(w)) {
                    Some(Type::List(t)) => Some(*t),
                    _ => None,
                };
                let (out, inner) = self.elements(xs, inner, span)?;
                mk(TKind::List(out), Type::List(Box::new(inner)))
            }
            K::SetOf(xs) => {
                let inner = match want.map(|w| self.base(w)) {
                    Some(Type::Set(t)) => Some(*t),
                    _ => None,
                };
                let (out, inner) = self.elements(xs, inner, span)?;
                mk(TKind::SetOf(out), Type::Set(Box::new(inner)))
            }
            K::MapOf(pairs) => {
                let Some(Type::Map(k, v)) = want.map(|w| self.base(w)) else {
                    return self.unsupported(span, "a Map literal where its type is not known");
                };
                let mut out = Vec::new();
                for (key, value) in pairs {
                    out.push((self.expr(key, Some(&k))?, self.expr(value, Some(&v))?));
                }
                mk(TKind::MapOf(out), Type::Map(k, v))
            }
            K::Lambda(_) => self.unsupported(span, "a lambda outside a core-library call"),
            K::If { cond, then, els } => {
                let cond = self.expr(cond, Some(&Type::Bool))?;
                let then = self.expr(then, want)?;
                let ty = then.ty.clone();
                let els = self.expr(els, Some(&ty))?;
                mk(TKind::If { cond: Box::new(cond), then: Box::new(then), els: Box::new(els) }, ty)
            }
            K::Else { .. } => self.unsupported(span, "`else` on a value"),
            K::Fail(_) => self.unsupported(span, "`fail` as a value"),
            K::Match { scrutinee, arms } => self.match_(scrutinee, arms, want, span),
            K::Block(b) => {
                let ret = want.cloned().unwrap_or(Type::Unit);
                let depth = self.scope.len();
                let stmts = self.block(b, &ret)?;
                self.unbind(depth);
                mk(TKind::Block(stmts), ret)
            }
            K::OutText => mk(TKind::OutText, Type::Text),
            K::Hole(name) => self.unsupported(span, format!("the hole `?{name}`: a build needs every body")),
        }
    }

    fn elements(&mut self, xs: &[ir::Expr], inner: Option<Type>, span: Span) -> R<(Vec<TExpr>, Type)> {
        let mut out = Vec::new();
        let mut inner = inner;
        for x in xs {
            let x = self.expr(x, inner.as_ref())?;
            inner.get_or_insert_with(|| x.ty.clone());
            out.push(x);
        }
        match inner {
            Some(t) => Ok((out, t)),
            None => self.unsupported(span, "an empty collection where its type is not known"),
        }
    }

    fn call(&mut self, callee: ir::Callee, args: &[Option<ir::Expr>], want: Option<&Type>, span: Span) -> R<TExpr> {
        match callee {
            ir::Callee::Fn(id) => {
                let f = self.p.func(id);
                let params = self.param_types(&f.params)?;
                let args = self.args_for(&params, args, span)?;
                let Some(result) = &f.result else {
                    return self.unsupported(span, "a call of a fn without a written result type");
                };
                let ty = self.ty(result, f.info.span)?;
                Ok(TExpr::new(TKind::CallFn { func: id, args }, ty, span))
            }
            ir::Callee::Rule(id) => {
                if self.kind != BodyKind::Test {
                    return self.unsupported(span, "a rule called outside a test");
                }
                let r = self.p.rule(id);
                let params = self.param_types(&r.params)?;
                let args = self.args_for(&params, args, span)?;
                let ty = match &r.result {
                    Some(t) => self.ty(t, r.info.span)?,
                    None => Type::Unit,
                };
                Ok(TExpr::new(TKind::CallRule { rule: id, args }, ty, span))
            }
            ir::Callee::Record(id) => {
                let Named::Record(fields) = self.named(id)? else {
                    return self.unsupported(span, "constructing a type that is not a record");
                };
                let params: Vec<(Type, bool)> =
                    fields.iter().map(|f| Ok((self.ty(&f.ty, f.span)?, f.default.is_some()))).collect::<R<_>>()?;
                let args = self.args_for(&params, args, span)?;
                Ok(TExpr::new(TKind::Record { ty: id, args }, Type::Named(id), span))
            }
            ir::Callee::Variant(ty, tag) if ty == self.p.option_type && tag == ir::SOME_TAG => {
                let inner = match want.map(|w| self.base(w)) {
                    Some(Type::Option(t)) => Some(*t),
                    _ => None,
                };
                let Some(Some(arg)) = args.first() else { return self.unsupported(span, "`Some` without a value") };
                let x = self.expr(arg, inner.as_ref())?;
                let ty = Type::Option(Box::new(x.ty.clone()));
                Ok(TExpr::new(TKind::Just(Box::new(x)), ty, span))
            }
            ir::Callee::Variant(..) => self.unsupported(span, "a sum variant that carries data"),
            ir::Callee::Effect(_) => self.unsupported(span, "an effect used as a value"),
        }
    }

    fn lambda(&mut self, arg: Option<&ir::Arg>, param: &Type, want: Option<&Type>, span: Span) -> R<TExpr> {
        let Some(arg) = arg else { return self.unsupported(span, "a core-library call without its function argument") };
        let ir::ExprKind::Lambda(id) = &arg.value.kind else {
            return self.unsupported(arg.value.span, "a function argument that is not a lambda");
        };
        let lam = self.p.lambda(*id);
        let [name] = lam.params.as_slice() else {
            return self.unsupported(lam.span, "a lambda with more than one parameter");
        };
        let ir::LambdaBody::Expr(body) = &lam.body else {
            return self.unsupported(lam.span, "a lambda with a block body");
        };
        let depth = self.scope.len();
        self.bind(name, param.clone());
        let body = self.expr(body, want);
        self.unbind(depth);
        let body = body?;
        let ty = body.ty.clone();
        Ok(TExpr::new(
            TKind::Lambda { params: vec![(name.clone(), param.clone())], body: Box::new(body) },
            ty,
            lam.span,
        ))
    }

    fn method(&mut self, recv: &ir::Expr, name: &str, args: &[ir::Arg], span: Span) -> R<TExpr> {
        let recv = self.expr(recv, None)?;
        let base = self.base(&recv.ty);
        let core = |op, recv: TExpr, args, ty| Ok(TExpr::new(TKind::Core { op, recv: Box::new(recv), args }, ty, span));
        match (&base, name) {
            (Type::Text, "trim") if args.is_empty() => core(CoreOp::TextTrim, recv, vec![], Type::Text),
            (Type::Text, "contains" | "starts_with") => {
                let Some(part) = args.first() else {
                    return self.unsupported(span, format!("`{name}` without a text"));
                };
                let part = self.expr(&part.value, Some(&Type::Text))?;
                let op = if name == "contains" { CoreOp::TextContains } else { CoreOp::TextStartsWith };
                core(op, recv, vec![part], Type::Bool)
            }
            (Type::List(t) | Type::Set(t), "filter" | "all" | "any") => {
                let f = self.lambda(args.first(), t, Some(&Type::Bool), span)?;
                match name {
                    "filter" => core(CoreOp::Filter, recv, vec![f], Type::List(t.clone())),
                    "all" => core(CoreOp::All, recv, vec![f], Type::Bool),
                    _ => core(CoreOp::Any, recv, vec![f], Type::Bool),
                }
            }
            (Type::List(t) | Type::Set(t), "map") => {
                let f = self.lambda(args.first(), t, None, span)?;
                let out = f.ty.clone();
                core(CoreOp::Map, recv, vec![f], Type::List(Box::new(out)))
            }
            (Type::List(t) | Type::Set(t), "sort") => {
                let f = self.lambda(args.first(), t, None, span)?;
                if !self.is_ordered(&f.ty) {
                    return self.unsupported(span, "a sort key that is not an Int, a Text or a tuple of them");
                }
                core(CoreOp::Sort, recv, vec![f], Type::List(t.clone()))
            }
            (Type::List(t), "join") if self.base(t) == Type::Text => {
                let Some(sep) = args.first() else { return self.unsupported(span, "`join` without a separator") };
                let sep = self.expr(&sep.value, Some(&Type::Text))?;
                core(CoreOp::Join, recv, vec![sep], Type::Text)
            }
            (Type::Map(k, v), "get") => {
                let Some(key) = args.first() else { return self.unsupported(span, "`get` without a key") };
                let key = self.expr(&key.value, Some(k))?;
                core(CoreOp::Get, recv, vec![key], Type::Option(v.clone()))
            }
            _ => self.unsupported(span, format!("the core-library call `{name}` on a {}", self.show(&recv.ty))),
        }
    }

    fn is_ordered(&self, t: &Type) -> bool {
        match self.base(t) {
            Type::Int | Type::Size | Type::Text => true,
            Type::Tuple(ts) => ts.iter().all(|t| self.is_ordered(t)),
            _ => false,
        }
    }

    fn prop(&mut self, recv: &ir::Expr, name: &str, span: Span) -> R<TExpr> {
        let recv = self.expr(recv, None)?;
        let base = self.base(&recv.ty);
        let core =
            |op, recv: TExpr, ty| Ok(TExpr::new(TKind::Core { op, recv: Box::new(recv), args: vec![] }, ty, span));
        let trace = |prop, recv: TExpr, ty| Ok(TExpr::new(TKind::Trace { recv: Box::new(recv), prop }, ty, span));
        match (&base, name) {
            (Type::Text, "size") => core(CoreOp::TextSize, recv, Type::Size),
            (Type::Text, "bytes") => core(CoreOp::TextBytes, recv, Type::List(Box::new(Type::Size))),
            (Type::List(_) | Type::Set(_) | Type::Map(..), "size") => core(CoreOp::Size, recv, Type::Size),
            (Type::Set(t), "list") => core(CoreOp::ToList, recv, Type::List(t.clone())),
            (Type::Map(k, _), "keys") => core(CoreOp::Keys, recv, Type::List(k.clone())),
            (Type::Map(_, v), "values") => core(CoreOp::Values, recv, Type::List(v.clone())),
            (Type::Named(id), _) => {
                let Named::Record(fields) = self.named(*id)? else {
                    return self.unsupported(span, format!("`.{name}` on a {}", self.show(&recv.ty)));
                };
                let Some(f) = fields.iter().find(|f| f.name == name) else {
                    return self.unsupported(span, format!("the unknown field `{name}`"));
                };
                let ty = self.ty(&f.ty, f.span)?;
                Ok(TExpr::new(TKind::Field { recv: Box::new(recv), name: name.to_string() }, ty, span))
            }
            (Type::Trace(_), "states") => trace(TraceProp::States, recv, Type::List(Box::new(Type::World))),
            (Type::Trace(rules), "steps") => {
                let ty = Type::List(Box::new(Type::Step(rules.clone())));
                trace(TraceProp::Steps, recv, ty)
            }
            (Type::Step(_), "before") => trace(TraceProp::Before, recv, Type::World),
            (Type::Step(_), "after") => trace(TraceProp::After, recv, Type::World),
            (Type::Step(rules), "event") => {
                let ty = Type::Event(rules.clone());
                trace(TraceProp::Event, recv, ty)
            }
            (Type::Step(_), "reply") => trace(TraceProp::Reply, recv, Type::Reply),
            (Type::World, _) => {
                let Some(id) = self.world.iter().copied().find(|id| self.p.state(*id).info.name == name) else {
                    return self.unsupported(span, format!("`{name}`, which is not a state of this app's world"));
                };
                let ty = self.state_ty(id)?;
                trace(TraceProp::State(id), recv, ty)
            }
            (Type::Event(rules), _) => {
                let mut ty: Option<Type> = None;
                for r in rules {
                    let rule = self.p.rule(*r);
                    let Some(p) = rule.params.iter().find(|p| p.name == name) else {
                        return self.unsupported(
                            span,
                            format!("`event.{name}`: rule `{}` has no such parameter", rule.info.name),
                        );
                    };
                    let t = self.ty(&p.ty, p.span)?;
                    if ty.as_ref().is_some_and(|seen| seen != &t) {
                        return self.unsupported(
                            span,
                            format!("`event.{name}`: the rules of the trace give it different types"),
                        );
                    }
                    ty = Some(t);
                }
                match ty {
                    Some(ty) => trace(TraceProp::Arg(name.to_string()), recv, ty),
                    None => self.unsupported(span, "an event of a trace with no rules"),
                }
            }
            _ => self.unsupported(span, format!("`.{name}` on a {}", self.show(&recv.ty))),
        }
    }

    fn is(&mut self, value: &ir::Expr, pattern: &ir::Pattern, span: Span) -> R<TExpr> {
        let value = self.expr(value, None)?;
        match (self.base(&value.ty), pattern) {
            (Type::Event(_), ir::Pattern::Call { rule, args }) if args.is_empty() => {
                Ok(TExpr::new(TKind::IsRule { event: Box::new(value), rule: *rule }, Type::Bool, span))
            }
            // A reply is a core `Result`: `Err(_)` is a refusal, `Ok(_)` is not one.
            (Type::Reply, ir::Pattern::Variant { ty, tag, args }) if *ty == self.p.result_type => {
                if !args.iter().all(|a| matches!(a.pattern, ir::Pattern::Wild)) {
                    return self.unsupported(span, "a pattern inside `Ok(...)` or `Err(...)` other than `_`");
                }
                let refused = TExpr::new(TKind::IsRefused { reply: Box::new(value), error: None }, Type::Bool, span);
                if *tag == ir::ERR_TAG {
                    Ok(refused)
                } else {
                    Ok(TExpr::new(TKind::Not(Box::new(refused)), Type::Bool, span))
                }
            }
            (Type::Named(id), ir::Pattern::Variant { ty, tag, args }) if id == *ty && args.is_empty() => {
                let variant = TExpr::new(TKind::Variant { ty: *ty, tag: *tag }, Type::Named(*ty), span);
                Ok(TExpr::new(TKind::Compare(CmpOp::Eq, Box::new(value), Box::new(variant)), Type::Bool, span))
            }
            _ => self.unsupported(span, "this `.is(...)` pattern"),
        }
    }

    /// Bring two numbers to one representation.
    fn numbers(&self, a: TExpr, b: TExpr) -> R<(TExpr, TExpr, Type)> {
        let (ta, tb) = (self.base(&a.ty), self.base(&b.ty));
        match (&ta, &tb) {
            (Type::Size, Type::Size) => Ok((a, b, Type::Size)),
            (Type::Size, Type::Int) if matches!(b.kind, TKind::Int(_)) => {
                let b = TExpr::new(b.kind, Type::Size, b.span);
                Ok((a, b, Type::Size))
            }
            (Type::Int, Type::Size) if matches!(a.kind, TKind::Int(_)) => {
                let a = TExpr::new(a.kind, Type::Size, a.span);
                Ok((a, b, Type::Size))
            }
            (Type::Int | Type::Size, Type::Int | Type::Size) => {
                Ok((self.fit(a, &Type::Int)?, self.fit(b, &Type::Int)?, Type::Int))
            }
            _ => {
                let span = b.span;
                self.mismatch(span, &Type::Int, &tb)
            }
        }
    }

    fn binary(&mut self, op: BinOp, a: &ir::Expr, b: &ir::Expr, span: Span) -> R<TExpr> {
        match op {
            BinOp::And | BinOp::Or | BinOp::Implies => {
                let a = self.expr(a, Some(&Type::Bool))?;
                let b = self.expr(b, Some(&Type::Bool))?;
                Ok(TExpr::new(TKind::Binary(op, Box::new(a), Box::new(b)), Type::Bool, span))
            }
            BinOp::Add | BinOp::Sub | BinOp::Mul => {
                let a = self.expr(a, None)?;
                let b = self.expr(b, None)?;
                let (a, b, ty) = self.numbers(a, b)?;
                Ok(TExpr::new(TKind::Binary(op, Box::new(a), Box::new(b)), ty, span))
            }
            _ => self.unsupported(span, "this operator"),
        }
    }

    fn compare(&mut self, first: &ir::Expr, links: &[(CmpOp, ir::Expr)], span: Span) -> R<TExpr> {
        let mut left = self.expr(first, None)?;
        let mut out: Option<TExpr> = None;
        for (op, right) in links {
            let link = if *op == CmpOp::In {
                let coll = self.expr(right, None)?;
                let key = match self.base(&coll.ty) {
                    Type::List(t) | Type::Set(t) => *t,
                    Type::Map(k, _) => *k,
                    _ => return self.unsupported(right.span, "`in` on a value that is not a collection"),
                };
                let item = self.fit(left.clone(), &key)?;
                left = coll.clone();
                TExpr::new(TKind::Core { op: CoreOp::In, recv: Box::new(coll), args: vec![item] }, Type::Bool, span)
            } else {
                let want = self.base(&left.ty);
                let right = self.expr_inner(right, Some(&want))?;
                let numeric = |t: &Type| matches!(t, Type::Int | Type::Size);
                let (a, b) = if numeric(&self.base(&left.ty)) && numeric(&self.base(&right.ty)) {
                    let (a, b, _) = self.numbers(left.clone(), right.clone())?;
                    (a, b)
                } else {
                    if self.base(&left.ty) != self.base(&right.ty) {
                        return self.mismatch(right.span, &left.ty, &right.ty);
                    }
                    if !matches!(op, CmpOp::Eq | CmpOp::Ne) && !self.is_ordered(&left.ty) {
                        return self.unsupported(span, "ordering values that are not Ints, Texts or tuples of them");
                    }
                    (left.clone(), right.clone())
                };
                left = right;
                TExpr::new(TKind::Compare(*op, Box::new(a), Box::new(b)), Type::Bool, span)
            };
            out = Some(match out {
                Some(prev) => TExpr::new(TKind::Binary(BinOp::And, Box::new(prev), Box::new(link)), Type::Bool, span),
                None => link,
            });
        }
        match out {
            Some(e) => Ok(e),
            None => Ok(left),
        }
    }

    fn match_(&mut self, scrutinee: &ir::Expr, arms: &[ir::Arm], want: Option<&Type>, span: Span) -> R<TExpr> {
        let scrutinee = self.expr(scrutinee, None)?;
        let base = self.base(&scrutinee.ty);
        let mut out = Vec::new();
        let mut ty = want.cloned();
        for arm in arms {
            let depth = self.scope.len();
            let pat = match (&base, &arm.pattern) {
                (_, ir::Pattern::Wild) => TPat::Wild,
                (Type::Named(id), ir::Pattern::Variant { ty, tag, args }) if id == ty && args.is_empty() => {
                    TPat::Variant { ty: *ty, tag: *tag }
                }
                (Type::Option(_), ir::Pattern::Variant { ty, tag, args }) if *ty == self.p.option_type => {
                    if *tag == ir::NONE_TAG {
                        TPat::Nothing
                    } else {
                        if !scrutinee.is_trivial() {
                            return self.unsupported(arm.span, "matching `Some(...)` on a value that is not a local");
                        }
                        match args.as_slice() {
                            [ir::PatArg { pattern: ir::Pattern::Bind(name), .. }] => {
                                if let Type::Option(inner) = &base {
                                    self.bind(name, inner.as_ref().clone());
                                }
                                TPat::Just(Some(name.clone()))
                            }
                            [ir::PatArg { pattern: ir::Pattern::Wild, .. }] => TPat::Just(None),
                            _ => return self.unsupported(arm.span, "this pattern inside `Some(...)`"),
                        }
                    }
                }
                _ => return self.unsupported(arm.span, "this match pattern"),
            };
            let value = self.expr(&arm.value, ty.as_ref());
            self.unbind(depth);
            let value = value?;
            ty.get_or_insert_with(|| value.ty.clone());
            out.push((pat, value));
        }
        let Some(ty) = ty else { return self.unsupported(span, "a match with no arms") };
        if !self.exhaustive(&base, &out) {
            return self.unsupported(span, "a match that does not cover every variant");
        }
        Ok(TExpr::new(TKind::Match { scrutinee: Box::new(scrutinee), arms: out }, ty, span))
    }

    fn exhaustive(&self, base: &Type, arms: &[(TPat, TExpr)]) -> bool {
        if arms.iter().any(|(p, _)| *p == TPat::Wild) {
            return true;
        }
        match base {
            Type::Named(id) => match self.named(*id) {
                Ok(Named::Sum(variants)) => (0..variants.len())
                    .all(|i| arms.iter().any(|(p, _)| matches!(p, TPat::Variant { tag, .. } if *tag as usize == i))),
                _ => false,
            },
            Type::Option(_) => {
                arms.iter().any(|(p, _)| *p == TPat::Nothing) && arms.iter().any(|(p, _)| matches!(p, TPat::Just(_)))
            }
            _ => false,
        }
    }

    /// Statements of a block; `ret` is what a `return` must give.
    pub fn block(&mut self, b: &ir::Block, ret: &Type) -> R<Vec<TStmt>> {
        let mut out = Vec::new();
        for s in &b.stmts {
            out.push(TStmt { kind: self.stmt(s, ret)?, span: s.span });
        }
        Ok(out)
    }

    fn scoped(&mut self, b: &ir::Block, ret: &Type) -> R<Vec<TStmt>> {
        let depth = self.scope.len();
        let out = self.block(b, ret);
        self.unbind(depth);
        out
    }

    fn stmt(&mut self, s: &ir::Stmt, ret: &Type) -> R<TStmtKind> {
        use ir::StmtKind as S;
        let span = s.span;
        Ok(match &s.kind {
            S::Let { name, mutable, ty, value } => {
                let want = match ty {
                    Some(t) => Some(self.ty(t, span)?),
                    None => None,
                };
                let value = self.expr(value, want.as_ref())?;
                self.bind(name, value.ty.clone());
                TStmtKind::Let { name: name.clone(), mutable: *mutable, value }
            }
            S::Assign { place, value } => self.assign(place, value)?,
            S::Mutate { place, op, args } => match (&place.root, place.path.as_slice(), op, args.as_slice()) {
                (ir::PlaceRoot::State(id), [], ir::MutOp::Remove, [key]) => {
                    let Type::Map(k, _) = self.base(&self.state_ty(*id)?) else {
                        return self.unsupported(span, "`remove` on a state that is not a Map");
                    };
                    self.need_rule(span)?;
                    TStmtKind::RemoveEntry { state: *id, key: self.expr(key, Some(&k))? }
                }
                _ => return self.unsupported(span, "this change of a place"),
            },
            S::Expr(e) => match &e.kind {
                ir::ExprKind::Call { callee: ir::Callee::Effect(ir::Effect::OutLine), args } => {
                    let Some(Some(text)) = args.first() else {
                        return self.unsupported(span, "`out.line` without a text");
                    };
                    TStmtKind::OutLine(self.expr(text, Some(&Type::Text))?)
                }
                ir::ExprKind::Fail(f) => {
                    if !f.args.is_empty() {
                        return self.unsupported(span, "an error that carries data");
                    }
                    TStmtKind::Fail(f.name.clone())
                }
                _ => TStmtKind::Expr(self.expr(e, None)?),
            },
            S::Return(value) => match value {
                Some(v) => TStmtKind::Return(Some(self.expr(v, Some(ret))?)),
                None => TStmtKind::Return(None),
            },
            S::Assert(e) => TStmtKind::Assert(self.expr(e, Some(&Type::Bool))?),
            S::For { var, iter, body } => {
                let iter = self.expr(iter, None)?;
                let Some(elem) = self.elem(&iter.ty) else {
                    return self.unsupported(iter.span, "a `for` over a value that is not a List or a Set");
                };
                let depth = self.scope.len();
                self.bind(var, elem);
                let body = self.block(body, ret);
                self.unbind(depth);
                TStmtKind::For { var: var.clone(), iter, body: body? }
            }
            S::While { cond, body } => {
                let cond = self.expr(cond, Some(&Type::Bool))?;
                TStmtKind::While { cond, body: self.scoped(body, ret)? }
            }
            S::If { cond, then, els } => {
                let cond = self.expr(cond, Some(&Type::Bool))?;
                let then = self.scoped(then, ret)?;
                let els = match els {
                    Some(b) => Some(self.scoped(b, ret)?),
                    None => None,
                };
                TStmtKind::If { cond, then, els }
            }
            S::Expect { subject, expect } => {
                let call = self.expr(subject, None)?;
                if !matches!(call.kind, TKind::CallRule { .. }) {
                    return self.unsupported(span, "`->` after something that is not a rule call");
                }
                match expect {
                    ir::Expect::Value(v) => {
                        let want = call.ty.clone();
                        TStmtKind::ExpectValue { value: self.expr(v, Some(&want))?, call }
                    }
                    ir::Expect::Fail(f) => TStmtKind::ExpectFail { call, error: f.name.clone() },
                }
            }
        })
    }

    fn need_rule(&self, span: Span) -> R<()> {
        if self.kind == BodyKind::Rule { Ok(()) } else { self.unsupported(span, "a write to state outside a rule") }
    }

    fn assign(&mut self, place: &ir::Place, value: &ir::Expr) -> R<TStmtKind> {
        let span = place.span;
        match (&place.root, place.path.as_slice()) {
            (ir::PlaceRoot::Local(name), []) => {
                let Some(ty) = self.lookup(name).cloned() else {
                    return self.unsupported(span, "an assignment to an unknown local");
                };
                Ok(TStmtKind::SetLocal { name: name.clone(), value: self.expr(value, Some(&ty))? })
            }
            (ir::PlaceRoot::State(id), path) => {
                self.need_rule(span)?;
                let ty = self.state_ty(*id)?;
                match path {
                    [] => Ok(TStmtKind::SetState { state: *id, value: self.expr(value, Some(&ty))? }),
                    [ir::PlaceStep::Index(key), rest @ ..] => {
                        let Type::Map(k, v) = self.base(&ty) else {
                            return self.unsupported(span, "an indexed write to a state that is not a Map");
                        };
                        let key = self.expr(key, Some(&k))?;
                        match rest {
                            [] => Ok(TStmtKind::PutEntry { state: *id, key, value: self.expr(value, Some(&v))? }),
                            [ir::PlaceStep::Field(field)] => {
                                let Type::Named(rec) = v.as_ref() else {
                                    return self.unsupported(span, "a field write into an entry that is not a record");
                                };
                                let Named::Record(fields) = self.named(*rec)? else {
                                    return self.unsupported(span, "a field write into an entry that is not a record");
                                };
                                let Some(f) = fields.iter().find(|f| &f.name == field) else {
                                    return self.unsupported(span, format!("a write to the unknown field `{field}`"));
                                };
                                let fty = self.ty(&f.ty, f.span)?;
                                Ok(TStmtKind::SetEntryField {
                                    state: *id,
                                    key,
                                    field: field.clone(),
                                    value: self.expr(value, Some(&fty))?,
                                })
                            }
                            _ => self.unsupported(span, "a write deeper than one field of a map entry"),
                        }
                    }
                    _ => self.unsupported(span, "a write to a field of a state"),
                }
            }
            _ => self.unsupported(span, "a write into a part of a local"),
        }
    }
}
