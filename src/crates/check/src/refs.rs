//! Which declarations each declaration names, in code or in its signature.

use spec_lang_ir::visit::{named_types, walk_block, walk_expr};
use spec_lang_ir::*;
use std::collections::{BTreeMap, BTreeSet};

/// Per declaration, every declaration it names. Rule-set types named inside
/// a `Trace[...]` (already flattened into rules by the IR) are listed under
/// the key `None`.
pub fn decl_refs(p: &Program) -> BTreeMap<Option<DeclRef>, BTreeSet<DeclRef>> {
    let mut out = BTreeMap::new();
    for m in &p.modules {
        for d in &m.decls {
            let mut set = BTreeSet::new();
            Collect { p, set: &mut set }.decl(*d);
            set.remove(d);
            out.insert(Some(*d), set);
        }
    }
    out.insert(None, p.rule_set_uses.iter().map(|t| DeclRef::Type(*t)).collect());
    out
}

/// Per module, every declaration its declarations name.
pub(crate) fn module_refs(p: &Program) -> Vec<BTreeSet<DeclRef>> {
    let mut out = vec![BTreeSet::new(); p.modules.len()];
    for (from, to) in decl_refs(p) {
        let m = from.map_or(0, |d| p.info(d).module.index());
        out[m].extend(to);
    }
    out
}

struct Collect<'a, 'p> {
    p: &'p Program,
    set: &'a mut BTreeSet<DeclRef>,
}

impl Collect<'_, '_> {
    fn decl(&mut self, d: DeclRef) {
        let p = self.p;
        match d {
            DeclRef::Type(t) => match &p.ty(t).def {
                TypeDef::Alias(a) => self.ty(a),
                TypeDef::Record(fs) => fs.iter().for_each(|f| self.field(f)),
                TypeDef::Sum(vs) => vs.iter().flat_map(|v| &v.fields).for_each(|f| self.field(f)),
                TypeDef::Rules(rs) => self.set.extend(rs.iter().map(|r| DeclRef::Rule(*r))),
            },
            DeclRef::State(s) => {
                self.ty(&p.state(s).ty);
                if let Some(e) = &p.state(s).init {
                    self.expr(e);
                }
            }
            DeclRef::Const(c) => {
                if let Some(t) = &p.constant(c).ty {
                    self.ty(t);
                }
                self.expr(&p.constant(c).value);
            }
            DeclRef::Fn(f) => {
                let f = p.func(f);
                self.params(&f.params);
                f.result.iter().for_each(|t| self.ty(t));
                self.expr(&f.body);
                self.cases(&f.cases);
            }
            DeclRef::Rule(r) => {
                let r = p.rule(r);
                self.params(&r.params);
                r.result.iter().for_each(|t| self.ty(t));
                r.guards.iter().for_each(|g| self.expr(&g.cond));
                self.expr(&r.body);
            }
            DeclRef::Test(t) => {
                let t = p.test(t);
                self.params(&t.params);
                self.cases(&t.cases);
                for g in &t.given {
                    match g {
                        Given::State { state, value, .. } => {
                            self.set.insert(DeclRef::State(*state));
                            self.expr(value);
                        }
                        Given::Test { test, .. } => {
                            self.set.insert(DeclRef::Test(*test));
                        }
                    }
                }
                let set = &mut *self.set;
                walk_block(p, &t.body, &mut |e| one(e, set));
            }
            DeclRef::Law(l) => {
                self.params(&p.law(l).params);
                self.expr(&p.law(l).body);
            }
            DeclRef::App(a) => self.set.extend(p.app(a).named()),
        }
    }

    fn field(&mut self, f: &Field) {
        self.ty(&f.ty);
        if let Some(d) = &f.default {
            self.expr(d);
        }
    }

    fn params(&mut self, ps: &[Param]) {
        for prm in ps {
            self.ty(&prm.ty);
            if let Some(d) = &prm.default {
                self.expr(d);
            }
        }
    }

    fn cases(&mut self, cs: &[Case]) {
        for c in cs {
            c.args.iter().flatten().for_each(|a| self.expr(a));
            if let Expect::Value(e) = &c.expect {
                self.expr(e);
            }
        }
    }

    fn ty(&mut self, t: &Ty) {
        let mut ts = Vec::new();
        named_types(t, &mut ts);
        self.set.extend(ts.into_iter().map(DeclRef::Type));
        match t {
            Ty::Trace(rs) => self.set.extend(rs.iter().map(|r| DeclRef::Rule(*r))),
            Ty::Where(b, c) => {
                self.expr(&c.expr);
                self.ty(b);
            }
            Ty::Option(x) | Ty::List(x) | Ty::Set(x) => self.ty(x),
            Ty::Map(k, v) => {
                self.ty(k);
                self.ty(v);
            }
            Ty::Tuple(xs) => xs.iter().for_each(|x| self.ty(x)),
            _ => {}
        }
    }

    fn expr(&mut self, e: &Expr) {
        let set = &mut *self.set;
        walk_expr(self.p, e, &mut |x| one(x, set));
    }
}

fn one(x: &Expr, set: &mut BTreeSet<DeclRef>) {
    match &x.kind {
        ExprKind::Const(c) => {
            set.insert(DeclRef::Const(*c));
        }
        ExprKind::State(s) => {
            set.insert(DeclRef::State(*s));
        }
        ExprKind::Variant { ty, .. } => {
            set.insert(DeclRef::Type(*ty));
        }
        ExprKind::Call { callee, .. } => match callee {
            Callee::Fn(f) => {
                set.insert(DeclRef::Fn(*f));
            }
            Callee::Rule(r) => {
                set.insert(DeclRef::Rule(*r));
            }
            Callee::Record(t) | Callee::Variant(t, _) => {
                set.insert(DeclRef::Type(*t));
            }
            Callee::Effect(_) => {}
        },
        ExprKind::Is { pattern, .. } => pattern_refs(pattern, set),
        ExprKind::Match { arms, .. } => arms.iter().for_each(|a| pattern_refs(&a.pattern, set)),
        _ => {}
    }
}

fn pattern_refs(pat: &Pattern, set: &mut BTreeSet<DeclRef>) {
    match pat {
        Pattern::Variant { ty, args, .. } => {
            set.insert(DeclRef::Type(*ty));
            args.iter().for_each(|a| pattern_refs(&a.pattern, set));
        }
        Pattern::Call { rule, args } => {
            set.insert(DeclRef::Rule(*rule));
            args.iter().for_each(|a| pattern_refs(&a.pattern, set));
        }
        Pattern::Tuple(ps) => ps.iter().for_each(|x| pattern_refs(x, set)),
        _ => {}
    }
}
