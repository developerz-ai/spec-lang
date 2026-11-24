//! Warnings: what is legal but probably not meant.

use crate::effects::infer;
use crate::refs::module_refs;
use spec_lang_diag::Diag;
use spec_lang_ir::visit::{walk_expr, walk_stmts_in_expr};
use spec_lang_ir::*;
use std::collections::BTreeSet;

pub(crate) fn lint(p: &Program) -> Vec<Diag> {
    let mut out = Vec::new();
    let refs = module_refs(p);
    why(p, &mut out);
    unused_uses(p, &refs, &mut out);
    unused_decls(p, &refs, &mut out);
    tests_without_expectations(p, &mut out);
    let (_, effects) = infer(p);
    effects_unused(p, &effects, &mut out);
    inert_laws(p, &effects, &mut out);
    out
}

fn why(p: &Program, out: &mut Vec<Diag>) {
    for r in p.rules.iter().filter(|r| r.info.is_pub && r.info.why.is_none()) {
        out.push(
            Diag::warning(
                "W-LINT-WHY",
                r.info.name_span,
                "a pub rule without `@why`: the reason is what tests and laws are traced to",
            )
            .with_decl(p.qualified(&r.info))
            .with_edit(
                spec_lang_syntax::Span::new(r.info.span.file, r.info.span.lo, r.info.span.lo),
                "@why(\"...\")\n",
            ),
        );
    }
}

fn decl_of(p: &Program, d: DeclRef) -> &DeclInfo {
    p.info(d)
}

fn unused_uses(p: &Program, refs: &[BTreeSet<DeclRef>], out: &mut Vec<Diag>) {
    for m in p.modules.iter().skip(1) {
        let used = &refs[m.id.index()];
        for u in &m.uses {
            let (is_used, span, name) = match u {
                UseItem::Module { module, span, alias } => {
                    (used.iter().any(|d| decl_of(p, *d).module == *module), *span, alias.clone())
                }
                UseItem::Decl { decl, span, name } => (used.contains(decl), *span, name.clone()),
                UseItem::Vocabulary { name, span } => (vocabulary_used(p, m.id, name), *span, name.clone()),
            };
            if !is_used {
                out.push(
                    Diag::warning("W-LINT-UNUSED-USE", span, format!("`{name}` is used nowhere in {}", m.name))
                        .with_decl(m.name.clone())
                        .with_edit(span, ""),
                );
            }
        }
    }
}

/// `use cli;` is used by a `cli` trigger, `out.line`, or an app's `@bin`/`@store`.
fn vocabulary_used(p: &Program, m: ModuleId, name: &str) -> bool {
    if name != vocab::CLI {
        return true;
    }
    let rules = p.rules.iter().filter(|r| r.info.module == m);
    let mut used = false;
    for r in rules {
        used |= !r.triggers.is_empty();
        walk_expr(p, &r.body, &mut |e| used |= matches!(e.kind, ExprKind::Call { callee: Callee::Effect(_), .. }));
    }
    used || p.apps.iter().any(|a| a.info.module == m && (a.bin.is_some() || a.store.is_some()))
}

fn unused_decls(p: &Program, refs: &[BTreeSet<DeclRef>], out: &mut Vec<Diag>) {
    let all: BTreeSet<DeclRef> = refs.iter().flatten().copied().collect();
    for m in p.modules.iter().skip(1) {
        for d in &m.decls {
            if !matches!(d, DeclRef::Fn(_) | DeclRef::Type(_) | DeclRef::Const(_) | DeclRef::State(_)) {
                continue;
            }
            let info = decl_of(p, *d);
            if info.is_pub || all.contains(d) {
                continue;
            }
            out.push(
                Diag::warning("W-LINT-UNUSED-DECL", info.name_span, format!("`{}` is not used anywhere", info.name))
                    .with_decl(p.qualified(info)),
            );
        }
    }
}

fn tests_without_expectations(p: &Program, out: &mut Vec<Diag>) {
    let parents: BTreeSet<TestId> = p
        .tests
        .iter()
        .flat_map(|t| t.given.iter())
        .filter_map(|g| if let Given::Test { test, .. } = g { Some(*test) } else { None })
        .collect();
    for (i, t) in p.tests.iter().enumerate() {
        let id = TestId(u32::try_from(i).unwrap_or(0));
        if parents.contains(&id) || !t.cases.is_empty() {
            continue;
        }
        let mut expects = false;
        let as_expr = Expr::new(ExprKind::Block(t.body.clone()), t.body.span);
        walk_stmts_in_expr(p, &as_expr, &mut |s| {
            expects |= matches!(s.kind, StmtKind::Assert(_) | StmtKind::Expect { .. })
        });
        if !expects {
            out.push(
                Diag::warning(
                    "W-LINT-NO-EXPECT",
                    t.info.name_span,
                    "a test with no `assert` and no `->` step checks only that nothing fails",
                )
                .with_decl(p.qualified(&t.info)),
            );
        }
    }
}

fn effects_unused(p: &Program, effects: &[crate::effects::Effects], out: &mut Vec<Diag>) {
    for (r, e) in p.rules.iter().zip(effects) {
        let inferred = e.labels(p, r.info.module);
        for u in &r.uses {
            let read = u.strip_suffix(".write").map(|s| format!("{s}.read"));
            let only_read = read.as_ref().is_some_and(|x| inferred.contains(x));
            if !inferred.contains(u) {
                let msg = if only_read {
                    format!("`@uses` lists `{u}`, but the body only reads it")
                } else {
                    format!("`@uses` lists `{u}`, but the body never has that effect")
                };
                out.push(Diag::warning("W-LINT-EFFECT-UNUSED", r.info.name_span, msg).with_decl(p.qualified(&r.info)));
            }
        }
    }
}

/// A law that reads states by name, over a rule that writes none of them,
/// checks nothing about that rule.
fn inert_laws(p: &Program, effects: &[crate::effects::Effects], out: &mut Vec<Diag>) {
    for l in &p.laws {
        let Some((_, rules)) = l.trace() else { continue };
        let mut names = BTreeSet::new();
        walk_expr(p, &l.body, &mut |e| {
            if let ExprKind::Prop { name, .. } = &e.kind {
                names.insert(name.clone());
            }
        });
        let reads: BTreeSet<StateId> = p
            .states
            .iter()
            .enumerate()
            .filter(|(_, s)| names.contains(&s.info.name))
            .map(|(i, _)| StateId(u32::try_from(i).unwrap_or(0)))
            .collect();
        if reads.is_empty() {
            continue;
        }
        let inert: Vec<&str> = rules
            .iter()
            .filter(|r| effects[r.index()].writes.is_disjoint(&reads))
            .map(|r| p.rule(*r).info.name.as_str())
            .collect();
        if !inert.is_empty() {
            let read: Vec<&str> = reads.iter().map(|s| p.state(*s).info.name.as_str()).collect();
            out.push(
                Diag::warning(
                    "W-LINT-LAW-INERT",
                    l.info.name_span,
                    format!("{} write(s) none of {}, so this law checks nothing about them", inert.join(", "), read.join(", ")),
                )
                .with_decl(p.qualified(&l.info))
                .with_note("narrow the Trace to the rules that can change what the law reads, or keep it as a guard against future edits"),
            );
        }
    }
}
