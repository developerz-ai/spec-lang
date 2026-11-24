//! What the printer must know about an expression before it prints it:
//! whether a body needs the machine, and what evaluating an expression can
//! do, so sibling expressions keep the language's left-to-right order where
//! C++ leaves the order of arguments open.

use spec_lang_lir::node::*;
use spec_lang_lir::{Body, Lir, Prim};
use std::collections::BTreeSet;

/// What evaluating an expression can do besides give a value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Effects {
    /// Failure names it can end with; `*` is any name or a failure with data.
    pub fails: BTreeSet<String>,
    /// It can stop (other than by leaving the Int range) or block.
    pub hard: bool,
    pub reads_state: bool,
    pub writes_state: bool,
    pub reads_local: bool,
    pub writes_local: bool,
}

impl Effects {
    fn add(&mut self, o: &Effects) {
        self.fails.extend(o.fails.iter().cloned());
        self.hard |= o.hard;
        self.reads_state |= o.reads_state;
        self.writes_state |= o.writes_state;
        self.reads_local |= o.reads_local;
        self.writes_local |= o.writes_local;
    }
    fn fail(&mut self, name: &str) {
        self.fails.insert(name.to_string());
    }
    fn unknown() -> Effects {
        let mut e =
            Effects { hard: true, reads_state: true, writes_state: true, reads_local: true, ..Effects::default() };
        e.fail("*");
        e
    }
    pub fn can_fail(&self) -> bool {
        !self.fails.is_empty()
    }
    fn halts(&self) -> bool {
        self.can_fail() || self.hard
    }
    fn writes(&self) -> bool {
        self.writes_state || self.writes_local
    }
}

/// Whether C++ may evaluate these siblings in any order and give what left to right gives.
pub fn any_order(parts: &[Effects]) -> bool {
    for (i, a) in parts.iter().enumerate() {
        for (j, b) in parts.iter().enumerate() {
            if i == j {
                continue;
            }
            let clash = (a.writes_state && (b.reads_state || b.writes_state))
                || (a.writes_local && (b.reads_local || b.writes_local))
                || (a.writes() && b.halts());
            if clash {
                return false;
            }
        }
    }
    let halting: Vec<&Effects> = parts.iter().filter(|p| p.halts()).collect();
    if halting.len() < 2 || halting.iter().all(|p| !p.can_fail()) {
        return true;
    }
    // Two that can fail: only harmless when each can fail one same way and none can stop.
    let mut names = BTreeSet::new();
    halting.iter().for_each(|p| names.extend(p.fails.iter().cloned()));
    names.len() == 1 && !names.contains("*") && halting.iter().all(|p| !p.hard)
}

#[derive(Debug)]
pub struct Facts {
    /// Per `fn`: its body needs the machine (a state, a rule, a handler).
    pub fn_ctx: Vec<bool>,
    fn_fx: Vec<Effects>,
    rule_fx: Vec<Effects>,
    /// Per type: a value of it has something `conform` checks.
    pub type_checks: Vec<bool>,
}

