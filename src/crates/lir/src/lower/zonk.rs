//! The last step of a declaration: every unknown replaced by what it was
//! solved to.

use super::Typer;
use crate::node::*;
use crate::ty::LTy;
use spec_lang_syntax::Span;

/// Every type in an expression tree, with the span of the node that holds it.
pub(crate) fn each_ty(e: &mut Expr, f: &mut dyn FnMut(&mut LTy, Span)) {
    let span = e.span;
    f(&mut e.ty, span);
    match &mut e.kind {
        ExprKind::Unit
        | ExprKind::Bool(_)
        | ExprKind::Int(_)
        | ExprKind::Text(_)
        | ExprKind::Local(_)
        | ExprKind::Const(_)
        | ExprKind::State(_)
        | ExprKind::Variant { .. }
        | ExprKind::None
        | ExprKind::EffectRead { .. }
        | ExprKind::Hole(_) => {}
        ExprKind::Interp(parts) => {
            for p in parts {
                if let Part::Show(x) = p {
                    each_ty(x, f);
                }
            }
        }
        ExprKind::Some(x) => each_ty(x, f),
        ExprKind::Construct { args, .. } | ExprKind::CallFn { args, .. } | ExprKind::CallRule { args, .. } => {
            for a in args.iter_mut().flatten() {
                each_ty(a, f);
            }
        }
        ExprKind::Effect { args, .. }
        | ExprKind::Prim { args, .. }
        | ExprKind::Fail { args, .. }
        | ExprKind::Tuple(args)
        | ExprKind::List(args)
        | ExprKind::SetOf(args) => {
            for a in args {
                each_ty(a, f);
            }
        }
        ExprKind::Lambda(l) => {
            for (_, t) in l.params.iter_mut().chain(l.captures.iter_mut()) {
                f(t, span);
            }
            f(&mut l.result, span);
            match &mut l.body {
                LambdaBody::Expr(x) => each_ty(x, f),
                LambdaBody::Block(b) => block_tys(b, f),
            }
        }
        ExprKind::CallLambda { callee, args } => {
            each_ty(callee, f);
            for a in args {
                each_ty(a, f);
            }
        }
        ExprKind::Field { recv, .. }
        | ExprKind::VariantField { recv, .. }
        | ExprKind::WorldState { recv, .. }
        | ExprKind::EventArg { recv, .. } => each_ty(recv, f),
        ExprKind::With { recv, changes, .. } => {
            each_ty(recv, f);
            for (_, x) in changes {
                each_ty(x, f);
            }
        }
        ExprKind::Is { value, pat } => {
            each_ty(value, f);
            pat_tys(pat, span, f);
        }
        ExprKind::If { cond, then, els } => {
            each_ty(cond, f);
            each_ty(then, f);
            each_ty(els, f);
        }
        ExprKind::Logic { left, right, .. } => {
            each_ty(left, f);
            each_ty(right, f);
        }
        ExprKind::Compare { first, links } => {
            each_ty(first, f);
            for (_, x) in links {
                each_ty(x, f);
            }
        }
        ExprKind::Else { value, fallback, .. } => {
            each_ty(value, f);
            each_ty(fallback, f);
        }
        ExprKind::Match { scrutinee, arms } => {
            each_ty(scrutinee, f);
            for a in arms {
                pat_tys(&mut a.pat, span, f);
                each_ty(&mut a.value, f);
            }
        }
        ExprKind::Block(b) => block_tys(b, f),
        ExprKind::MapOf(pairs) => {
            for (k, v) in pairs {
                each_ty(k, f);
                each_ty(v, f);
            }
        }
        ExprKind::Range { lo, hi, .. } => {
            each_ty(lo, f);
            each_ty(hi, f);
        }
    }
}

fn pat_tys(p: &mut Pat, span: Span, f: &mut dyn FnMut(&mut LTy, Span)) {
    for s in &mut p.steps {
        match s {
            PatStep::Equals { value, .. } => each_ty(value, f),
            PatStep::Bind { ty, .. } => f(ty, span),
            PatStep::Tag { .. } | PatStep::Event { .. } | PatStep::Never => {}
        }
    }
}

pub(crate) fn block_tys(b: &mut Block, f: &mut dyn FnMut(&mut LTy, Span)) {
    f(&mut b.ty, b.span);
    for s in &mut b.stmts {
        stmt_tys(s, f);
    }
}

fn stmt_tys(s: &mut Stmt, f: &mut dyn FnMut(&mut LTy, Span)) {
    let span = s.span;
    match &mut s.kind {
        StmtKind::Let { ty, value, .. } => {
            f(ty, span);
            each_ty(value, f);
        }
        StmtKind::Assign { place, value } => {
            place_tys(place, span, f);
            each_ty(value, f);
        }
        StmtKind::Mutate { place, args, .. } => {
            place_tys(place, span, f);
            for a in args {
                each_ty(a, f);
            }
        }
        StmtKind::Expr(e) | StmtKind::Assert(e) => each_ty(e, f),
        StmtKind::Return(v) => {
            if let Some(e) = v {
                each_ty(e, f);
            }
        }
        StmtKind::For { ty, iter, body, .. } => {
            f(ty, span);
            each_ty(iter, f);
            block_tys(body, f);
        }
        StmtKind::While { cond, body } => {
            each_ty(cond, f);
            block_tys(body, f);
        }
        StmtKind::If { cond, then, els } => {
            each_ty(cond, f);
            block_tys(then, f);
            if let Some(b) = els {
                block_tys(b, f);
            }
        }
        StmtKind::Expect { subject, expect } => {
            each_ty(subject, f);
            match expect {
                Expected::Value(x) => each_ty(x, f),
                Expected::Fail { args, .. } => {
                    for a in args {
                        each_ty(a, f);
                    }
                }
            }
        }
    }
}

fn place_tys(p: &mut Place, span: Span, f: &mut dyn FnMut(&mut LTy, Span)) {
    f(&mut p.ty, span);
    for s in &mut p.path {
        if let PlaceStep::MapKey(x) | PlaceStep::ListIndex(x) = s {
            each_ty(x, f);
        }
    }
}

fn default_unit(t: &LTy) -> LTy {
    match t {
        LTy::Var(_) => LTy::Unit,
        LTy::Option(x) => LTy::option(default_unit(x)),
        LTy::List(x) => LTy::list(default_unit(x)),
        LTy::Set(x) => LTy::set(default_unit(x)),
        LTy::Map(k, v) => LTy::map(default_unit(k), default_unit(v)),
        LTy::Tuple(ts) => LTy::Tuple(ts.iter().map(default_unit).collect()),
        LTy::Func(ps, r) => LTy::func(ps.iter().map(default_unit).collect(), default_unit(r)),
        other => other.clone(),
    }
}

impl Typer<'_, '_> {
    /// Solve what is solved; an unknown left open is a type nothing in the
    /// program observes (the items of `[]`, `Map()` or `none` never read or
    /// compared with a typed value): it becomes `()`, which changes no result.
    pub(crate) fn finish_expr(&self, e: &mut Expr) {
        each_ty(e, &mut |t, _| *t = default_unit(&self.zonk(t)));
    }

    pub(crate) fn finish_block(&self, b: &mut Block) {
        block_tys(b, &mut |t, _| *t = default_unit(&self.zonk(t)));
    }
}
