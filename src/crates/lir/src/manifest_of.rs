//! Building an app's manifest from the IR's app declaration.

use crate::manifest::{AppParam, Manifest, Named, Note, NoteArg, NoteValue};
use crate::node::{Expr, ExprKind};
use crate::program::{Body, Lir};
use spec_lang_ir as ir;
use spec_lang_ir::{DeclRef, RuleId};

pub(crate) fn note(n: &ir::NoteUse) -> Note {
    Note { library: n.library.clone(), name: n.name.clone(), args: n.args.iter().map(arg).collect() }
}

fn arg(a: &ir::NoteArg) -> NoteArg {
    let value = match &a.value {
        ir::NoteValue::Int(i) => NoteValue::Int(*i),
        ir::NoteValue::Num { value, unit } => NoteValue::Quantity { value: *value, unit: unit.clone() },
        ir::NoteValue::Text(t) => NoteValue::Text(t.clone()),
        ir::NoteValue::Variant { name, args } => {
            NoteValue::Variant { name: name.clone(), args: args.iter().map(arg).collect() }
        }
    };
    NoteArg { param: a.param.clone(), value }
}

pub(crate) fn manifest(prog: &ir::Program, lir: &Lir, a: &ir::AppDecl) -> Manifest {
    let named = |d: &DeclRef| match d {
        DeclRef::Rule(r) => Some(Named::Rule(*r)),
        DeclRef::Fn(f) => Some(Named::Fn(*f)),
        _ => None,
    };
    let mut params = Vec::new();
    if !a.rules.is_empty() {
        params.push(AppParam { name: "rules".into(), decls: a.rules.iter().map(|r| Named::Rule(*r)).collect() });
    }
    for f in &a.fields {
        params.push(AppParam { name: f.name.clone(), decls: named(&f.decl).into_iter().collect() });
    }
    let roots: Vec<Named> = a.named().iter().filter_map(named).collect();
    Manifest {
        name: a.info.name.clone(),
        qualified: prog.qualified(&a.info),
        module: prog.module(a.info.module).name.clone(),
        target: a.target.clone(),
        shape: a.shape.clone(),
        notes: a.notes.iter().map(note).collect(),
        params,
        rules: reachable_rules(lir, &roots),
    }
}

/// Every rule reachable from `roots` through calls, roots included.
fn reachable_rules(lir: &Lir, roots: &[Named]) -> Vec<RuleId> {
    let mut seen: Vec<Named> = roots.to_vec();
    let mut todo: Vec<Named> = roots.to_vec();
    while let Some(n) = todo.pop() {
        let mut found = Vec::new();
        match n {
            Named::Fn(f) => body_calls(&lir.func(f).body, &mut found),
            Named::Rule(r) => {
                let rule = lir.rule(r);
                for g in &rule.guards {
                    calls(&g.cond, &mut found);
                }
                body_calls(&rule.body, &mut found);
            }
        }
        for c in found {
            if !seen.contains(&c) {
                seen.push(c);
                todo.push(c);
            }
        }
    }
    let mut rules: Vec<RuleId> =
        seen.into_iter().filter_map(|n| if let Named::Rule(r) = n { Some(r) } else { None }).collect();
    rules.sort();
    rules
}

fn body_calls(b: &Body, out: &mut Vec<Named>) {
    if let Body::Expr(e) = b {
        calls(e, out);
    }
}

fn calls(e: &Expr, out: &mut Vec<Named>) {
    crate::walk::exprs(e, &mut |x| direct(x, out));
}

fn direct(e: &Expr, out: &mut Vec<Named>) {
    match &e.kind {
        ExprKind::CallFn { func, .. } => out.push(Named::Fn(*func)),
        ExprKind::CallRule { rule, .. } => out.push(Named::Rule(*rule)),
        _ => {}
    }
}
