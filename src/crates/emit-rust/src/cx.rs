//! What every part of the printer reads: the LIR, names, Rust types, and the facts computed once (which
//! declarations need the machine, which fields are boxed, which checks are trivial).

use crate::names::{Names, ident};
use spec_lang_diag::Diag;
use spec_lang_ir::{ModuleId, TypeId};
use spec_lang_lir::node::{Expr, ExprKind, LambdaBody, Lambda};
use spec_lang_lir::{Body, DTy, LTy, Lir, TypeDef, walk};
use std::collections::BTreeSet;

/// A DTy and the name of the function printed for it.
#[derive(Debug, Default)]
pub struct Registry {
    pub items: Vec<(DTy, String)>,
}

impl Registry {
    /// The function's name, registering it on first use.
    pub fn name(&mut self, d: &DTy, make: impl FnOnce(&BTreeSet<String>) -> String) -> String {
        if let Some((_, n)) = self.items.iter().find(|(x, _)| x == d) {
            return n.clone();
        }
        let taken: BTreeSet<String> = self.items.iter().map(|(_, n)| n.clone()).collect();
        let n = make(&taken);
        self.items.push((d.clone(), n.clone()));
        n
    }
}

/// Which mode a printed program runs in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Checks only: handlers run their pretend, foreign bodies block.
    Tests,
    /// An app: handlers run their pretend in checks and the library runtime in the app; foreign bodies call it.
    App,
}

#[derive(Debug)]
pub struct Cx<'a> {
    pub lir: &'a Lir,
    pub names: Names,
    pub mode: Mode,
    pub fn_m: Vec<bool>,
    pub const_m: Vec<bool>,
    pub claim_m: Vec<bool>,
    /// Fields held in a `Box` (a type that contains itself): (type, variant or `u32::MAX`, field).
    pub boxed: BTreeSet<(TypeId, u32, u32)>,
    pub conforms: Registry,
    pub gens: Registry,
    pub diags: Vec<Diag>,
}

pub const RECORD: u32 = u32::MAX;

impl<'a> Cx<'a> {
    pub fn new(lir: &'a Lir, mode: Mode) -> Self {
        let mut cx = Cx {
            lir,
            names: Names::new(lir),
            mode,
            fn_m: vec![false; lir.fns.len()],
            const_m: vec![false; lir.consts.len()],
            claim_m: vec![false; lir.claims.len()],
            boxed: BTreeSet::new(),
            conforms: Registry::default(),
            gens: Registry::default(),
            diags: Vec::new(),
        };
        cx.boxes();
        cx.machine_needs();
        cx
    }

    /// Where a declaration is written: `path:line`.
    pub fn origin(&self, info: &spec_lang_lir::Info) -> String {
        let at = self.lir.sources.location(info.span);
        format!("{}:{}", at.path, at.line)
    }

    pub fn unsupported(&mut self, span: spec_lang_syntax::Span, msg: impl Into<String>) {
        self.diags.push(Diag::error("E-UNSUPPORTED", span, msg));
    }

    // ---- types ----

    /// The Rust type of an `LTy`, written from module `from`.
    pub fn ty(&self, t: &LTy, from: Option<ModuleId>) -> String {
        match t {
            LTy::Unit => "()".into(),
            LTy::Bool => "bool".into(),
            LTy::Int => "Int".into(),
            LTy::Text => "String".into(),
            LTy::Named(id) if *id == self.lir.option_type => "Opt<()>".into(),
            LTy::Named(id) if *id == self.lir.result_type => "crate::machine::Reply".into(),
            LTy::Named(id) => self.type_path(*id, from),
            LTy::Option(x) => format!("Opt<{}>", self.ty(x, from)),
            LTy::List(x) => format!("Vec<{}>", self.ty(x, from)),
            LTy::Set(x) => format!("BTreeSet<{}>", self.ty(x, from)),
            LTy::Map(k, v) => format!("BTreeMap<{}, {}>", self.ty(k, from), self.ty(v, from)),
            LTy::Tuple(xs) => self.tuple_ty(xs, from),
            LTy::Range => "Range".into(),
            LTy::Func(ps, r) => format!("Fun<{}, {}>", self.tuple_ty(ps, from), self.ty(r, from)),
            LTy::Event(_) => "crate::machine::Event".into(),
            LTy::Reply(_) => "crate::machine::Reply".into(),
            LTy::Failure => "rt::Failure".into(),
            LTy::Step(_) => "rt::runner::Step".into(),
            LTy::World => "crate::machine::World".into(),
            LTy::Trace(_) => "rt::runner::Trace".into(),
            LTy::Never | LTy::Var(_) => "Never".into(),
        }
    }

    pub fn tuple_ty(&self, xs: &[LTy], from: Option<ModuleId>) -> String {
        match xs {
            [] => "()".into(),
            [x] => format!("({},)", self.ty(x, from)),
            xs => format!("({})", xs.iter().map(|x| self.ty(x, from)).collect::<Vec<_>>().join(", ")),
        }
    }

    pub fn type_path(&self, id: TypeId, from: Option<ModuleId>) -> String {
        let d = self.lir.ty(id);
        self.names.path(from, d.info.module, &ident(&d.info.name))
    }

