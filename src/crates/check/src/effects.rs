//! Effects inferred from bodies, against what `@uses` declares.
//!
//! Labels: `s.write` for a state written, `s.read` for a state only read. A
//! write covers a read of the same state. World effects are the labels a
//! loaded library's handler adds (`out.write`, `screen.write`): a call of a
//! handler's operation has the effects its `@uses` declares in the library;
//! a `foreign` declaration has the ones its `@uses` states, taken as written
//! (they cannot be inferred). Both pass to whatever calls them.
//! A state of the rule's own module is named bare (`tasks`), another's with
//! its module's last segment (`board.tasks`).

use spec_lang_diag::Diag;
use spec_lang_ir::visit::{walk_expr, walk_stmts_in_expr};
use spec_lang_ir::*;
use std::collections::BTreeSet;

/// What a body does, directly and through what it calls.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Effects {
    pub reads: BTreeSet<StateId>,
    pub writes: BTreeSet<StateId>,
    /// World effects (`out.write`, `screen.write`): from library operations
    /// and `foreign` declarations.
    pub world: BTreeSet<String>,
    pub rules: BTreeSet<RuleId>,
    pub fns: BTreeSet<FnId>,
}

impl Effects {
    fn join(&mut self, o: &Effects) -> bool {
        let before = (self.reads.len(), self.writes.len(), self.world.len());
        self.reads.extend(&o.reads);
        self.writes.extend(&o.writes);
        self.world.extend(o.world.iter().cloned());
        before != (self.reads.len(), self.writes.len(), self.world.len())
    }

    /// The labels, as `@uses` would write them from module `m`.
    pub fn labels(&self, p: &Program, m: ModuleId) -> BTreeSet<String> {
        let name = |s: &StateId| {
            let st = p.state(*s);
            if st.info.module == m {
                st.info.name.clone()
            } else {
                let module = p.module(st.info.module).name.rsplit('.').next().unwrap_or("").to_string();
                format!("{module}.{}", st.info.name)
            }
        };
        let mut out: BTreeSet<String> = self.writes.iter().map(|s| format!("{}.write", name(s))).collect();
        out.extend(self.reads.iter().filter(|s| !self.writes.contains(s)).map(|s| format!("{}.read", name(s))));
        out.extend(self.world.iter().cloned());
        out
    }
}

/// A `foreign` body's effects: its `@uses` world effects, as written.
fn stated(p: &Program, uses: &[String]) -> Effects {
    let labels = p.world_labels();
    let world = uses.iter().filter(|u| labels.contains(*u)).cloned().collect();
    Effects { world, ..Effects::default() }
}

impl Effects {
    /// Add another set and its callees.
    fn join_calls(&mut self, o: Effects) {
        self.join(&o);
        self.rules.extend(o.rules);
        self.fns.extend(o.fns);
    }
}

fn direct(p: &Program, exprs: &[&Expr]) -> Effects {
    let mut e = Effects::default();
    for x in exprs {
        walk_expr(p, x, &mut |x| match &x.kind {
            ExprKind::State(s) => {
                e.reads.insert(*s);
            }
            ExprKind::Call { callee: Callee::Rule(r), .. } => {
                e.rules.insert(*r);
            }
            ExprKind::Call { callee: Callee::Fn(f), .. } => {
                e.fns.insert(*f);
            }
            ExprKind::Call { callee: Callee::Effect(op), .. } => {
                e.world.extend(p.effect_op(*op).into_iter().flat_map(|o| o.uses.iter().cloned()));
            }
            _ => {}
        });
        walk_stmts_in_expr(p, x, &mut |s| {
            if let StmtKind::Assign { place, .. } | StmtKind::Mutate { place, .. } = &s.kind
                && let PlaceRoot::State(st) = place.root
            {
                e.writes.insert(st);
            }
        });
    }
    e
}

