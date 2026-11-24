//! Walking expressions: every sub-expression once, lambda bodies included.

use crate::expr::*;
use crate::program::Program;
use crate::ty::Ty;

/// Call `f` on `e` and every expression inside it, pre-order. Lambda bodies
/// are followed through the program's lambda table; types' claims are not.
pub fn walk_expr<'a>(p: &'a Program, e: &'a Expr, f: &mut dyn FnMut(&'a Expr)) {
    f(e);
    let mut go = |x: &'a Expr| walk_expr(p, x, f);
    match &e.kind {
        ExprKind::Unit
        | ExprKind::Bool(_)
        | ExprKind::Int(_)
        | ExprKind::Text(_)
        | ExprKind::Local(_)
        | ExprKind::Const(_)
        | ExprKind::State(_)
        | ExprKind::Variant { .. }
        | ExprKind::EffectRead(_)
        | ExprKind::Hole(_) => {}
        ExprKind::Interp(parts) => parts.iter().for_each(|x| {
            if let InterpPart::Expr(x) = x {
                go(x)
            }
        }),
        ExprKind::Call { args, .. } => args.iter().flatten().for_each(go),
        ExprKind::CallValue { callee, args } => {
            go(callee);
            args.iter().for_each(go);
        }
        ExprKind::Method { recv, args, .. } => {
            go(recv);
            args.iter().for_each(|a| go(&a.value));
        }
        ExprKind::Prop { recv, .. } => go(recv),
        ExprKind::Index { recv, key } => {
            go(recv);
            go(key);
        }
        ExprKind::With { recv, fields } => {
            go(recv);
            fields.iter().for_each(|(_, x)| go(x));
        }
        ExprKind::Is { value, pattern } => {
            go(value);
            walk_pattern(pattern, &mut go);
        }
        ExprKind::Unary(_, x) => go(x),
        ExprKind::Binary(_, l, r) => {
            go(l);
            go(r);
        }
        ExprKind::Compare(first, rest) => {
            go(first);
            rest.iter().for_each(|(_, x)| go(x));
        }
        ExprKind::Range { lo, hi, .. } => {
            go(lo);
            go(hi);
        }
        ExprKind::Tuple(xs) | ExprKind::List(xs) | ExprKind::SetOf(xs) => xs.iter().for_each(go),
        ExprKind::MapOf(pairs) => pairs.iter().for_each(|(k, v)| {
            go(k);
            go(v);
        }),
        ExprKind::Lambda(id) => match &p.lambda(*id).body {
            LambdaBody::Expr(x) => go(x),
            LambdaBody::Block(b) => walk_block(p, b, &mut go),
        },
        ExprKind::If { cond, then, els } => {
            go(cond);
            go(then);
            go(els);
        }
        ExprKind::Else { value, fallback, .. } => {
            go(value);
            go(fallback);
        }
        ExprKind::Fail(fl) => fl.args.iter().for_each(go),
        ExprKind::Match { scrutinee, arms } => {
            go(scrutinee);
            for a in arms {
                walk_pattern(&a.pattern, &mut go);
                go(&a.value);
            }
        }
        ExprKind::Block(b) => walk_block(p, b, &mut go),
    }
}

/// Every expression in a pattern (literals and compared names).
fn walk_pattern<'a>(pat: &'a Pattern, f: &mut dyn FnMut(&'a Expr)) {
    match pat {
        Pattern::Value(e) => f(e),
        Pattern::Variant { args, .. } | Pattern::Call { args, .. } => {
            args.iter().for_each(|a| walk_pattern(&a.pattern, f))
        }
        Pattern::Tuple(ps) => ps.iter().for_each(|x| walk_pattern(x, f)),
        Pattern::Wild | Pattern::Bind(_) => {}
    }
}

/// Every expression in a block's statements, places' indexes included.
pub fn walk_block<'a>(p: &'a Program, b: &'a Block, f: &mut dyn FnMut(&'a Expr)) {
    for s in &b.stmts {
        walk_stmt(p, s, f);
    }
}

pub fn walk_stmt<'a>(p: &'a Program, s: &'a Stmt, f: &mut dyn FnMut(&'a Expr)) {
    let place = |pl: &'a Place, f: &mut dyn FnMut(&'a Expr)| {
        for st in &pl.path {
            if let PlaceStep::Index(k) = st {
                f(k);
            }
        }
    };
    match &s.kind {
        StmtKind::Let { value, .. } => walk_expr(p, value, f),
        StmtKind::Assign { place: pl, value } => {
            place(pl, &mut |x| walk_expr(p, x, f));
            walk_expr(p, value, f);
        }
        StmtKind::Mutate { place: pl, args, .. } => {
            place(pl, &mut |x| walk_expr(p, x, f));
            args.iter().for_each(|x| walk_expr(p, x, f));
        }
        StmtKind::Expr(e) | StmtKind::Assert(e) => walk_expr(p, e, f),
        StmtKind::Return(e) => e.iter().for_each(|x| walk_expr(p, x, f)),
        StmtKind::For { iter, body, .. } => {
            walk_expr(p, iter, f);
            walk_block(p, body, f);
        }
        StmtKind::While { cond, body } => {
            walk_expr(p, cond, f);
            walk_block(p, body, f);
        }
        StmtKind::If { cond, then, els } => {
            walk_expr(p, cond, f);
            walk_block(p, then, f);
            els.iter().for_each(|b| walk_block(p, b, f));
        }
        StmtKind::Expect { subject, expect } => {
            walk_expr(p, subject, f);
            match expect {
                Expect::Value(e) => walk_expr(p, e, f),
                Expect::Fail(fl) => fl.args.iter().for_each(|x| walk_expr(p, x, f)),
            }
        }
    }
}

/// Call `f` on every statement under `e`: statements of blocks used as
/// values, of lambda bodies, and of nested `for`, `while` and `if`.
pub fn walk_stmts_in_expr<'a>(p: &'a Program, e: &'a Expr, f: &mut dyn FnMut(&'a Stmt)) {
    let mut blocks: Vec<&'a Block> = Vec::new();
    walk_expr(p, e, &mut |x| match &x.kind {
        ExprKind::Block(b) => blocks.push(b),
        ExprKind::Lambda(id) => {
            if let LambdaBody::Block(b) = &p.lambda(*id).body {
                blocks.push(b);
            }
        }
        _ => {}
    });
    // `walk_expr` already descends into nested blocks' expressions, so each
    // block is visited once here, without recursing into its child blocks.
    for b in blocks {
        stmts_shallow(b, f);
    }
}

fn stmts_shallow<'a>(b: &'a Block, f: &mut dyn FnMut(&'a Stmt)) {
    for s in &b.stmts {
        f(s);
        match &s.kind {
            StmtKind::For { body, .. } | StmtKind::While { body, .. } => stmts_shallow(body, f),
            StmtKind::If { then, els, .. } => {
                stmts_shallow(then, f);
                if let Some(e) = els {
                    stmts_shallow(e, f);
                }
            }
            _ => {}
        }
    }
}

/// Every named type a type mentions, claims' expressions excluded.
pub fn named_types(t: &Ty, out: &mut Vec<crate::ids::TypeId>) {
    match t {
        Ty::Named(id) => out.push(*id),
        Ty::Option(x) | Ty::List(x) | Ty::Set(x) => named_types(x, out),
        Ty::Map(k, v) => {
            named_types(k, out);
            named_types(v, out);
        }
        Ty::Tuple(xs) => xs.iter().for_each(|x| named_types(x, out)),
        Ty::Where(b, _) => named_types(b, out),
        _ => {}
    }
}
