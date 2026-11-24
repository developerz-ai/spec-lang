//! Effects inferred from bodies, against what `@uses` declares.
//!
//! Labels: `s.write` for a state written, `s.read` for a state only read,
//! `out.write` for `out.line`. A write covers a read of the same state.
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
    pub out: bool,
    pub rules: BTreeSet<RuleId>,
    pub fns: BTreeSet<FnId>,
}

impl Effects {
    fn join(&mut self, o: &Effects) -> bool {
        let before = (self.reads.len(), self.writes.len(), self.out);
        self.reads.extend(&o.reads);
        self.writes.extend(&o.writes);
        self.out |= o.out;
        before != (self.reads.len(), self.writes.len(), self.out)
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
        if self.out {
            out.insert("out.write".into());
        }
        out
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
            ExprKind::Call { callee: Callee::Effect(Effect::OutLine), .. } => e.out = true,
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
    let mut fns: Vec<Effects> = p.fns.iter().map(|f| direct(p, &[&f.body])).collect();
    let mut rules: Vec<Effects> = p
        .rules
        .iter()
        .map(|r| {
            let mut xs: Vec<&Expr> = r.guards.iter().map(|g| &g.cond).collect();
            xs.push(&r.body);
            direct(p, &xs)
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
/// `@uses` says, or that has no `@uses`.
pub(crate) fn check_uses(p: &Program) -> Vec<Diag> {
    let (_, rules) = infer(p);
    let mut out = Vec::new();
    for (r, eff) in p.rules.iter().zip(&rules) {
        if !r.info.is_pub {
            continue;
        }
        let inferred = eff.labels(p, r.info.module);
        let declared: BTreeSet<String> = r.uses.iter().cloned().collect();
        let covered = |l: &String| {
            declared.contains(l) || l.strip_suffix(".read").is_some_and(|s| declared.contains(&format!("{s}.write")))
        };
        let missing: Vec<&String> = inferred.iter().filter(|l| !covered(l)).collect();
        if missing.is_empty() {
            continue;
        }
        let all: Vec<String> = declared
            .iter()
            .cloned()
            .chain(missing.iter().map(|s| s.to_string()))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let msg = if r.uses.is_empty() {
            "a pub rule writes its effects in `@uses`".to_string()
        } else {
            format!(
                "the body has effects `@uses` does not list: {}",
                missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
            )
        };
        out.push(
            Diag::error("E-EFFECT-UNDECLARED", r.info.name_span, msg)
                .with_decl(p.qualified(&r.info))
                .with_expected(format!("@uses({})", all.join(", ")), format!("@uses({})", r.uses.join(", ")))
                .with_note(format!("add `@uses({})`", all.join(", "))),
        );
    }
    out
}
