//! Read-only walks over LIR trees: every expression, in source order.

use crate::node::*;

/// `e` and every expression inside it, lambdas and blocks included.
pub fn exprs<'a>(e: &'a Expr, f: &mut dyn FnMut(&'a Expr)) {
    f(e);
    match &e.kind {
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
                    exprs(x, f);
                }
            }
        }
        ExprKind::Some(x) => exprs(x, f),
        ExprKind::Construct { args, .. } | ExprKind::CallFn { args, .. } | ExprKind::CallRule { args, .. } => {
            for a in args.iter().flatten() {
                exprs(a, f);
            }
        }
        ExprKind::Effect { args, .. }
        | ExprKind::Prim { args, .. }
        | ExprKind::Fail { args, .. }
        | ExprKind::Tuple(args)
        | ExprKind::List(args)
        | ExprKind::SetOf(args) => {
            for a in args {
                exprs(a, f);
            }
        }
        ExprKind::Lambda(l) => match &l.body {
            LambdaBody::Expr(x) => exprs(x, f),
            LambdaBody::Block(b) => block_exprs(b, f),
        },
        ExprKind::CallLambda { callee, args } => {
            exprs(callee, f);
            for a in args {
                exprs(a, f);
            }
        }
        ExprKind::Field { recv, .. }
        | ExprKind::VariantField { recv, .. }
        | ExprKind::WorldState { recv, .. }
        | ExprKind::EventArg { recv, .. } => exprs(recv, f),
        ExprKind::With { recv, changes, .. } => {
            exprs(recv, f);
            for (_, x) in changes {
                exprs(x, f);
            }
        }
        ExprKind::Is { value, pat } => {
            exprs(value, f);
            pat_exprs(pat, f);
        }
        ExprKind::If { cond, then, els } => {
            exprs(cond, f);
            exprs(then, f);
            exprs(els, f);
        }
        ExprKind::Logic { left, right, .. } => {
            exprs(left, f);
            exprs(right, f);
        }
        ExprKind::Compare { first, links } => {
            exprs(first, f);
            for (_, x) in links {
                exprs(x, f);
            }
        }
        ExprKind::Else { value, fallback, .. } => {
            exprs(value, f);
            exprs(fallback, f);
        }
        ExprKind::Match { scrutinee, arms } => {
            exprs(scrutinee, f);
            for a in arms {
                pat_exprs(&a.pat, f);
                exprs(&a.value, f);
            }
        }
        ExprKind::Block(b) => block_exprs(b, f),
        ExprKind::MapOf(pairs) => {
            for (k, v) in pairs {
                exprs(k, f);
                exprs(v, f);
            }
        }
        ExprKind::Range { lo, hi, .. } => {
            exprs(lo, f);
            exprs(hi, f);
        }
    }
}

fn pat_exprs<'a>(p: &'a Pat, f: &mut dyn FnMut(&'a Expr)) {
    for s in &p.steps {
        if let PatStep::Equals { value, .. } = s {
            exprs(value, f);
        }
    }
}

/// Every expression of every statement of a block.
pub fn block_exprs<'a>(b: &'a Block, f: &mut dyn FnMut(&'a Expr)) {
    for s in &b.stmts {
        match &s.kind {
            StmtKind::Let { value, .. } => exprs(value, f),
            StmtKind::Assign { place, value } => {
                place_exprs(place, f);
                exprs(value, f);
            }
            StmtKind::Mutate { place, args, .. } => {
                place_exprs(place, f);
                for a in args {
                    exprs(a, f);
                }
            }
            StmtKind::Expr(e) | StmtKind::Assert(e) => exprs(e, f),
            StmtKind::Return(v) => {
                if let Some(e) = v {
                    exprs(e, f);
                }
            }
            StmtKind::For { iter, body, .. } => {
                exprs(iter, f);
                block_exprs(body, f);
            }
            StmtKind::While { cond, body } => {
                exprs(cond, f);
                block_exprs(body, f);
            }
            StmtKind::If { cond, then, els } => {
                exprs(cond, f);
                block_exprs(then, f);
                if let Some(b) = els {
                    block_exprs(b, f);
                }
            }
            StmtKind::Expect { subject, expect } => {
                exprs(subject, f);
                match expect {
                    Expected::Value(x) => exprs(x, f),
                    Expected::Fail { args, .. } => {
                        for a in args {
                            exprs(a, f);
                        }
                    }
                }
            }
        }
    }
}

fn place_exprs<'a>(p: &'a Place, f: &mut dyn FnMut(&'a Expr)) {
    for s in &p.path {
        if let PlaceStep::MapKey(x) | PlaceStep::ListIndex(x) = s {
            exprs(x, f);
        }
    }
}
