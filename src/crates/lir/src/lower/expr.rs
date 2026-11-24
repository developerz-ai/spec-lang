//! Expressions.

use super::{R, Typer};
use crate::node::*;
use crate::prim::Prim;
use crate::ty::LTy;
use spec_lang_diag::Diag;
use spec_lang_ir as ir;
use spec_lang_ir::{BinOp, Callee, CmpOp, ExprKind as K, InterpPart, LambdaBody as IrBody, UnOp};
use spec_lang_syntax::Span;

impl<'p> Typer<'_, 'p> {
    /// Lower `e` and make its type `want`.
    pub(crate) fn expr_as(&mut self, e: &'p ir::Expr, want: &LTy) -> R<Expr> {
        let x = self.expr(e)?;
        self.unify(&x.ty, want, e.span)?;
        Ok(x)
    }

    pub(crate) fn exprs(&mut self, xs: &'p [ir::Expr]) -> R<Vec<Expr>> {
        xs.iter().map(|x| self.expr(x)).collect()
    }

    pub(crate) fn expr(&mut self, e: &'p ir::Expr) -> R<Expr> {
        let span = e.span;
        let mk = |kind: ExprKind, ty: LTy| Ok(Expr::new(kind, ty, span));
        match &e.kind {
            K::Unit => mk(ExprKind::Unit, LTy::Unit),
            K::Bool(b) => mk(ExprKind::Bool(*b), LTy::Bool),
            K::Int(n) => mk(ExprKind::Int(*n), LTy::Int),
            K::Text(t) => mk(ExprKind::Text(t.clone()), LTy::Text),
            K::Interp(parts) => {
                let mut out = Vec::new();
                for p in parts {
                    out.push(match p {
                        InterpPart::Lit(s) => Part::Lit(s.clone()),
                        InterpPart::Expr(x) => Part::Show(self.expr(x)?),
                    });
                }
                mk(ExprKind::Interp(out), LTy::Text)
            }
            K::Local(n) => match self.local(n) {
                Some(t) => mk(ExprKind::Local(n.clone()), t),
                None => Err(self.unsupported(span, format!("`{n}` has no value here"))),
            },
            K::Const(c) => {
                let t = self.low.const_ty(*c)?;
                mk(ExprKind::Const(*c), t)
            }
            K::State(s) => {
                let t = self.low.erase(&self.low.prog.state(*s).ty);
                mk(ExprKind::State(*s), t)
            }
            K::Variant { ty, tag } => {
                let prog = self.low.prog;
                if *ty == prog.option_type && *tag == ir::NONE_TAG {
                    let inner = self.fresh();
                    mk(ExprKind::None, LTy::option(inner))
                } else if *ty == prog.option_type || *ty == prog.result_type {
                    Err(self.unsupported(span, "`Some`, `Ok` or `Err` without its value"))
                } else {
                    mk(ExprKind::Variant { ty: *ty, tag: *tag }, LTy::Named(*ty))
                }
            }
            K::Call { callee, args } => self.call(*callee, args, span),
            K::CallValue { callee, args } => {
                let f = self.expr(callee)?;
                let args = self.exprs(args)?;
                let ret = self.fresh();
                let want = LTy::func(args.iter().map(|a| a.ty.clone()).collect(), ret.clone());
                self.unify(&f.ty, &want, span)?;
                mk(ExprKind::CallLambda { callee: Box::new(f), args }, ret)
            }
            K::Method { recv, name, args } => self.method(recv, name, args, span),
            K::Prop { recv, name } => self.prop(recv, name, span),
            K::Index { recv, key } => {
                let r = self.expr(recv)?;
                match self.shallow(&r.ty) {
                    LTy::Map(k, v) => {
                        let key = self.expr_as(key, &k)?;
                        Ok(self.prim(Prim::MapIndex, vec![r, key], *v, span))
                    }
                    LTy::List(t) => {
                        let key = self.expr_as(key, &LTy::Int)?;
                        Ok(self.prim(Prim::ListIndex, vec![r, key], *t, span))
                    }
                    other => Err(Diag::error(
                        "E-TYPE-MISMATCH",
                        span,
                        format!("{} has no index", self.low.show_ty(&self.zonk(&other))),
                    )
                    .with_decl(self.decl.clone())
                    .with_note("Text: use `take` and `drop`")),
                }
            }
            K::With { recv, fields } => {
                let r = self.expr(recv)?;
                let LTy::Named(t) = self.shallow(&r.ty) else {
                    return Err(self.unsupported(span, "`.with` on a value that is not a record"));
                };
                let ir::TypeDef::Record(fs) = &self.low.prog.ty(t).def else {
                    return Err(self.unsupported(span, "`.with` on a value that is not a record"));
                };
                let mut changes = Vec::new();
                for (name, x) in fields {
                    let Some(i) = fs.iter().position(|f| &f.name == name) else {
                        return Err(Diag::error(
                            "E-TYPE-ARITY",
                            span,
                            format!("{} has no field `{name}`", self.low.prog.ty(t).info.name),
                        )
                        .with_decl(self.decl.clone()));
                    };
                    let want = self.low.erase(&fs[i].ty);
                    changes.push((super::id32(i), self.expr_as(x, &want)?));
                }
                let ty = r.ty.clone();
                mk(ExprKind::With { recv: Box::new(r), ty: t, changes }, ty)
            }
            K::Is { value, pattern } => {
                let v = self.expr(value)?;
                let mark = self.mark();
                let pat = self.pattern(pattern, &v.ty, span)?;
                self.reset(mark);
                mk(ExprKind::Is { value: Box::new(v), pat }, LTy::Bool)
            }
            K::Unary(op, x) => {
                let v = self.expr(x)?;
                match op {
                    UnOp::Not => {
                        self.unify(&v.ty, &LTy::Bool, x.span)?;
                        Ok(self.prim(Prim::Not, vec![v], LTy::Bool, span))
                    }
                    UnOp::Neg => {
                        self.unify(&v.ty, &LTy::Int, x.span)?;
                        Ok(self.prim(Prim::IntNeg, vec![v], LTy::Int, span))
                    }
                    UnOp::Pos => {
                        self.unify(&v.ty, &LTy::Int, x.span)?;
                        Ok(v)
                    }
                    other => Err(self.unsupported(span, format!("`{other:?}` outside machine altitude"))),
                }
            }
            K::Binary(op, l, r) => self.binary(*op, l, r, span),
            K::Compare(first, rest) => {
                let mut left = self.expr(first)?;
                if let [(op @ (CmpOp::Eq | CmpOp::Ne), x)] = rest.as_slice() {
                    let right = self.expr(x)?;
                    if self.unify(&left.ty, &right.ty, span).is_err() {
                        return Ok(self.unequal_types(left, right, *op == CmpOp::Ne, span));
                    }
                    let p = self.cmp_prim(*op, &left.ty, &right.ty, x.span)?;
                    return mk(ExprKind::Compare { first: Box::new(left), links: vec![(p, right)] }, LTy::Bool);
                }
                let mut links = Vec::new();
                let mut prev = left.ty.clone();
                for (op, x) in rest {
                    let right = self.expr(x)?;
                    let p = self.cmp_prim(*op, &prev, &right.ty, x.span)?;
                    prev = right.ty.clone();
                    links.push((p, right));
                }
                left.span = first.span;
                mk(ExprKind::Compare { first: Box::new(left), links }, LTy::Bool)
            }
            K::Range { lo, hi, inclusive } => {
                let lo = self.expr_as(lo, &LTy::Int)?;
                let hi = self.expr_as(hi, &LTy::Int)?;
                mk(ExprKind::Range { lo: Box::new(lo), hi: Box::new(hi), inclusive: *inclusive }, LTy::Range)
            }
            K::Tuple(xs) => {
                let xs = self.exprs(xs)?;
                let ty = LTy::Tuple(xs.iter().map(|x| x.ty.clone()).collect());
                mk(ExprKind::Tuple(xs), ty)
            }
            K::List(xs) => {
                let item = self.fresh();
                let xs = self.items(xs, &item)?;
                mk(ExprKind::List(xs), LTy::list(item))
            }
            K::SetOf(xs) => {
                let item = self.fresh();
                let xs = self.items(xs, &item)?;
                mk(ExprKind::SetOf(xs), LTy::set(item))
            }
            K::MapOf(pairs) => {
                let (k, v) = (self.fresh(), self.fresh());
                let mut out = Vec::new();
                for (a, b) in pairs {
                    out.push((self.expr_as(a, &k)?, self.expr_as(b, &v)?));
                }
                mk(ExprKind::MapOf(out), LTy::map(k, v))
            }
            K::Lambda(id) => {
                let n = self.low.prog.lambda(*id).params.len();
                let params: Vec<LTy> = (0..n).map(|_| self.fresh()).collect();
                self.lambda(*id, params, span)
            }
            K::If { cond, then, els } => {
                let c = self.expr_as(cond, &LTy::Bool)?;
                let a = self.expr(then)?;
                let b = self.expr(els)?;
                let ty = self.join(&a.ty, &b.ty, span)?;
                mk(ExprKind::If { cond: Box::new(c), then: Box::new(a), els: Box::new(b) }, ty)
            }
            K::Else { value, fallback, unwrap_option } => {
                // `a else b else c` with `a` and `b` both Options: the engine
                // gives `b` itself when `a` is `none`, and the outer `else`
                // unwraps it. That is `a else (b else c)`, which has one type.
                if *unwrap_option && let K::Else { value: a, fallback: b, unwrap_option: true } = &value.kind {
                    let a = self.expr(a)?;
                    let b = self.expr(b)?;
                    if let (LTy::Option(t), LTy::Option(_)) = (self.shallow(&a.ty), self.shallow(&b.ty)) {
                        self.unify(&b.ty, &a.ty, span)?;
                        let c = self.expr(fallback)?;
                        let inner = self.else_of(b, true, c, span)?;
                        let ty = self.join(&t, &inner.ty, span)?;
                        return mk(ExprKind::Else { value: Box::new(a), fallback: Box::new(inner), unwrap: true }, ty);
                    }
                    let inner = self.else_of(a, true, b, value.span)?;
                    let c = self.expr(fallback)?;
                    return self.else_of(inner, true, c, span);
                }
                let v = self.expr(value)?;
                let f = self.expr(fallback)?;
                self.else_of(v, *unwrap_option, f, span)
            }
            K::Fail(f) => {
                let args = self.exprs(&f.args)?;
                mk(ExprKind::Fail { name: f.name.clone(), args }, LTy::Never)
            }
            K::Match { scrutinee, arms } => {
                let s = self.expr(scrutinee)?;
                let mut ty = LTy::Never;
                let mut out = Vec::new();
                for a in arms {
                    let mark = self.mark();
                    let pat = self.pattern(&a.pattern, &s.ty, a.span)?;
                    let v = self.expr(&a.value)?;
                    self.reset(mark);
                    ty = self.join(&v.ty, &ty, a.span)?;
                    out.push(Arm { pat, value: v });
                }
                mk(ExprKind::Match { scrutinee: Box::new(s), arms: out }, ty)
            }
            K::Block(b) => {
                let b = self.value_block(b)?;
                let ty = b.ty.clone();
                mk(ExprKind::Block(b), ty)
            }
            K::EffectRead(x) => {
                let prog = self.low.prog;
                let Some(op) = prog.effect_op(*x) else {
                    return Err(self.unsupported(span, "an operation its handler does not have"));
                };
                let ty = op.result.as_ref().map_or(LTy::Unit, |t| self.low.erase(t));
                mk(ExprKind::EffectRead { handler: x.handler, op: x.op, name: prog.effect_name(*x) }, ty)
            }
            K::Hole(h) => mk(ExprKind::Hole(h.clone()), LTy::Never),
        }
    }

    /// `v else f`; it unwraps when the source allows it and `v` is an Option.
    fn else_of(&mut self, v: Expr, unwrap_option: bool, f: Expr, span: Span) -> R<Expr> {
        let inner = match self.shallow(&v.ty) {
            LTy::Option(t) if unwrap_option => Some(*t),
            _ => None,
        };
        let unwrap = inner.is_some();
        let want = inner.unwrap_or_else(|| v.ty.clone());
        let ty = self.join(&want, &f.ty, span)?;
        Ok(Expr::new(ExprKind::Else { value: Box::new(v), fallback: Box::new(f), unwrap }, ty, span))
    }

    /// `a == b` where the two sides have different types: never equal (the
    /// value order separates them), after both are evaluated in order. A
    /// block: `let $left = a; let $right = b; return false;` (`true` for `!=`).
    fn unequal_types(&self, left: Expr, right: Expr, ne: bool, span: Span) -> Expr {
        let st = |name: &str, value: Expr| Stmt {
            span: value.span,
            kind: StmtKind::Let { name: name.into(), mutable: false, ty: value.ty.clone(), check: None, value },
        };
        let ret = Stmt { kind: StmtKind::Return(Some(Expr::new(ExprKind::Bool(ne), LTy::Bool, span))), span };
        let block = Block { stmts: vec![st("$left", left), st("$right", right), ret], ty: LTy::Bool, span };
        Expr::new(ExprKind::Block(block), LTy::Bool, span)
    }

    fn items(&mut self, xs: &'p [ir::Expr], item: &LTy) -> R<Vec<Expr>> {
        xs.iter().map(|x| self.expr_as(x, item)).collect()
    }

    pub(crate) fn prim(&self, op: Prim, args: Vec<Expr>, ty: LTy, span: Span) -> Expr {
        Expr::new(ExprKind::Prim { op, args }, ty, span)
    }

    /// A lambda with known parameter types.
    pub(crate) fn lambda(&mut self, id: ir::LambdaId, params: Vec<LTy>, span: Span) -> R<Expr> {
        let lam = self.low.prog.lambda(id);
        if lam.params.len() != params.len() {
            return Err(Diag::error(
                "E-TYPE-ARITY",
                span,
                format!("the function takes {} argument(s), given {}", lam.params.len(), params.len()),
            )
            .with_decl(self.decl.clone()));
        }
        let mark = self.mark();
        self.open_lambda();
        for (n, t) in lam.params.iter().zip(&params) {
            self.bind(n, t.clone());
        }
        let body = match &lam.body {
            IrBody::Expr(x) => self.expr(x).map(LambdaBody::Expr),
            IrBody::Block(b) => self.value_block(b).map(LambdaBody::Block),
        };
        let captures = self.close_lambda();
        self.reset(mark);
        let body = body?;
        let result = match &body {
            LambdaBody::Expr(x) => x.ty.clone(),
            LambdaBody::Block(b) => b.ty.clone(),
        };
        let ty = LTy::func(params.clone(), result.clone());
        let params = lam.params.iter().cloned().zip(params).collect();
        Ok(Expr::new(ExprKind::Lambda(Box::new(Lambda { params, captures, result, body })), ty, span))
    }

    /// An argument that must be a function of `params`: a lambda is typed
    /// with them; any other value must have that type.
    pub(crate) fn fn_arg(&mut self, e: &'p ir::Expr, params: Vec<LTy>) -> R<Expr> {
        if let K::Lambda(id) = &e.kind {
            return self.lambda(*id, params, e.span);
        }
        let x = self.expr(e)?;
        let ret = self.fresh();
        self.unify(&x.ty, &LTy::func(params, ret), e.span)?;
        Ok(x)
    }

    /// Arguments in parameter order against declared parameter types.
    fn args(
        &mut self,
        args: &'p [Option<ir::Expr>],
        params: &[(LTy, bool, String)],
        span: Span,
    ) -> R<Vec<Option<Expr>>> {
        let mut out = Vec::new();
        for (a, (ty, has_default, name)) in args.iter().zip(params) {
            out.push(match a {
                Some(x) => Some(self.expr_as(x, ty)?),
                None if *has_default => None,
                None => {
                    return Err(Diag::error("E-TYPE-ARITY", span, format!("missing argument `{name}`"))
                        .with_decl(self.decl.clone()));
                }
            });
        }
        Ok(out)
    }

    fn call(&mut self, callee: Callee, args: &'p [Option<ir::Expr>], span: Span) -> R<Expr> {
        let prog = self.low.prog;
        let params = |ps: &[ir::Param], low: &super::Lowerer<'_>| -> Vec<(LTy, bool, String)> {
            ps.iter().map(|p| (low.erase(&p.ty), p.default.is_some(), p.name.clone())).collect()
        };
        match callee {
            Callee::Fn(f) => {
                let ps = params(&prog.func(f).params, self.low);
                let args = self.args(args, &ps, span)?;
                let ty = self.low.fn_result(f)?;
                Ok(Expr::new(ExprKind::CallFn { func: f, args }, ty, span))
            }
            Callee::Rule(r) => {
                let ps = params(&prog.rule(r).params, self.low);
                let args = self.args(args, &ps, span)?;
                let ty = self.low.rule_result(r)?;
                Ok(Expr::new(ExprKind::CallRule { rule: r, args }, ty, span))
            }
            Callee::Record(t) => {
                let ir::TypeDef::Record(fs) = &prog.ty(t).def else {
                    return Err(self.unsupported(span, "a constructor of a type that is not a record"));
                };
                let ps: Vec<_> =
                    fs.iter().map(|f| (self.low.erase(&f.ty), f.default.is_some(), f.name.clone())).collect();
                let args = self.args(args, &ps, span)?;
                Ok(Expr::new(ExprKind::Construct { ty: t, tag: None, args }, LTy::Named(t), span))
            }
            Callee::Variant(t, tag) if t == prog.option_type => match args {
                [Some(x)] if tag == ir::SOME_TAG => {
                    let v = self.expr(x)?;
                    let ty = LTy::option(v.ty.clone());
                    Ok(Expr::new(ExprKind::Some(Box::new(v)), ty, span))
                }
                _ => Err(self.unsupported(span, "an Option built other than `Some(x)` or `none`")),
            },
            Callee::Variant(t, _) if t == prog.result_type => {
                Err(self.unsupported(span, "building `Ok(..)` or `Err(..)` outside a pattern"))
            }
            Callee::Variant(t, tag) => {
                let ir::TypeDef::Sum(vs) = &prog.ty(t).def else {
                    return Err(self.unsupported(span, "a variant of a type that is not a sum"));
                };
                let Some(v) = vs.get(tag as usize) else {
                    return Err(self.unsupported(span, "no such variant"));
                };
                let ps: Vec<_> =
                    v.fields.iter().map(|f| (self.low.erase(&f.ty), f.default.is_some(), f.name.clone())).collect();
                let args = self.args(args, &ps, span)?;
                Ok(Expr::new(ExprKind::Construct { ty: t, tag: Some(tag), args }, LTy::Named(t), span))
            }
            Callee::Effect(x) => {
                let Some(op) = prog.effect_op(x) else {
                    return Err(self.unsupported(span, "an operation its handler does not have"));
                };
                let ps = op.params.as_deref().unwrap_or(&[]);
                if ps.len() != args.len() {
                    return Err(Diag::error(
                        "E-TYPE-ARITY",
                        span,
                        format!("`{}` takes {} argument(s)", prog.effect_name(x), ps.len()),
                    )
                    .with_decl(self.decl.clone()));
                }
                let mut out = Vec::new();
                for (a, p) in args.iter().zip(ps) {
                    let Some(a) = a else {
                        return Err(Diag::error("E-TYPE-ARITY", span, "an effect takes every argument")
                            .with_decl(self.decl.clone()));
                    };
                    let want = self.low.erase(&p.ty);
                    out.push(self.expr_as(a, &want)?);
                }
                let ty = op.result.as_ref().map_or(LTy::Unit, |t| self.low.erase(t));
                let kind = ExprKind::Effect { handler: x.handler, op: x.op, name: prog.effect_name(x), args: out };
                Ok(Expr::new(kind, ty, span))
            }
        }
    }

    fn binary(&mut self, op: BinOp, l: &'p ir::Expr, r: &'p ir::Expr, span: Span) -> R<Expr> {
        let logic = match op {
            BinOp::And => Some(Logic::And),
            BinOp::Or => Some(Logic::Or),
            BinOp::Implies => Some(Logic::Implies),
            _ => None,
        };
        if let Some(op) = logic {
            let a = self.expr_as(l, &LTy::Bool)?;
            let b = self.expr_as(r, &LTy::Bool)?;
            return Ok(Expr::new(ExprKind::Logic { op, left: Box::new(a), right: Box::new(b) }, LTy::Bool, span));
        }
        let a = self.expr(l)?;
        let b = self.expr(r)?;
        let ty = self.join(&a.ty, &b.ty, span)?;
        let found = self.shallow(&ty);
        let p = match (op, &found) {
            (BinOp::Add, LTy::Int) => Prim::IntAdd,
            (BinOp::Sub, LTy::Int) => Prim::IntSub,
            (BinOp::Mul, LTy::Int) => Prim::IntMul,
            (BinOp::Div, LTy::Int) => Prim::IntDiv,
            (BinOp::Mod, LTy::Int) => Prim::IntMod,
            (BinOp::Pow, LTy::Int) => Prim::IntPow,
            (BinOp::Band, LTy::Int) => Prim::IntBand,
            (BinOp::Bor, LTy::Int) => Prim::IntBor,
            (BinOp::Bxor, LTy::Int) => Prim::IntBxor,
            (BinOp::Shl, LTy::Int) => Prim::IntShl,
            (BinOp::Shr, LTy::Int) => Prim::IntShr,
            (BinOp::Add, LTy::List(_)) => Prim::ListConcat,
            (BinOp::Add, LTy::Set(_)) => Prim::SetUnion,
            (BinOp::Sub, LTy::Set(_)) => Prim::SetDifference,
            (BinOp::Add, LTy::Text) => {
                return Err(Diag::error("E-TYPE-MISMATCH", span, "`+` does not join Text")
                    .with_decl(self.decl.clone())
                    .with_note("write \"{a}{b}\""));
            }
            (_, LTy::Var(_)) => {
                return Err(self.unsupported(span, "an operator on values whose type is not known here"));
            }
            _ => {
                return Err(Diag::error(
                    "E-TYPE-MISMATCH",
                    span,
                    format!("`{op:?}` does not apply to {}", self.low.show_ty(&self.zonk(&found))),
                )
                .with_decl(self.decl.clone()));
            }
        };
        Ok(self.prim(p, vec![a, b], ty, span))
    }

    fn cmp_prim(&mut self, op: CmpOp, left: &LTy, right: &LTy, span: Span) -> R<Prim> {
        match op {
            CmpOp::Eq | CmpOp::Ne => {
                self.unify(left, right, span)?;
                if !self.zonk(left).comparable() {
                    return Err(self.unsupported(span, "`==` on functions"));
                }
                Ok(if op == CmpOp::Eq { Prim::Eq } else { Prim::Ne })
            }
            CmpOp::In => match self.shallow(right) {
                LTy::List(t) => self.unify(left, &t, span).map(|_| Prim::ListContains),
                LTy::Set(t) => self.unify(left, &t, span).map(|_| Prim::SetContains),
                LTy::Map(k, _) => self.unify(left, &k, span).map(|_| Prim::MapHas),
                LTy::Range => self.unify(left, &LTy::Int, span).map(|_| Prim::RangeHas),
                LTy::Text => self.unify(left, &LTy::Text, span).map(|_| Prim::TextContains),
                other => Err(Diag::error(
                    "E-TYPE-MISMATCH",
                    span,
                    format!("`in` needs a collection, found {}", self.low.show_ty(&self.zonk(&other))),
                )
                .with_decl(self.decl.clone())),
            },
            _ => {
                self.unify(left, right, span)?;
                let ints = [Prim::IntLt, Prim::IntLe, Prim::IntGt, Prim::IntGe];
                let texts = [Prim::TextLt, Prim::TextLe, Prim::TextGt, Prim::TextGe];
                let i = match op {
                    CmpOp::Lt => 0,
                    CmpOp::Le => 1,
                    CmpOp::Gt => 2,
                    _ => 3,
                };
                match self.shallow(left) {
                    LTy::Int => Ok(ints[i]),
                    LTy::Text => Ok(texts[i]),
                    other => Err(Diag::error(
                        "E-TYPE-MISMATCH",
                        span,
                        format!("`<` compares numbers or Text, not {}", self.low.show_ty(&self.zonk(&other))),
                    )
                    .with_decl(self.decl.clone())),
                }
            }
        }
    }
}
