//! IR to LIR. Each declaration is typed on its own (unknowns solved by
//! unification inside it); a `fn`, rule or `const` without a written type
//! is lowered first, on demand, to learn it. Anything the LIR cannot say is
//! an error at its span (`E-UNSUPPORTED`, `E-TYPE-MISMATCH`), never a guess.

mod checks;
mod decls;
mod expr;
mod handlers;
mod member;
mod pattern;
mod stmt;
mod typer;
mod zonk;

use crate::program::*;
use crate::ty::{ClaimId, DTy, LTy};
use spec_lang_diag::Diag;
use spec_lang_ir as ir;
use spec_lang_ir::{ConstId, FnId, RuleId, Ty, TypeDef as IrTypeDef};
pub(crate) use typer::{R, Typer};

/// Lower a whole program. Every error found is returned, deduplicated.
pub fn lower(prog: &ir::Program) -> Result<Lir, Vec<Diag>> {
    let mut low = Lowerer::new(prog);
    let mut errors = Vec::new();
    let mut types = Vec::new();
    for i in 0..prog.types.len() {
        match decls::lower_type(&mut low, ir::TypeId(id32(i))) {
            Ok(t) => types.push(t),
            Err(d) => errors.push(d),
        }
    }
    for i in 0..prog.consts.len() {
        if let Err(d) = low.ensure_const(ConstId(id32(i))) {
            errors.push(d);
        }
    }
    let mut states = Vec::new();
    for i in 0..prog.states.len() {
        match decls::lower_state(&mut low, ir::StateId(id32(i))) {
            Ok(s) => states.push(s),
            Err(d) => errors.push(d),
        }
    }
    for i in 0..prog.fns.len() {
        if let Err(d) = low.ensure_fn(FnId(id32(i))) {
            errors.push(d);
        }
    }
    for i in 0..prog.rules.len() {
        if let Err(d) = low.ensure_rule(RuleId(id32(i))) {
            errors.push(d);
        }
    }
    let mut handlers = Vec::new();
    for i in 0..prog.handlers.len() {
        match handlers::lower_handler(&mut low, ir::HandlerId(id32(i))) {
            Ok(h) => handlers.push(h),
            Err(d) => errors.push(d),
        }
    }
    let mut tests = Vec::new();
    for i in 0..prog.tests.len() {
        match checks::lower_test(&mut low, ir::TestId(id32(i))) {
            Ok(t) => tests.push(t),
            Err(d) => errors.push(d),
        }
    }
    let mut laws = Vec::new();
    for i in 0..prog.laws.len() {
        match checks::lower_law(&mut low, ir::LawId(id32(i))) {
            Ok(l) => laws.push(l),
            Err(d) => errors.push(d),
        }
    }
    let checks = match checks::checks(&mut low) {
        Ok(c) => c,
        Err(d) => {
            errors.push(d);
            Vec::new()
        }
    };
    // Claims last: lowering one can name another.
    let mut claims = Vec::new();
    let mut i = 0;
    while i < low.claims.len() {
        match decls::lower_claim(&mut low, ClaimId(id32(i))) {
            Ok(c) => claims.push(c),
            Err(d) => errors.push(d),
        }
        i += 1;
    }
    let consts: Vec<Const> = low.consts.iter_mut().filter_map(Option::take).collect();
    let fns: Vec<Func> = low.fns.iter_mut().filter_map(Option::take).collect();
    let rules: Vec<Rule> = low.rules.iter_mut().filter_map(Option::take).collect();
    if !errors.is_empty() {
        let mut seen = Vec::new();
        errors.retain(|d: &Diag| {
            let key = (d.code, d.span, d.message.clone());
            let new = !seen.contains(&key);
            seen.push(key);
            new
        });
        return Err(errors);
    }
    let mut lir = Lir {
        modules: prog
            .modules
            .iter()
            .map(|m| Module {
                id: m.id,
                name: m.name.clone(),
                path: m.path.clone(),
                vocabularies: m.vocabularies.clone(),
            })
            .collect(),
        types,
        claims,
        consts,
        states,
        fns,
        rules,
        tests,
        laws,
        checks,
        apps: Vec::new(),
        handlers,
        option_type: prog.option_type,
        result_type: prog.result_type,
        libraries: prog.libraries.iter().map(|l| Library { name: l.name.clone(), dir: l.dir.clone() }).collect(),
        sources: copy_sources(&prog.sources),
    };
    lir.apps = prog.apps.iter().map(|a| crate::manifest_of::manifest(prog, &lir, a)).collect();
    Ok(lir)
}

