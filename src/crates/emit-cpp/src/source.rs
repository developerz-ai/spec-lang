//! Where a declaration was written, for the comment above what it became, and
//! which modules a declaration reaches, for includes.

use spec_lang_ir::ModuleId;
use spec_lang_lir::node::*;
use spec_lang_lir::{DTy, Info, LTy, Lir};
use std::collections::BTreeSet;

/// Line numbers of the program's files, read from the modules' paths.
#[derive(Debug)]
pub struct Lines {
    /// Per module: the offset each line starts at, when its file could be read.
    starts: Vec<Option<Vec<usize>>>,
}

impl Lines {
    pub fn read(lir: &Lir) -> Lines {
        let starts = lir
            .modules
            .iter()
            .map(|m| {
                let text = std::fs::read_to_string(&m.path).ok()?;
                let mut at = vec![0];
                at.extend(text.bytes().enumerate().filter(|(_, b)| *b == b'\n').map(|(i, _)| i + 1));
                Some(at)
            })
            .collect();
        Lines { starts }
    }

    /// `todo/board.slg:16`: the module's file as the project names it, and the line.
    pub fn at(&self, lir: &Lir, info: &Info) -> String {
        let file = file_of(lir, info.module);
        match self.starts.get(info.module.index()).and_then(Option::as_ref) {
            Some(starts) => format!("{file}:{}", starts.partition_point(|s| *s <= info.span.lo as usize)),
            None => file,
        }
    }
}

/// `todo.board` is written in `todo/board.slg`.
pub fn file_of(lir: &Lir, module: ModuleId) -> String {
    format!("{}.slg", lir.modules[module.index()].name.replace('.', "/"))
}

/// The modules whose declarations something names.
#[derive(Debug, Default)]
pub struct Reach {
    pub modules: BTreeSet<ModuleId>,
    /// It names the machine or the world.
    pub machine: bool,
}

impl Reach {
    pub fn lty(&mut self, lir: &Lir, t: &LTy) {
        match t {
            LTy::Named(id) => {
                self.modules.insert(lir.ty(*id).info.module);
            }
            LTy::Option(t) | LTy::List(t) | LTy::Set(t) => self.lty(lir, t),
            LTy::Map(k, v) => {
                self.lty(lir, k);
                self.lty(lir, v);
            }
            LTy::Tuple(ts) => ts.iter().for_each(|t| self.lty(lir, t)),
            LTy::Func(ps, r) => {
                ps.iter().for_each(|t| self.lty(lir, t));
                self.lty(lir, r);
            }
            LTy::Step(_) | LTy::World | LTy::Trace(_) => self.machine = true,
            _ => {}
        }
    }

    pub fn dty(&mut self, lir: &Lir, t: &DTy) {
        match t {
            DTy::Named(id) => {
                self.modules.insert(lir.ty(*id).info.module);
            }
            DTy::Option(t) | DTy::List(t) | DTy::Set(t) | DTy::Where(t, _) => self.dty(lir, t),
            DTy::Map(k, v) => {
                self.dty(lir, k);
                self.dty(lir, v);
            }
            DTy::Tuple(ts) => ts.iter().for_each(|t| self.dty(lir, t)),
            DTy::Trace(_) => self.machine = true,
            _ => {}
        }
    }

    pub fn expr(&mut self, lir: &Lir, e: &Expr) {
        spec_lang_lir::walk::exprs(e, &mut |x| {
            self.lty(lir, &x.ty);
            match &x.kind {
                ExprKind::CallFn { func, .. } => {
                    self.modules.insert(lir.func(*func).info.module);
                }
                ExprKind::CallRule { rule, .. } => {
                    self.modules.insert(lir.rule(*rule).info.module);
                    self.machine = true;
                }
                ExprKind::Const(c) => {
                    self.modules.insert(lir.constant(*c).info.module);
                }
                ExprKind::Construct { ty, .. } | ExprKind::Variant { ty, .. } | ExprKind::With { ty, .. } => {
                    self.modules.insert(lir.ty(*ty).info.module);
                }
                ExprKind::State(_) | ExprKind::Effect { .. } | ExprKind::EffectRead { .. } | ExprKind::WorldState { .. } => {
                    self.machine = true
                }
                ExprKind::Lambda(l) => {
                    l.params.iter().for_each(|(_, t)| self.lty(lir, t));
                    self.lty(lir, &l.result);
                }
                ExprKind::Is { pat, .. } => self.pat(lir, pat),
                ExprKind::Match { arms, .. } => arms.iter().for_each(|a| self.pat(lir, &a.pat)),
                _ => {}
            }
        });
    }

    fn pat(&mut self, lir: &Lir, p: &Pat) {
        for s in &p.steps {
            match s {
                PatStep::Tag { ty, .. } => {
                    self.modules.insert(lir.ty(*ty).info.module);
                }
                PatStep::Bind { ty, .. } => self.lty(lir, ty),
                _ => {}
            }
        }
    }