/// Effects of every fn and rule, closed over calls (a least fixed point).
pub fn infer(p: &Program) -> (Vec<Effects>, Vec<Effects>) {
    let mut fns: Vec<Effects> = p
        .fns
        .iter()
        .map(|f| match &f.foreign {
            Some(_) => stated(p, &f.uses),
            None => direct(p, &[&f.body]),
        })
        .collect();
    let mut rules: Vec<Effects> = p
        .rules
        .iter()
        .map(|r| {
            let mut xs: Vec<&Expr> = r.guards.iter().map(|g| &g.cond).collect();
            let mut e = match &r.foreign {
                Some(_) => stated(p, &r.uses),
                None => {
                    xs.push(&r.body);
                    Effects::default()
                }
            };
            e.join_calls(direct(p, &xs));
            e
        })
        .collect();
    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..fns.len() {
            let callees: Vec<FnId> = fns[i].fns.iter().copied().collect();
            for c in callees {
                let o = fns[c.index()].clone();
                changed |= fns[i].join(&o);
            }
        }
        for i in 0..rules.len() {
            let fcs: Vec<FnId> = rules[i].fns.iter().copied().collect();
            let rcs: Vec<RuleId> = rules[i].rules.iter().copied().collect();
            for c in fcs {
                let o = fns[c.index()].clone();
                changed |= rules[i].join(&o);
            }
            for c in rcs {
                let o = rules[c.index()].clone();
                changed |= rules[i].join(&o);
            }
        }
    }
    (fns, rules)
}

/// `E-EFFECT-UNDECLARED` for every `pub` rule whose body does more than its
/// `@uses` says, or that has no `@uses`; for every `fn` with `@uses` whose
/// body does more, and every `pub fn` with a world effect and no `@uses`.
/// `E-EFFECT-KIND` for a `fn` that writes the world. `E-UNSUPPORTED` for a
/// `foreign` declaration whose `@uses` names more than world effects.
pub(crate) fn check_uses(p: &Program) -> Vec<Diag> {
    let (fns, rules) = infer(p);
    let mut out = Vec::new();
    for f in &p.fns {
        out.extend(foreign_uses(p, &f.info, f.foreign.is_some(), &f.uses));
    }
    for r in &p.rules {
        out.extend(foreign_uses(p, &r.info, r.foreign.is_some(), &r.uses));
    }
    for (f, eff) in p.fns.iter().zip(&fns) {
        if let Some(w) = eff.world.iter().find(|w| w.ends_with(".write")) {
            out.push(
                Diag::error(
                    "E-EFFECT-KIND",
                    f.info.name_span,
                    format!("a fn cannot have the effect `{w}`; a rule can"),
                )
                .with_decl(p.qualified(&f.info))
                .with_expected("a fn: state and world reads only", w.clone()),
            );
            continue;
        }
        let inferred = eff.labels(p, f.info.module);
        let required: BTreeSet<String> = if !f.uses.is_empty() {
            inferred
        } else if f.info.is_pub {
            eff.world.clone()
        } else {
            BTreeSet::new()
        };
        out.extend(undeclared(p, &f.info, "fn", &f.uses, &required));
    }
    for (r, eff) in p.rules.iter().zip(&rules) {
        if r.info.is_pub {
            out.extend(undeclared(p, &r.info, "rule", &r.uses, &eff.labels(p, r.info.module)));
        }
    }
    out
}

/// A `foreign` declaration states its effects; only world effects can be stated.
fn foreign_uses(p: &Program, info: &DeclInfo, foreign: bool, uses: &[String]) -> Option<Diag> {
    let labels = p.world_labels();
    let other = uses.iter().find(|u| !labels.contains(*u))?;
    if !foreign {
        return None;
    }
    Some(
        Diag::error(
            "E-UNSUPPORTED",
            info.name_span,
            format!("a foreign declaration with the effect `{other}`: only world effects are known"),
        )
        .with_decl(p.qualified(info))
        .with_expected(labels.into_iter().collect::<Vec<_>>().join(", "), other.clone()),
    )
}

/// The labels of `inferred` that `uses` does not cover, as one diagnostic.
fn undeclared(p: &Program, info: &DeclInfo, kind: &str, uses: &[String], inferred: &BTreeSet<String>) -> Option<Diag> {
    let declared: BTreeSet<String> = uses.iter().cloned().collect();
    let covered = |l: &String| {
        declared.contains(l) || l.strip_suffix(".read").is_some_and(|s| declared.contains(&format!("{s}.write")))
    };
    let missing: Vec<&String> = inferred.iter().filter(|l| !covered(l)).collect();
    if missing.is_empty() {
        return None;
    }
    let all: Vec<String> = declared
        .iter()
        .cloned()
        .chain(missing.iter().map(|s| s.to_string()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let msg = if uses.is_empty() {
        format!("a pub {kind} writes its effects in `@uses`")
    } else {
        format!(
            "the body has effects `@uses` does not list: {}",
            missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        )
    };
    Some(
        Diag::error("E-EFFECT-UNDECLARED", info.name_span, msg)
            .with_decl(p.qualified(info))
            .with_expected(format!("@uses({})", all.join(", ")), format!("@uses({})", uses.join(", ")))
            .with_note(format!("add `@uses({})`", all.join(", "))),
    )
}