/// The program's files again, for [`Lir::sources`].
fn copy_sources(from: &spec_lang_syntax::SourceMap) -> spec_lang_syntax::SourceMap {
    let mut to = spec_lang_syntax::SourceMap::new();
    let mut id = 0;
    while let Some(f) = from.get(spec_lang_syntax::FileId(id)) {
        to.add(spec_lang_syntax::SourceFile::new(f.path.clone(), f.text.clone(), f.first_line));
        id += 1;
    }
    to
}

pub(crate) fn id32(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(u32::MAX)
}

/// Where a lazily lowered declaration stands.
#[derive(Clone, Debug)]
enum Slot {
    Todo,
    Busy,
    Done(LTy),
    Failed(Box<Diag>),
}

/// A claim found while building a declared type.
#[derive(Debug)]
pub(crate) struct ClaimSlot<'p> {
    pub(crate) src: &'p ir::Claim,
    pub(crate) self_ty: LTy,
}

/// Program-wide lowering state.
#[derive(Debug)]
pub(crate) struct Lowerer<'p> {
    pub(crate) prog: &'p ir::Program,
    pub(crate) claims: Vec<ClaimSlot<'p>>,
    fn_slots: Vec<Slot>,
    rule_slots: Vec<Slot>,
    const_slots: Vec<Slot>,
    pub(crate) fns: Vec<Option<Func>>,
    pub(crate) rules: Vec<Option<Rule>>,
    pub(crate) consts: Vec<Option<Const>>,
}

impl<'p> Lowerer<'p> {
    fn new(prog: &'p ir::Program) -> Self {
        Lowerer {
            prog,
            claims: Vec::new(),
            fn_slots: vec![Slot::Todo; prog.fns.len()],
            rule_slots: vec![Slot::Todo; prog.rules.len()],
            const_slots: vec![Slot::Todo; prog.consts.len()],
            fns: (0..prog.fns.len()).map(|_| None).collect(),
            rules: (0..prog.rules.len()).map(|_| None).collect(),
            consts: (0..prog.consts.len()).map(|_| None).collect(),
        }
    }

    pub(crate) fn info(&self, i: &ir::DeclInfo) -> Info {
        Info {
            module: i.module,
            name: i.name.clone(),
            qualified: self.prog.qualified(i),
            is_pub: i.is_pub,
            why: i.why.clone(),
            span: i.span,
        }
    }

    /// A declared type with its claims erased.
    pub(crate) fn erase(&self, t: &Ty) -> LTy {
        match t {
            Ty::Unit => LTy::Unit,
            Ty::Bool => LTy::Bool,
            Ty::Int | Ty::Range { .. } => LTy::Int,
            Ty::Text => LTy::Text,
            Ty::Named(id) => match &self.prog.ty(*id).def {
                IrTypeDef::Alias(a) => self.erase(a),
                IrTypeDef::Rules(rs) => LTy::Event(rs.clone()),
                _ => LTy::Named(*id),
            },
            Ty::Option(x) => LTy::option(self.erase(x)),
            Ty::List(x) => LTy::list(self.erase(x)),
            Ty::Set(x) => LTy::set(self.erase(x)),
            Ty::Map(k, v) => LTy::map(self.erase(k), self.erase(v)),
            Ty::Tuple(ts) => LTy::Tuple(ts.iter().map(|x| self.erase(x)).collect()),
            Ty::Where(b, _) => self.erase(b),
            Ty::Trace(rs) => LTy::Trace(rs.clone()),
        }
    }

    /// A declared type, its claims interned.
    pub(crate) fn dty(&mut self, t: &'p Ty) -> DTy {
        match t {
            Ty::Unit => DTy::Unit,
            Ty::Bool => DTy::Bool,
            Ty::Int => DTy::Int,
            Ty::Text => DTy::Text,
            Ty::Named(id) => DTy::Named(*id),
            Ty::Option(x) => DTy::Option(Box::new(self.dty(x))),
            Ty::List(x) => DTy::List(Box::new(self.dty(x))),
            Ty::Set(x) => DTy::Set(Box::new(self.dty(x))),
            Ty::Map(k, v) => DTy::Map(Box::new(self.dty(k)), Box::new(self.dty(v))),
            Ty::Tuple(ts) => DTy::Tuple(ts.iter().map(|x| self.dty(x)).collect()),
            Ty::Range { lo, hi, inclusive } => DTy::Range { lo: *lo, hi: *hi, inclusive: *inclusive },
            Ty::Where(b, c) => {
                let base = self.dty(b);
                let id = match self.claims.iter().position(|s| std::ptr::eq(s.src, &**c) || *s.src == **c) {
                    Some(i) => ClaimId(id32(i)),
                    None => {
                        let self_ty = self.erase(b);
                        self.claims.push(ClaimSlot { src: c, self_ty });
                        ClaimId(id32(self.claims.len() - 1))
                    }
                };
                DTy::Where(Box::new(base), id)
            }
            Ty::Trace(rs) => DTy::Trace(rs.clone()),
        }
    }