    pub fn params(&mut self, lir: &Lir, params: &[spec_lang_lir::Param]) {
        for p in params {
            self.lty(lir, &p.ty);
            self.dty(lir, &p.check);
        }
    }
}

/// Every `(sum, field name)` some expression reads through `value.field` on a sum.
pub fn variant_fields(lir: &Lir) -> BTreeSet<(u32, String)> {
    let mut out = BTreeSet::new();
    every_expr(lir, &mut |e| {
        spec_lang_lir::walk::exprs(e, &mut |x| {
            if let ExprKind::VariantField { ty, name, .. } = &x.kind {
                out.insert((ty.0, name.clone()));
            }
        })
    });
    out
}

/// Whether some expression of the program uses a primitive.
pub fn uses_prim(lir: &Lir, op: spec_lang_lir::Prim) -> bool {
    let mut found = false;
    every_expr(lir, &mut |e| {
        spec_lang_lir::walk::exprs(e, &mut |x| {
            found |= matches!(&x.kind, ExprKind::Prim { op: o, .. } if *o == op)
                || matches!(&x.kind, ExprKind::Compare { links, .. } if links.iter().any(|(o, _)| *o == op));
        })
    });
    found
}

/// Every root expression of the program, each once.
pub fn every_expr<'a>(lir: &'a Lir, f: &mut dyn FnMut(&'a Expr)) {
    use spec_lang_lir::{Body, CheckWhat, Given, TypeDef};
    let block = |b: &'a Block, f: &mut dyn FnMut(&'a Expr)| {
        for s in &b.stmts {
            stmt_roots(s, f);
        }
    };
    for t in &lir.types {
        match &t.def {
            TypeDef::Record { fields } => fields.iter().filter_map(|x| x.default.as_ref()).for_each(&mut *f),
            TypeDef::Sum { variants } => {
                variants.iter().flat_map(|v| &v.fields).filter_map(|x| x.default.as_ref()).for_each(&mut *f)
            }
            _ => {}
        }
    }
    lir.claims.iter().for_each(|c| f(&c.body));
    lir.consts.iter().for_each(|c| f(&c.value));
    lir.states.iter().filter_map(|s| s.init.as_ref()).for_each(&mut *f);
    for d in &lir.fns {
        d.params.iter().filter_map(|p| p.default.as_ref()).for_each(&mut *f);
        if let Body::Expr(e) = &d.body {
            f(e);
        }
    }
    for d in &lir.rules {
        d.params.iter().filter_map(|p| p.default.as_ref()).for_each(&mut *f);
        d.guards.iter().for_each(|g| f(&g.cond));
        if let Body::Expr(e) = &d.body {
            f(e);
        }
    }
    for t in &lir.tests {
        for g in &t.given {
            if let Given::State { value, .. } = g {
                f(value);
            }
        }
        block(&t.body, f);
    }
    lir.laws.iter().for_each(|l| f(&l.body));
    for c in &lir.checks {
        if let CheckWhat::Case { args, expect, .. } = &c.what {
            args.iter().flatten().for_each(&mut *f);
            match expect {
                Expected::Value(x) => f(x),
                Expected::Fail { args, .. } => args.iter().for_each(&mut *f),
            }
        }
    }
    for h in &lir.handlers {
        if let Some(p) = &h.pretend {
            p.state.iter().filter_map(|s| s.default.as_ref()).for_each(&mut *f);
            p.arms.iter().flatten().for_each(|a| f(&a.body));
        }
    }
}

fn stmt_roots<'a>(s: &'a Stmt, f: &mut dyn FnMut(&'a Expr)) {
    match &s.kind {
        StmtKind::Let { value, .. } => f(value),
        StmtKind::Assign { place, value } => {
            f(value);
            place_roots(place, f);
        }
        StmtKind::Mutate { place, args, .. } => {
            args.iter().for_each(&mut *f);
            place_roots(place, f);
        }
        StmtKind::Expr(e) | StmtKind::Assert(e) => f(e),
        StmtKind::Return(v) => v.iter().for_each(f),
        StmtKind::For { iter, body, .. } => {
            f(iter);
            body.stmts.iter().for_each(|s| stmt_roots(s, f));
        }
        StmtKind::While { cond, body } => {
            f(cond);
            body.stmts.iter().for_each(|s| stmt_roots(s, f));
        }
        StmtKind::If { cond, then, els } => {
            f(cond);
            then.stmts.iter().for_each(|s| stmt_roots(s, f));
            els.iter().flat_map(|b| &b.stmts).for_each(|s| stmt_roots(s, f));
        }
        StmtKind::Expect { subject, expect } => {
            f(subject);
            match expect {
                Expected::Value(x) => f(x),
                Expected::Fail { args, .. } => args.iter().for_each(&mut *f),
            }
        }
    }
}

fn place_roots<'a>(p: &'a Place, f: &mut dyn FnMut(&'a Expr)) {
    for s in &p.path {
        if let PlaceStep::MapKey(k) | PlaceStep::ListIndex(k) = s {
            f(k);
        }
    }
}