impl Facts {
    pub fn of(lir: &Lir) -> Facts {
        let mut f = Facts {
            fn_ctx: vec![false; lir.fns.len()],
            fn_fx: vec![Effects::default(); lir.fns.len()],
            rule_fx: vec![Effects::default(); lir.rules.len()],
            type_checks: vec![false; lir.types.len()],
        };
        loop {
            let mut changed = false;
            for (i, t) in lir.types.iter().enumerate() {
                let has = !t.builtin
                    && match &t.def {
                        spec_lang_lir::TypeDef::Alias { of } => f.checks(of),
                        spec_lang_lir::TypeDef::Record { fields } => fields.iter().any(|x| f.checks(&x.check)),
                        spec_lang_lir::TypeDef::Sum { variants } => {
                            variants.iter().flat_map(|v| &v.fields).any(|x| f.checks(&x.check))
                        }
                        spec_lang_lir::TypeDef::RuleSet { .. } => false,
                    };
                if has != f.type_checks[i] {
                    f.type_checks[i] = has;
                    changed = true;
                }
            }
            for (i, d) in lir.fns.iter().enumerate() {
                let (ctx, mut fx) = match &d.body {
                    Body::Expr(e) => (f.needs_ctx(e), f.effects(e)),
                    Body::Extern(_) => (false, Effects { hard: true, writes_state: true, ..Effects::default() }),
                };
                fx.hard |=
                    d.params.iter().any(|p| f.checks(&p.check)) || d.result_check.as_ref().is_some_and(|c| f.checks(c));
                fx.reads_local = false;
                fx.writes_local = false;
                let ctx = ctx || d.params.iter().filter_map(|p| p.default.as_ref()).any(|e| f.needs_ctx(e));
                if ctx != f.fn_ctx[i] || fx != f.fn_fx[i] {
                    f.fn_ctx[i] = ctx;
                    f.fn_fx[i] = fx;
                    changed = true;
                }
            }
            for (i, d) in lir.rules.iter().enumerate() {
                let mut fx = match &d.body {
                    Body::Expr(e) => f.effects(e),
                    Body::Extern(_) => Effects { hard: true, ..Effects::default() },
                };
                for g in &d.guards {
                    let mut c = f.effects(&g.cond);
                    c.fails.clear();
                    fx.add(&c);
                    fx.fail(&g.error);
                }
                fx.hard |=
                    d.params.iter().any(|p| f.checks(&p.check)) || d.result_check.as_ref().is_some_and(|c| f.checks(c));
                fx.reads_state = true;
                fx.writes_state = true;
                fx.reads_local = false;
                fx.writes_local = false;
                if fx != f.rule_fx[i] {
                    f.rule_fx[i] = fx;
                    changed = true;
                }
            }
            if !changed {
                return f;
            }
        }
    }

    /// Whether `conform` to this type checks anything at run time.
    pub fn checks(&self, d: &spec_lang_lir::DTy) -> bool {
        use spec_lang_lir::DTy;
        match d {
            DTy::Unit | DTy::Bool | DTy::Int | DTy::Text | DTy::Trace(_) => false,
            DTy::Named(t) => self.type_checks[t.index()],
            DTy::Option(t) | DTy::List(t) | DTy::Set(t) => self.checks(t),
            DTy::Map(k, v) => self.checks(k) || self.checks(v),
            DTy::Tuple(ts) => ts.iter().any(|t| self.checks(t)),
            DTy::Range { .. } | DTy::Where(..) => true,
        }
    }

    /// Whether evaluating `e` needs the machine.
    pub fn needs_ctx(&self, e: &Expr) -> bool {
        let mut needs = false;
        spec_lang_lir::walk::exprs(e, &mut |x| {
            needs |= match &x.kind {
                ExprKind::State(_)
                | ExprKind::CallRule { .. }
                | ExprKind::Effect { .. }
                | ExprKind::EffectRead { .. } => true,
                ExprKind::CallFn { func, .. } => self.fn_ctx[func.index()],
                _ => false,
            }
        });
        needs || block_writes_state(e)
    }