    /// Whether a value of the type is `Copy` in Rust (read without `.clone()`).
    pub fn copy(&self, t: &LTy) -> bool {
        match t {
            LTy::Unit | LTy::Bool | LTy::Int | LTy::Range | LTy::Never => true,
            LTy::Tuple(xs) => xs.iter().all(|x| self.copy(x)),
            _ => false,
        }
    }

    /// The `LTy` a value of a declared type has.
    pub fn lty(&self, d: &DTy) -> LTy {
        match d {
            DTy::Unit => LTy::Unit,
            DTy::Bool => LTy::Bool,
            DTy::Int | DTy::Range { .. } => LTy::Int,
            DTy::Text => LTy::Text,
            DTy::Named(t) => match &self.lir.ty(*t).def {
                TypeDef::Alias { of } => self.lty(of),
                TypeDef::RuleSet { rules } => LTy::Event(rules.clone()),
                _ => LTy::Named(*t),
            },
            DTy::Option(x) => LTy::option(self.lty(x)),
            DTy::List(x) => LTy::list(self.lty(x)),
            DTy::Set(x) => LTy::set(self.lty(x)),
            DTy::Map(k, v) => LTy::map(self.lty(k), self.lty(v)),
            DTy::Tuple(xs) => LTy::Tuple(xs.iter().map(|x| self.lty(x)).collect()),
            DTy::Where(b, _) => self.lty(b),
            DTy::Trace(rs) => LTy::Trace(rs.clone()),
        }
    }

    // ---- boxes ----

    /// Named types a type holds without a collection between.
    fn direct(&self, t: &LTy, out: &mut BTreeSet<TypeId>) {
        match t {
            LTy::Named(id) => {
                out.insert(*id);
            }
            LTy::Option(x) => self.direct(x, out),
            LTy::Tuple(xs) => xs.iter().for_each(|x| self.direct(x, out)),
            _ => {}
        }
    }

