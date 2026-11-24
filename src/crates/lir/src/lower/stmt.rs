//! Statements, blocks and places.

use super::{R, Typer, id32};
use crate::node::*;
use crate::prim::Prim;
use crate::ty::{DTy, LTy};
use spec_lang_diag::Diag;
use spec_lang_ir as ir;
use spec_lang_ir::{MutOp, StmtKind as S, Ty};

/// Whether running the statements can reach their end without `return` or a failure.
fn falls_through(stmts: &[Stmt]) -> bool {
    match stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Return(_)) => false,
        Some(StmtKind::If { then, els: Some(els), .. }) => falls_through(&then.stmts) || falls_through(&els.stmts),
        Some(StmtKind::Expr(e)) => e.ty != LTy::Never,
        _ => true,
    }
}

impl<'p> Typer<'_, 'p> {
    /// A block whose `return` is its value (a body, a lambda, a block expression).
    pub(crate) fn value_block(&mut self, b: &'p ir::Block) -> R<Block> {
        let ret = self.fresh();
        self.returns.push(ret.clone());
        let stmts = self.stmts(&b.stmts);
        self.returns.pop();
        let stmts = stmts?;
        if falls_through(&stmts) {
            match self.shallow(&ret) {
                LTy::Var(_) | LTy::Unit => self.unify(&ret, &LTy::Unit, b.span)?,
                // The engine gives `()` here; the conform of a declared result reports it.
                _ => {}
            }
        } else if let LTy::Var(_) = self.shallow(&ret) {
            self.solve(&ret, LTy::Never);
        }
        Ok(Block { stmts, ty: ret, span: b.span })
    }

    /// A block inside statements: a `return` leaves the enclosing value block.
    pub(crate) fn block(&mut self, b: &'p ir::Block) -> R<Block> {
        let ty = self.returns.last().cloned().unwrap_or(LTy::Unit);
        Ok(Block { stmts: self.stmts(&b.stmts)?, ty, span: b.span })
    }

    pub(crate) fn stmts(&mut self, stmts: &'p [ir::Stmt]) -> R<Vec<Stmt>> {
        let mark = self.mark();
        let out: R<Vec<Stmt>> = stmts.iter().map(|s| self.stmt(s)).collect();
        self.reset(mark);
        out
    }

    pub(crate) fn stmt(&mut self, s: &'p ir::Stmt) -> R<Stmt> {
        let span = s.span;
        let kind = match &s.kind {
            S::Let { name, mutable, ty, value } => {
                let v = self.expr(value)?;
                let check = match ty {
                    Some(t) => {
                        let want = self.low.erase(t);
                        self.unify(&v.ty, &want, span)?;
                        Some(self.low.dty(t))
                    }
                    None => None,
                };
                let lty = v.ty.clone();
                self.bind(name, lty.clone());
                StmtKind::Let { name: name.clone(), mutable: *mutable, ty: lty, check, value: v }
            }
            S::Assign { place, value } => {
                let v = self.expr(value)?;
                let p = self.place(place)?;
                self.unify(&v.ty, &p.ty, span)?;
                StmtKind::Assign { place: p, value: v }
            }
            S::Mutate { place, op, args } => {
                let args = self.exprs(args)?;
                let p = self.place(place)?;
                let op = self.mut_prim(&p.ty, *op, &args, span)?;
                StmtKind::Mutate { place: p, op, args }
            }
            S::Expr(e) => StmtKind::Expr(self.expr(e)?),
            S::Return(v) => {
                let ret = self.returns.last().cloned().unwrap_or(LTy::Unit);
                let v = match v {
                    Some(e) => Some(self.expr(e)?),
                    None => None,
                };
                let ty = v.as_ref().map_or(LTy::Unit, |e| e.ty.clone());
                self.unify(&ret, &ty, span)?;
                StmtKind::Return(v)
            }
            S::Assert(e) => StmtKind::Assert(self.expr_as(e, &LTy::Bool)?),
            S::For { var, iter, body } => {
                let it = self.expr(iter)?;
                let (iter, item) = match self.shallow(&it.ty) {
                    LTy::List(t) => (it, *t),
                    LTy::Set(t) => (self.prim(Prim::SetList, vec![it], LTy::List(t.clone()), iter.span), *t),
                    LTy::Range => (self.prim(Prim::RangeList, vec![it], LTy::list(LTy::Int), iter.span), LTy::Int),
                    other => {
                        return Err(Diag::error(
                            "E-TYPE-MISMATCH",
                            iter.span,
                            format!("`for` needs a List, Set or range, found {}", self.low.show_ty(&self.zonk(&other))),
                        )
                        .with_decl(self.decl.clone()));
                    }
                };
                let mark = self.mark();
                self.bind(var, item.clone());
                let body = self.block(body);
                self.reset(mark);
                StmtKind::For { var: var.clone(), ty: item, iter, body: body? }
            }
            S::While { cond, body } => {
                let cond = self.expr_as(cond, &LTy::Bool)?;
                StmtKind::While { cond, body: self.block(body)? }
            }
            S::If { cond, then, els } => {
                let cond = self.expr_as(cond, &LTy::Bool)?;
                let then = self.block(then)?;
                let els = match els {
                    Some(b) => Some(self.block(b)?),
                    None => None,
                };
                StmtKind::If { cond, then, els }
            }
            S::Expect { subject, expect } => {
                let subject = self.expr(subject)?;
                let expect = self.expected(expect, &subject.ty)?;
                StmtKind::Expect { subject, expect }
            }
        };
        Ok(Stmt { kind, span })
    }

    pub(crate) fn expected(&mut self, e: &'p ir::Expect, ty: &LTy) -> R<Expected> {
        Ok(match e {
            ir::Expect::Value(x) => Expected::Value(self.expr_as(x, ty)?),
            ir::Expect::Fail(f) => Expected::Fail { name: f.name.clone(), args: self.exprs(&f.args)? },
        })
    }

    fn mut_prim(&mut self, ty: &LTy, op: MutOp, args: &[Expr], span: spec_lang_syntax::Span) -> R<Prim> {
        let fit = |me: &mut Self, want: &[LTy]| -> R<()> {
            if want.len() != args.len() {
                return Err(Diag::error("E-TYPE-ARITY", span, format!("`{op:?}` takes {} argument(s)", want.len()))
                    .with_decl(me.decl.clone()));
            }
            for (a, w) in args.iter().zip(want) {
                me.unify(&a.ty, w, a.span)?;
            }
            Ok(())
        };
        match (self.shallow(ty), op) {
            (LTy::List(t), MutOp::Push) => fit(self, &[*t]).map(|_| Prim::ListPush),
            (LTy::List(t), MutOp::Insert) => fit(self, &[LTy::Int, *t]).map(|_| Prim::ListInsert),
            (LTy::List(_), MutOp::Remove) => fit(self, &[LTy::Int]).map(|_| Prim::ListRemove),
            (LTy::Set(t), MutOp::Insert) => fit(self, &[*t]).map(|_| Prim::SetInsert),
            (LTy::Set(t), MutOp::Remove) => fit(self, &[*t]).map(|_| Prim::SetRemove),
            (LTy::Map(k, _), MutOp::Remove) => fit(self, &[*k]).map(|_| Prim::MapRemove),
            (other, op) => Err(Diag::error(
                "E-TYPE-MISMATCH",
                span,
                format!("`{op:?}` does not apply to {}", self.low.show_ty(&self.zonk(&other))),
            )
            .with_decl(self.decl.clone())),
        }
    }

    fn place(&mut self, p: &'p ir::Place) -> R<Place> {
        let prog = self.low.prog;
        let (root, mut ty, mut decl): (PlaceRoot, LTy, Option<&'p Ty>) = match &p.root {
            ir::PlaceRoot::State(s) => {
                let st = &prog.state(*s).ty;
                (PlaceRoot::State(*s), self.low.erase(st), Some(st))
            }
            ir::PlaceRoot::Local(n) => match self.local(n) {
                Some(t) => (PlaceRoot::Local(n.clone()), t, None),
                None => return Err(self.unsupported(p.span, format!("`{n}` has no value here"))),
            },
        };
        let is_state = matches!(root, PlaceRoot::State(_));
        let mut path = Vec::new();
        for step in &p.path {
            match step {
                ir::PlaceStep::Field(f) => {
                    let LTy::Named(t) = self.shallow(&ty) else {
                        return Err(self.unsupported(p.span, format!("`.{f}` of a value that is not a record")));
                    };
                    let ir::TypeDef::Record(fs) = &prog.ty(t).def else {
                        return Err(self.unsupported(p.span, format!("`.{f}` of a value that is not a record")));
                    };
                    let Some(i) = fs.iter().position(|x| &x.name == f) else {
                        return Err(Diag::error("E-TYPE-MISMATCH", p.span, format!("no field `{f}`"))
                            .with_decl(self.decl.clone()));
                    };
                    ty = self.low.erase(&fs[i].ty);
                    decl = decl.and_then(|d| field_ty(prog, d, f));
                    path.push(PlaceStep::Field { ty: t, index: id32(i) });
                }
                ir::PlaceStep::Index(k) => match self.shallow(&ty) {
                    LTy::Map(kt, v) => {
                        path.push(PlaceStep::MapKey(self.expr_as(k, &kt)?));
                        ty = *v;
                        decl = decl.and_then(|d| item_ty(prog, d));
                    }
                    LTy::List(v) => {
                        path.push(PlaceStep::ListIndex(self.expr_as(k, &LTy::Int)?));
                        ty = *v;
                        decl = decl.and_then(|d| item_ty(prog, d));
                    }
                    other => {
                        return Err(Diag::error(
                            "E-TYPE-MISMATCH",
                            p.span,
                            format!("cannot write inside {}", self.low.show_ty(&self.zonk(&other))),
                        )
                        .with_decl(self.decl.clone()));
                    }
                },
            }
        }
        let check: Option<DTy> = if is_state { decl.map(|d| self.low.dty(d)) } else { None };
        Ok(Place { root, path, ty, check })
    }
}

/// The declared type of a record field inside `t`, as the engine finds it for
/// a state write (`None` past anything but an alias, a claim or a record).
fn field_ty<'p>(prog: &'p ir::Program, t: &'p Ty, f: &str) -> Option<&'p Ty> {
    match t {
        Ty::Named(id) => match &prog.ty(*id).def {
            ir::TypeDef::Alias(a) => field_ty(prog, a, f),
            ir::TypeDef::Record(fs) => fs.iter().find(|x| x.name == f).map(|x| &x.ty),
            _ => None,
        },
        Ty::Where(b, _) => field_ty(prog, b, f),
        _ => None,
    }
}

/// The declared item type of a `Map` or `List` inside `t`.
fn item_ty<'p>(prog: &'p ir::Program, t: &'p Ty) -> Option<&'p Ty> {
    match t {
        Ty::Map(_, v) | Ty::List(v) => Some(v),
        Ty::Named(id) => match &prog.ty(*id).def {
            ir::TypeDef::Alias(a) => item_ty(prog, a),
            _ => None,
        },
        Ty::Where(b, _) => item_ty(prog, b),
        _ => None,
    }
}
