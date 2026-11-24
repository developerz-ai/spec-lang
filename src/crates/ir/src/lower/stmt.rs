//! Lowering statements, blocks, values and places.

use super::Lowerer;
use super::expr::{Body, Cx, Named, R};
use super::scope::Binding;
use crate::error::IrError;
use crate::expr::*;
use crate::ids::DeclRef;
use spec_lang_syntax::ast::{self, ExprKind as AK, StmtKind as SK};

impl Lowerer {
    pub(crate) fn lower_block(&mut self, cx: &mut Cx, b: &ast::Block) -> R<Block> {
        let mark = cx.mark();
        let mut stmts = Vec::new();
        let mut result = Ok(());
        for (i, s) in b.stmts.iter().enumerate() {
            match self.lower_stmt(cx, s) {
                Ok(st) => stmts.push(st),
                Err(e) => {
                    result = Err(e.with_at(format!("stmt[{i}]")));
                    break;
                }
            }
        }
        cx.pop_to(mark);
        result.map(|()| Block { stmts, span: b.span })
    }

    /// `value: expr ";" | block | match`, as an expression.
    pub(crate) fn lower_value(&mut self, cx: &mut Cx, v: &ast::Value) -> R<Expr> {
        match v {
            ast::Value::Expr(e) => self.lower_expr(cx, e),
            ast::Value::Block(b) => Ok(Expr::new(ExprKind::Block(self.lower_block(cx, b)?), b.span)),
            ast::Value::Match(m) => self.lower_match(cx, m),
        }
    }

    fn lower_match(&mut self, cx: &mut Cx, m: &ast::Match) -> R<Expr> {
        let scrutinee = Box::new(self.lower_expr(cx, &m.scrutinee)?);
        let mut arms = Vec::new();
        for a in &m.arms {
            let mark = cx.mark();
            let mut binds = Vec::new();
            let arm = (|| {
                let pattern = self.lower_pattern(cx, &a.pattern, &mut binds)?;
                let value = self.lower_value(cx, &a.value)?;
                Ok(Arm { pattern, value, span: a.span })
            })();
            cx.pop_to(mark);
            arms.push(arm?);
        }
        Ok(Expr::new(ExprKind::Match { scrutinee, arms }, m.span))
    }

    fn lower_stmt(&mut self, cx: &mut Cx, s: &ast::Stmt) -> R<Stmt> {
        let kind = match &s.kind {
            SK::Let { mutable, name, ty, value } => {
                let value = self.lower_value(cx, value)?;
                let ty = match ty {
                    Some(t) => Some(self.lower_ty(cx.module, t)?),
                    None => None,
                };
                self.bind_local(cx, &name.name, *mutable, name.span)?;
                StmtKind::Let { name: name.name.clone(), mutable: *mutable, ty, value }
            }
            SK::Return(v) => StmtKind::Return(match v {
                Some(v) => Some(self.lower_value(cx, v)?),
                None => None,
            }),
            SK::Assert(e) => StmtKind::Assert(self.lower_expr(cx, e)?),
            SK::Expr(e) => return self.lower_expr_stmt(cx, e, s),
            SK::Assign { place, value } => {
                let place = self.lower_place(cx, place)?;
                StmtKind::Assign { place, value: self.lower_value(cx, value)? }
            }
            SK::For { var, iter, body } => {
                let iter = self.lower_expr(cx, iter)?;
                let mark = cx.mark();
                let body = self.bind_local(cx, &var.name, false, var.span).and_then(|()| self.lower_block(cx, body));
                cx.pop_to(mark);
                StmtKind::For { var: var.name.clone(), iter, body: body? }
            }
            SK::While { cond, body } => {
                StmtKind::While { cond: self.lower_expr(cx, cond)?, body: self.lower_block(cx, body)? }
            }
            SK::If(i) => return self.lower_if(cx, i),
            SK::Match(m) => StmtKind::Expr(self.lower_match(cx, m)?),
        };
        Ok(Stmt { kind, span: s.span })
    }

    fn lower_if(&mut self, cx: &mut Cx, i: &ast::IfStmt) -> R<Stmt> {
        let cond = self.lower_expr(cx, &i.cond)?;
        let then = self.lower_block(cx, &i.then)?;
        let els = match &i.els {
            None => None,
            Some(ast::Else::Block(b)) => Some(self.lower_block(cx, b)?),
            Some(ast::Else::If(inner)) => {
                let s = self.lower_if(cx, inner)?;
                Some(Block { stmts: vec![s], span: inner.span })
            }
        };
        Ok(Stmt { kind: StmtKind::If { cond, then, els }, span: i.span })
    }