    fn all<'e>(&self, xs: impl IntoIterator<Item = &'e Expr>) -> Effects {
        let mut fx = Effects::default();
        xs.into_iter().for_each(|x| fx.add(&self.effects(x)));
        fx
    }

    /// A function value applied now: a written lambda's body, or anything.
    fn applied(&self, f: &Expr) -> Effects {
        match &f.kind {
            ExprKind::Lambda(l) => match &l.body {
                LambdaBody::Expr(x) => self.effects(x),
                LambdaBody::Block(b) => self.block(b),
            },
            _ => Effects::unknown(),
        }
    }

    pub fn effects(&self, e: &Expr) -> Effects {
        let mut fx = Effects::default();
        match &e.kind {
            ExprKind::Unit
            | ExprKind::Bool(_)
            | ExprKind::Int(_)
            | ExprKind::Text(_)
            | ExprKind::Const(_)
            | ExprKind::Variant { .. }
            | ExprKind::None => {}
            ExprKind::Local(_) => fx.reads_local = true,
            ExprKind::State(_) | ExprKind::EffectRead { .. } => fx.reads_state = true,
            ExprKind::Interp(parts) => {
                for p in parts {
                    if let Part::Show(x) = p {
                        fx.add(&self.effects(x));
                    }
                }
            }
            ExprKind::Some(x) => fx = self.effects(x),
            ExprKind::Construct { ty, args, .. } => {
                fx = self.all(args.iter().flatten());
                fx.hard |= self.type_checks[ty.index()];
            }
            ExprKind::CallFn { func, args } => {
                fx = self.all(args.iter().flatten());
                fx.add(&self.fn_fx[func.index()]);
            }
            ExprKind::CallRule { rule, args } => {
                fx = self.all(args.iter().flatten());
                fx.add(&self.rule_fx[rule.index()]);
                fx.reads_state = true;
                fx.writes_state = true;
            }
            ExprKind::Effect { args, .. } => {
                fx = self.all(args);
                fx.reads_state = true;
                fx.writes_state = true;
            }
            ExprKind::Prim { op, args } => {
                for a in args {
                    match &a.kind {
                        ExprKind::Lambda(_) => {
                            fx.add(&self.applied(a));
                            fx.reads_local = true;
                        }
                        _ if matches!(a.ty, spec_lang_lir::LTy::Func(..)) => fx.add(&Effects::unknown()),
                        _ => fx.add(&self.effects(a)),
                    }
                }
                prim_effects(*op, &mut fx);
            }
            ExprKind::Lambda(_) => fx.reads_local = true,
            ExprKind::CallLambda { callee, args } => {
                fx = self.all(args);
                fx.add(&self.effects(callee));
                fx.add(&self.applied(callee));
            }
            ExprKind::Field { recv, .. } | ExprKind::WorldState { recv, .. } => fx = self.effects(recv),
            ExprKind::VariantField { recv, .. } | ExprKind::EventArg { recv, .. } => {
                fx = self.effects(recv);
                fx.hard = true;
            }
            ExprKind::With { recv, ty, changes } => {
                fx = self.all(changes.iter().map(|(_, x)| x));
                fx.add(&self.effects(recv));
                fx.hard |= self.type_checks[ty.index()];
            }
            ExprKind::Is { value, pat } => {
                fx = self.effects(value);
                fx.add(&self.pat(pat));
            }
            ExprKind::If { cond, then, els } => fx = self.all([&**cond, &**then, &**els]),
            ExprKind::Logic { left, right, .. } => fx = self.all([&**left, &**right]),
            ExprKind::Compare { first, links } => {
                fx = self.effects(first);
                for (op, x) in links {
                    fx.add(&self.effects(x));
                    prim_effects(*op, &mut fx);
                }
            }
            ExprKind::Else { value, fallback, .. } => {
                fx = self.effects(value);
                fx.fails.clear();
                fx.add(&self.effects(fallback));
            }
            ExprKind::Fail { name, args } => {
                fx = self.all(args);
                fx.fail(if args.is_empty() { name } else { "*" });
            }
            ExprKind::Match { scrutinee, arms } => {
                fx = self.effects(scrutinee);
                for a in arms {
                    fx.add(&self.pat(&a.pat));
                    fx.add(&self.effects(&a.value));
                }
                fx.hard |= arms.last().is_none_or(|a| a.pat.steps.iter().any(|s| !matches!(s, PatStep::Bind { .. })));
            }
            ExprKind::Block(b) => fx = self.block(b),
            ExprKind::Hole(_) => fx.hard = true,
            ExprKind::Tuple(xs) | ExprKind::List(xs) | ExprKind::SetOf(xs) => fx = self.all(xs),
            ExprKind::MapOf(pairs) => fx = self.all(pairs.iter().flat_map(|(k, v)| [k, v])),
            ExprKind::Range { lo, hi, .. } => fx = self.all([&**lo, &**hi]),
        }
        fx
    }

    fn pat(&self, p: &Pat) -> Effects {
        self.all(p.steps.iter().filter_map(|s| match s {
            PatStep::Equals { value, .. } => Some(value),
            _ => None,
        }))
    }

    fn place(&self, p: &Place, fx: &mut Effects) {
        match p.root {
            PlaceRoot::State(_) => fx.writes_state = true,
            PlaceRoot::Local(_) => fx.writes_local = true,
        }
        fx.hard |= p.check.as_ref().is_some_and(|c| self.checks(c));
        for s in &p.path {
            match s {
                PlaceStep::Field { .. } => {}
                PlaceStep::MapKey(k) | PlaceStep::ListIndex(k) => {
                    fx.add(&self.effects(k));
                    fx.fail("Missing");
                }
            }
        }
    }

    pub fn block(&self, b: &Block) -> Effects {
        let mut fx = Effects::default();
        for s in &b.stmts {
            match &s.kind {
                StmtKind::Let { value, check, .. } => {
                    fx.add(&self.effects(value));
                    fx.hard |= check.as_ref().is_some_and(|c| self.checks(c));
                }
                StmtKind::Assign { place, value } => {
                    fx.add(&self.effects(value));
                    self.place(place, &mut fx);
                }
                StmtKind::Mutate { place, op, args } => {
                    fx.add(&self.all(args));
                    self.place(place, &mut fx);
                    prim_effects(*op, &mut fx);
                }
                StmtKind::Expr(e) => fx.add(&self.effects(e)),
                StmtKind::Return(v) => {
                    if let Some(e) = v {
                        fx.add(&self.effects(e));
                    }
                }
                StmtKind::Assert(e) => {
                    fx.add(&self.effects(e));
                    fx.hard = true;
                }
                StmtKind::For { iter, body, .. } => {
                    fx.add(&self.effects(iter));
                    fx.add(&self.block(body));
                }
                StmtKind::While { cond, body } => {
                    fx.add(&self.effects(cond));
                    fx.add(&self.block(body));
                    fx.hard = true;
                }
                StmtKind::If { cond, then, els } => {
                    fx.add(&self.effects(cond));
                    fx.add(&self.block(then));
                    if let Some(b) = els {
                        fx.add(&self.block(b));
                    }
                }
                StmtKind::Expect { subject, expect } => {
                    fx.add(&self.effects(subject));
                    match expect {
                        Expected::Value(x) => fx.add(&self.effects(x)),
                        Expected::Fail { args, .. } => fx.add(&self.all(args)),
                    }
                    fx.hard = true;
                }
            }
        }
        fx
    }
}