    fn fields(&self, id: TypeId) -> Vec<(u32, u32, &'a LTy)> {
        match &self.lir.ty(id).def {
            TypeDef::Record { fields } => fields.iter().enumerate().map(|(i, f)| (RECORD, i as u32, &f.ty)).collect(),
            TypeDef::Sum { variants } => variants
                .iter()
                .enumerate()
                .flat_map(|(v, var)| var.fields.iter().enumerate().map(move |(i, f)| (v as u32, i as u32, &f.ty)))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn reaches(&self, from: TypeId, to: TypeId) -> bool {
        let mut seen = BTreeSet::new();
        let mut stack = vec![from];
        while let Some(t) = stack.pop() {
            if !seen.insert(t) {
                continue;
            }
            for (_, _, ty) in self.fields(t) {
                let mut d = BTreeSet::new();
                self.direct(ty, &mut d);
                if d.contains(&to) {
                    return true;
                }
                stack.extend(d);
            }
        }
        false
    }

    fn boxes(&mut self) {
        for (i, decl) in self.lir.types.iter().enumerate() {
            if decl.builtin {
                continue;
            }
            let id = TypeId(i as u32);
            for (v, f, ty) in self.fields(id) {
                let mut d = BTreeSet::new();
                self.direct(ty, &mut d);
                if d.iter().any(|u| *u == id || self.reaches(*u, id)) {
                    self.boxed.insert((id, v, f));
                }
            }
        }
    }

    pub fn is_boxed(&self, ty: TypeId, variant: u32, field: u32) -> bool {
        self.boxed.contains(&(ty, variant, field))
    }

    // ---- who needs the machine ----

    /// Whether checking a value against `d` runs a claim that needs the machine.
    pub fn dty_m(&self, d: &DTy) -> bool {
        self.dty_m_in(d, &mut BTreeSet::new())
    }

    fn dty_m_in(&self, d: &DTy, seen: &mut BTreeSet<TypeId>) -> bool {
        match d {
            DTy::Option(x) | DTy::List(x) | DTy::Set(x) => self.dty_m_in(x, seen),
            DTy::Map(k, v) => self.dty_m_in(k, seen) || self.dty_m_in(v, seen),
            DTy::Tuple(xs) => xs.iter().any(|x| self.dty_m_in(x, seen)),
            DTy::Where(b, c) => self.claim_m[c.index()] || self.dty_m_in(b, seen),
            DTy::Named(t) => {
                if !seen.insert(*t) {
                    return false;
                }
                match &self.lir.ty(*t).def {
                    TypeDef::Alias { of } => self.dty_m_in(of, seen),
                    TypeDef::Record { fields } => fields.iter().any(|f| self.dty_m_in(&f.check, seen)),
                    TypeDef::Sum { variants } => {
                        variants.iter().flat_map(|v| &v.fields).any(|f| self.dty_m_in(&f.check, seen))
                    }
                    TypeDef::RuleSet { .. } => false,
                }
            }
            _ => false,
        }
    }

    /// Whether checking against `d` can say no: false for `Int`, `List[Text]`, a record of plain fields.
    pub fn trivial(&self, d: &DTy) -> bool {
        self.trivial_in(d, &mut BTreeSet::new())
    }

    fn trivial_in(&self, d: &DTy, seen: &mut BTreeSet<TypeId>) -> bool {
        match d {
            DTy::Unit | DTy::Bool | DTy::Int | DTy::Text | DTy::Trace(_) => true,
            DTy::Range { .. } | DTy::Where(..) => false,
            DTy::Option(x) | DTy::List(x) | DTy::Set(x) => self.trivial_in(x, seen),
            DTy::Map(k, v) => self.trivial_in(k, seen) && self.trivial_in(v, seen),
            DTy::Tuple(xs) => xs.iter().all(|x| self.trivial_in(x, seen)),
            DTy::Named(t) => {
                if !seen.insert(*t) {
                    return true;
                }
                let decl = self.lir.ty(*t);
                match &decl.def {
                    TypeDef::Alias { of } => self.trivial_in(of, seen),
                    TypeDef::Record { fields } => fields.iter().all(|f| self.trivial_in(&f.check, seen)),
                    TypeDef::Sum { variants } => {
                        decl.builtin || variants.iter().flat_map(|v| &v.fields).all(|f| self.trivial_in(&f.check, seen))
                    }
                    TypeDef::RuleSet { .. } => false,
                }
            }
        }
    }

    /// Whether a constant is a literal printed as a Rust `const`.
    pub fn const_literal(&self, c: spec_lang_ir::ConstId) -> bool {
        matches!(self.lir.constant(c).value.kind, ExprKind::Int(_) | ExprKind::Bool(_) | ExprKind::Text(_))
    }

    /// Whether evaluating `e` needs the machine, given what is known of the declarations it calls.
    pub fn expr_m(&self, e: &Expr) -> bool {
        let mut needs = false;
        walk::exprs(e, &mut |x| needs |= self.node_m(x));
        needs
    }

    fn node_m(&self, x: &Expr) -> bool {
        match &x.kind {
            ExprKind::State(_)
            | ExprKind::CallRule { .. }
            | ExprKind::Effect { .. }
            | ExprKind::EffectRead { .. }
            | ExprKind::CallLambda { .. } => true,
            ExprKind::CallFn { func, .. } => self.fn_m[func.index()] || self.params_m(&self.lir.func(*func).params),
            ExprKind::Const(c) => self.const_m[c.index()],
            ExprKind::Construct { ty, tag, .. } => self.fields_m(*ty, *tag),
            ExprKind::With { ty, .. } => self.fields_m(*ty, None),
            ExprKind::Prim { args, .. } => {
                args.iter().any(|a| matches!(a.ty, LTy::Func(..)) && !matches!(a.kind, ExprKind::Lambda(_)))
            }
            ExprKind::Block(b) => {
                let mut m = false;
                for s in &b.stmts {
                    if let spec_lang_lir::node::StmtKind::Let { check: Some(d), .. } = &s.kind {
                        m |= self.dty_m(d);
                    }
                }
                m
            }
            _ => false,
        }
    }

    fn params_m(&self, ps: &[spec_lang_lir::Param]) -> bool {
        ps.iter().any(|p| self.dty_m(&p.check))
    }

    fn fields_m(&self, ty: TypeId, tag: Option<u32>) -> bool {
        match (&self.lir.ty(ty).def, tag) {
            (TypeDef::Record { fields }, _) => fields.iter().any(|f| self.dty_m(&f.check)),
            (TypeDef::Sum { variants }, Some(t)) => {
                variants.get(t as usize).is_some_and(|v| v.fields.iter().any(|f| self.dty_m(&f.check)))
            }
            _ => false,
        }
    }

    /// Whether a lambda's body needs the machine (an inline closure then captures it).
    pub fn lambda_m(&self, l: &Lambda) -> bool {
        match &l.body {
            LambdaBody::Expr(x) => self.expr_m(x),
            LambdaBody::Block(b) => {
                let mut needs = false;
                walk::block_exprs(b, &mut |x| needs |= self.node_m(x));
                needs
            }
        }
    }

    fn machine_needs(&mut self) {
        loop {
            let mut changed = false;
            for i in 0..self.lir.claims.len() {
                let m = self.expr_m(&self.lir.claims[i].body);
                changed |= m && !self.claim_m[i];
                self.claim_m[i] |= m;
            }
            for (i, f) in self.lir.fns.iter().enumerate() {
                let mut m = self.params_m(&f.params) || f.result_check.as_ref().is_some_and(|d| self.dty_m(d));
                m |= f.params.iter().filter_map(|p| p.default.as_ref()).any(|d| self.expr_m(d));
                m |= match &f.body {
                    Body::Expr(e) => self.expr_m(e),
                    Body::Extern(_) => false,
                };
                changed |= m && !self.fn_m[i];
                self.fn_m[i] |= m;
            }
            for (i, c) in self.lir.consts.iter().enumerate() {
                let m = self.expr_m(&c.value);
                changed |= m && !self.const_m[i];
                self.const_m[i] |= m;
            }
            if !changed {
                break;
            }
        }
    }
}