    /// A type as source writes it, for messages.
    pub(crate) fn show_ty(&self, t: &LTy) -> String {
        crate::print::text::ty(self.prog, t)
    }

    pub(crate) fn fn_result(&mut self, f: FnId) -> R<LTy> {
        if let Some(t) = &self.prog.func(f).result {
            return Ok(self.erase(t));
        }
        self.ensure_fn(f)?;
        self.done(&self.fn_slots[f.index()], &self.prog.func(f).info)
    }

    pub(crate) fn rule_result(&mut self, r: RuleId) -> R<LTy> {
        if let Some(t) = &self.prog.rule(r).result {
            return Ok(self.erase(t));
        }
        self.ensure_rule(r)?;
        self.done(&self.rule_slots[r.index()], &self.prog.rule(r).info)
    }

    pub(crate) fn const_ty(&mut self, c: ConstId) -> R<LTy> {
        if let Some(t) = &self.prog.constant(c).ty {
            return Ok(self.erase(t));
        }
        self.ensure_const(c)?;
        self.done(&self.const_slots[c.index()], &self.prog.constant(c).info)
    }

    fn done(&self, s: &Slot, info: &ir::DeclInfo) -> R<LTy> {
        match s {
            Slot::Done(t) => Ok(t.clone()),
            Slot::Failed(d) => Err((**d).clone()),
            Slot::Todo | Slot::Busy => Err(Diag::unsupported(
                info.name_span,
                "a declaration whose type depends on itself: write its result type",
            )
            .with_decl(self.prog.qualified(info))),
        }
    }

    pub(crate) fn ensure_fn(&mut self, f: FnId) -> R<()> {
        match &self.fn_slots[f.index()] {
            Slot::Done(_) => return Ok(()),
            Slot::Failed(d) => return Err((**d).clone()),
            Slot::Busy => return self.done(&Slot::Busy, &self.prog.func(f).info).map(|_| ()),
            Slot::Todo => {}
        }
        self.fn_slots[f.index()] = Slot::Busy;
        match decls::lower_fn(self, f) {
            Ok(func) => {
                self.fn_slots[f.index()] = Slot::Done(func.result.clone());
                self.fns[f.index()] = Some(func);
                Ok(())
            }
            Err(d) => {
                self.fn_slots[f.index()] = Slot::Failed(Box::new(d.clone()));
                Err(d)
            }
        }
    }

    pub(crate) fn ensure_rule(&mut self, r: RuleId) -> R<()> {
        match &self.rule_slots[r.index()] {
            Slot::Done(_) => return Ok(()),
            Slot::Failed(d) => return Err((**d).clone()),
            Slot::Busy => return self.done(&Slot::Busy, &self.prog.rule(r).info).map(|_| ()),
            Slot::Todo => {}
        }
        self.rule_slots[r.index()] = Slot::Busy;
        match decls::lower_rule(self, r) {
            Ok(rule) => {
                self.rule_slots[r.index()] = Slot::Done(rule.result.clone());
                self.rules[r.index()] = Some(rule);
                Ok(())
            }
            Err(d) => {
                self.rule_slots[r.index()] = Slot::Failed(Box::new(d.clone()));
                Err(d)
            }
        }
    }

    pub(crate) fn ensure_const(&mut self, c: ConstId) -> R<()> {
        match &self.const_slots[c.index()] {
            Slot::Done(_) => return Ok(()),
            Slot::Failed(d) => return Err((**d).clone()),
            Slot::Busy => return self.done(&Slot::Busy, &self.prog.constant(c).info).map(|_| ()),
            Slot::Todo => {}
        }
        self.const_slots[c.index()] = Slot::Busy;
        match decls::lower_const(self, c) {
            Ok(k) => {
                self.const_slots[c.index()] = Slot::Done(k.ty.clone());
                self.consts[c.index()] = Some(k);
                Ok(())
            }
            Err(d) => {
                self.const_slots[c.index()] = Slot::Failed(Box::new(d.clone()));
                Err(d)
            }
        }
    }
}