    fn lower_expr_stmt(&mut self, cx: &mut Cx, e: &ast::Expr, s: &ast::Stmt) -> R<Stmt> {
        if let AK::Arrow(subject, expected) = &e.kind {
            if cx.body != Body::Test {
                return Err(IrError::unsupported(e.span, "a `call -> value;` step outside a test"));
            }
            let subject = self.lower_expr(cx, subject)?;
            let expect = self.lower_expect(cx, expected)?;
            return Ok(Stmt { kind: StmtKind::Expect { subject, expect }, span: s.span });
        }
        if let AK::Call(callee, args) = &e.kind
            && let AK::Field(recv, m) = &callee.kind
        {
            let op = match m.name.as_str() {
                "push" => Some(MutOp::Push),
                "insert" => Some(MutOp::Insert),
                "remove" => Some(MutOp::Remove),
                _ => None,
            };
            if let Some(op) = op {
                // `board.remove(2)` is the rule `remove` of module `board`, not a mutation.
                if self.module_of(cx, recv).is_none() {
                    let place = self.lower_place(cx, recv)?;
                    let args = self.lower_positional(cx, args)?;
                    return Ok(Stmt { kind: StmtKind::Mutate { place, op, args }, span: s.span });
                }
            }
        }
        Ok(Stmt { kind: StmtKind::Expr(self.lower_expr(cx, e)?), span: s.span })
    }

    /// What a step or a case expects: `fail E` or a value.
    pub(crate) fn lower_expect(&mut self, cx: &mut Cx, e: &ast::Expr) -> R<Expect> {
        match &e.kind {
            AK::Fail(x) => Ok(Expect::Fail(self.lower_failure(cx, x)?)),
            _ => Ok(Expect::Value(self.lower_expr(cx, e)?)),
        }
    }

    fn lower_place(&mut self, cx: &mut Cx, e: &ast::Expr) -> R<Place> {
        match &e.kind {
            AK::Name(n) => match self.resolve(cx, n) {
                Ok(Named::Local) => match self.local_mutable(cx, n) {
                    Some(true) => Ok(Place { root: PlaceRoot::Local(n.clone()), path: Vec::new(), span: e.span }),
                    _ => Err(IrError::error(
                        "E-PLACE-ASSIGN",
                        e.span,
                        format!("`{n}` is a `let`: only a `var` or a `state` changes"),
                    )
                    .with_note("declare it with `var` to change it")),
                },
                Ok(Named::Bind(Binding::Decl(DeclRef::State(st)))) => self.state_place(cx, st, e),
                Ok(_) => Err(IrError::error("E-PLACE-ASSIGN", e.span, format!("`{n}` is not a place"))),
                Err(_) => Err(self.unknown_name(cx, n, e.span)),
            },
            AK::Field(recv, name) => {
                if let Some(m) = self.module_of(cx, recv) {
                    return match self.member(m, &name.name, name.span)? {
                        Binding::Decl(DeclRef::State(st)) => self.state_place(cx, st, e),
                        _ => Err(IrError::error("E-PLACE-ASSIGN", e.span, "not a place")),
                    };
                }
                let mut p = self.lower_place(cx, recv)?;
                p.path.push(PlaceStep::Field(name.name.clone()));
                p.span = e.span;
                Ok(p)
            }
            AK::Index(recv, args) if args.len() == 1 && args[0].name.is_none() => {
                let mut p = self.lower_place(cx, recv)?;
                p.path.push(PlaceStep::Index(self.lower_expr(cx, &args[0].value)?));
                p.span = e.span;
                Ok(p)
            }
            _ => Err(IrError::error(
                "E-PLACE-ASSIGN",
                e.span,
                "not a place: a `var` or a `state`, then fields and indexes",
            )),
        }
    }

    fn state_place(&self, cx: &Cx, st: crate::ids::StateId, e: &ast::Expr) -> R<Place> {
        let owner = self.prog.state(st).info.module;
        if cx.body != Body::Rule || owner != cx.module {
            let name = self.prog.qualified(&self.prog.state(st).info);
            return Err(IrError::error(
                "E-STATE-FOREIGN",
                e.span,
                format!("state {name} is written only by rules of {}", self.prog.module(owner).name),
            ));
        }
        Ok(Place { root: PlaceRoot::State(st), path: Vec::new(), span: e.span })
    }
}