/// Whether a statement inside `e` writes a state: an assignment is not an expression, so no walk of expressions sees it.
fn block_writes_state(e: &Expr) -> bool {
    fn block(b: &Block) -> bool {
        b.stmts.iter().any(|s| match &s.kind {
            StmtKind::Assign { place, .. } | StmtKind::Mutate { place, .. } => {
                matches!(place.root, PlaceRoot::State(_))
            }
            StmtKind::For { body, .. } | StmtKind::While { body, .. } => block(body),
            StmtKind::If { then, els, .. } => block(then) || els.as_ref().is_some_and(block),
            _ => false,
        })
    }
    let mut writes = false;
    spec_lang_lir::walk::exprs(e, &mut |x| {
        writes |= match &x.kind {
            ExprKind::Block(b) => block(b),
            ExprKind::Lambda(l) => matches!(&l.body, LambdaBody::Block(b) if block(b)),
            _ => false,
        }
    });
    writes
}

/// What a primitive itself can do, its arguments aside.
fn prim_effects(op: Prim, fx: &mut Effects) {
    match op {
        Prim::IntMod => fx.fail("DivByZero"),
        Prim::ListFirst
        | Prim::ListLast
        | Prim::ListIndex
        | Prim::ListMin
        | Prim::ListMax
        | Prim::ListMinBy
        | Prim::ListMaxBy
        | Prim::ListInsert
        | Prim::ListRemove
        | Prim::MapIndex => fx.fail("Missing"),
        Prim::ListZip => fx.fail("Uneven"),
        Prim::IntPow
        | Prim::IntDiv
        | Prim::IntBand
        | Prim::IntBor
        | Prim::IntBxor
        | Prim::IntShl
        | Prim::IntShr
        | Prim::IntAbs
        | Prim::IntClamp
        | Prim::TextSplit
        | Prim::TextReplace
        | Prim::ListTake
        | Prim::ListDrop
        | Prim::RangeList => fx.hard = true,
        _ => {}
    }
}
