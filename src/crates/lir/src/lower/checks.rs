//! Tests, laws, and the list of checks in the order `spec-lang test` runs them.

use super::{Lowerer, R, Typer, decls, id32};
use crate::node::Expected;
use crate::program::*;
use crate::ty::LTy;
use spec_lang_ir as ir;
use spec_lang_ir::{Callee, ExprKind, LawId, ModuleId, RuleId, TestId};

pub(super) fn lower_test(low: &mut Lowerer<'_>, id: TestId) -> R<Test> {
    let prog = low.prog;
    let d = prog.test(id);
    let info = low.info(&d.info);
    let mut given = Vec::new();
    for g in &d.given {
        given.push(match g {
            ir::Given::State { state, value, .. } => {
                let want = low.erase(&prog.state(*state).ty);
                let mut t = Typer::new(low, info.qualified.clone());
                let mut v = t.expr_as(value, &want)?;
                t.finish_expr(&mut v);
                Given::State { state: *state, value: v }
            }
            ir::Given::Test { test, .. } => Given::Test(*test),
        });
    }
    let mut t = Typer::new(low, info.qualified.clone());
    let params = decls::params(&mut t, &d.params)?;
    let mut body = t.value_block(&d.body)?;
    t.finish_block(&mut body);
    Ok(Test { info, params, given, body })
}

pub(super) fn lower_law(low: &mut Lowerer<'_>, id: LawId) -> R<Law> {
    let prog = low.prog;
    let d = prog.law(id);
    let info = low.info(&d.info);
    let Some((trace, traced)) = d.trace() else {
        return Err(spec_lang_diag::Diag::error("E-TYPE-MISMATCH", d.info.span, "a law needs a Trace parameter")
            .with_decl(info.qualified));
    };
    let mut t = Typer::new(low, info.qualified.clone());
    let params = decls::params(&mut t, &d.params)?;
    let mut body = t.expr_as(&d.body, &LTy::Bool)?;
    t.finish_expr(&mut body);
    let start_rules = start_rules(prog, traced);
    Ok(Law { info, params, trace, traced: traced.to_vec(), start_rules, body })
}

/// Every rule of the modules that own the traced rules, except one that can
/// reach a foreign body.
fn start_rules(prog: &ir::Program, traced: &[RuleId]) -> Vec<RuleId> {
    let modules: Vec<ModuleId> = traced.iter().map(|r| prog.rule(*r).info.module).collect();
    (0..prog.rules.len())
        .map(|i| RuleId(id32(i)))
        .filter(|r| modules.contains(&prog.rule(*r).info.module) && !reaches_foreign(prog, *r))
        .collect()
}

/// Whether a call of `r` can reach a foreign `fn` or rule through its guards and body.
pub(crate) fn reaches_foreign(prog: &ir::Program, r: RuleId) -> bool {
    let mut seen = vec![Callee::Rule(r)];
    let mut todo = vec![Callee::Rule(r)];
    while let Some(c) = todo.pop() {
        let exprs: Vec<&ir::Expr> = match c {
            Callee::Fn(f) if prog.func(f).foreign.is_some() => return true,
            Callee::Rule(x) if prog.rule(x).foreign.is_some() => return true,
            Callee::Fn(f) => vec![&prog.func(f).body],
            Callee::Rule(x) => prog.rule(x).guards.iter().map(|g| &g.cond).chain([&prog.rule(x).body]).collect(),
            _ => continue,
        };
        for e in exprs {
            ir::visit::walk_expr(prog, e, &mut |x| {
                if let ExprKind::Call { callee: c @ (Callee::Fn(_) | Callee::Rule(_)), .. } = &x.kind
                    && !seen.contains(c)
                {
                    seen.push(*c);
                    todo.push(*c);
                }
            });
        }
    }
    false
}

/// One `@cases` row.
fn case<'p>(low: &mut Lowerer<'p>, decl: &str, target: CaseTarget, case: &'p ir::Case) -> R<CheckWhat> {
    let prog = low.prog;
    let (ps, result): (&[ir::Param], Option<LTy>) = match target {
        CaseTarget::Fn(f) => (&prog.func(f).params, Some(low.fn_result(f)?)),
        CaseTarget::Test(t) => (&prog.test(t).params, None),
    };
    let mut t = Typer::new(low, decl.to_string());
    let mut args = Vec::new();
    for (a, p) in case.args.iter().zip(ps) {
        args.push(match a {
            Some(x) => {
                let want = t.low.erase(&p.ty);
                let mut v = t.expr_as(x, &want)?;
                t.finish_expr(&mut v);
                Some(v)
            }
            None => None,
        });
    }
    let expect = match (&case.expect, &result) {
        (ir::Expect::Value(x), Some(r)) => {
            let mut v = t.expr_as(x, r)?;
            t.finish_expr(&mut v);
            Expected::Value(v)
        }
        (ir::Expect::Value(x), None) => {
            let mut v = t.expr(x)?;
            t.finish_expr(&mut v);
            Expected::Value(v)
        }
        (ir::Expect::Fail(f), _) => Expected::Fail { name: f.name.clone(), args: Vec::new() },
    };
    Ok(CheckWhat::Case { target, args, expect })
}

/// Every check: modules by name, declarations in order; a `fn`'s rows, a
/// test's rows or the test itself (a property when it has parameters), a law.
pub(super) fn checks(low: &mut Lowerer<'_>) -> R<Vec<Check>> {
    let prog = low.prog;
    let mut modules: Vec<&ir::Module> = prog.modules.iter().skip(1).collect();
    modules.sort_by(|a, b| a.name.cmp(&b.name));
    let mut out = Vec::new();
    for m in modules {
        for d in &m.decls {
            let push = |out: &mut Vec<Check>, name: String, kind, decl: &str, what| {
                out.push(Check { name, module: m.name.clone(), kind, decl: decl.to_string(), what })
            };
            match *d {
                ir::DeclRef::Fn(f) => {
                    let decl = prog.qualified(&prog.func(f).info);
                    for (i, c) in prog.func(f).cases.iter().enumerate() {
                        let what = case(low, &decl, CaseTarget::Fn(f), c)?;
                        push(&mut out, format!("{decl} case[{i}]"), CheckKind::Case, &decl, what);
                    }
                }
                ir::DeclRef::Test(t) => {
                    let test = prog.test(t);
                    let decl = prog.qualified(&test.info);
                    if !test.cases.is_empty() {
                        for (i, c) in test.cases.iter().enumerate() {
                            let what = case(low, &decl, CaseTarget::Test(t), c)?;
                            push(&mut out, format!("{decl} case[{i}]"), CheckKind::Case, &decl, what);
                        }
                    } else if !test.params.is_empty() {
                        push(&mut out, decl.clone(), CheckKind::Property, &decl, CheckWhat::Property(t));
                    } else {
                        push(&mut out, decl.clone(), CheckKind::Test, &decl, CheckWhat::Test(t));
                    }
                }
                ir::DeclRef::Law(l) => {
                    let decl = prog.qualified(&prog.law(l).info);
                    push(&mut out, decl.clone(), CheckKind::Law, &decl, CheckWhat::Law(l));
                }
                _ => {}
            }
        }
    }
    Ok(out)
}
