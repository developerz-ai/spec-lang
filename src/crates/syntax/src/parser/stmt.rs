//! Statements, blocks and `match`.

use super::{PResult, Parser};
use crate::ast::{Arm, Block, Else, Expr, ExprKind, IfStmt, Match, Stmt, StmtKind};
use crate::token::Punct;

impl Parser<'_> {
    /// `"{" stmt* "}"`.
    pub(super) fn block(&mut self) -> PResult<Block> {
        let lo = self.expect_punct(Punct::LBrace)?;
        let mut stmts = Vec::new();
        while !self.is_punct(Punct::RBrace) {
            if self.at_eof() {
                return Err(self.expected("a statement or `}`"));
            }
            stmts.push(self.stmt()?);
        }
        self.bump();
        Ok(Block { stmts, span: self.span_from(lo) })
    }

    /// `match: "match" expr "{" arm* "}"`.
    pub(super) fn match_expr(&mut self) -> PResult<Match> {
        let lo = self.bump().span;
        let scrutinee = self.expr()?;
        self.expect_punct(Punct::LBrace)?;
        let mut arms = Vec::new();
        while !self.is_punct(Punct::RBrace) {
            if self.at_eof() {
                return Err(self.expected("a match arm or `}`"));
            }
            self.refuse_after_value("a match arm or `}`")?;
            let alo = self.tok().span;
            let pattern = self.bar()?;
            self.expect_punct(Punct::FatArrow)?;
            let value = self.value()?;
            arms.push(Arm { pattern, value, span: self.span_from(alo) });
        }
        self.bump();
        Ok(Match { scrutinee, arms, span: self.span_from(lo) })
    }

    fn stmt(&mut self) -> PResult<Stmt> {
        let lo = self.tok().span;
        let kind = self.stmt_kind()?;
        Ok(Stmt { kind, span: self.span_from(lo) })
    }

    fn stmt_kind(&mut self) -> PResult<StmtKind> {
        if self.is_kw("let") || self.is_kw("var") {
            let mutable = self.is_kw("var");
            self.bump();
            let name = self.expect_name()?;
            let ty = if self.eat_punct(Punct::Colon) { Some(self.wexpr()?) } else { None };
            self.expect_punct(Punct::Eq)?;
            let value = self.value()?;
            return Ok(StmtKind::Let { mutable, name, ty, value });
        }
        if self.eat_kw("return") {
            if self.eat_punct(Punct::Semi) {
                return Ok(StmtKind::Return(None));
            }
            return Ok(StmtKind::Return(Some(self.value()?)));
        }
        if self.eat_kw("assert") {
            let e = self.expr()?;
            self.expect_punct(Punct::Semi)?;
            return Ok(StmtKind::Assert(e));
        }
        if self.eat_kw("for") {
            let var = self.expect_name()?;
            if !(self.eat_kw("in") || self.eat_punct(Punct::Elem)) {
                return Err(self.expected("`in`"));
            }
            let iter = self.expr()?;
            let body = self.block()?;
            return Ok(StmtKind::For { var, iter, body });
        }
        if self.eat_kw("while") {
            let cond = self.expr()?;
            let body = self.block()?;
            return Ok(StmtKind::While { cond, body });
        }
        if self.is_kw("match") {
            return Ok(StmtKind::Match(self.match_expr()?));
        }
        let e = if self.is_kw("if") {
            match self.if_stmt_or_cond()? {
                IfOrCond::Stmt(s) => return Ok(StmtKind::If(s)),
                IfOrCond::Cond(e) => e,
            }
        } else {
            self.expr()?
        };
        if self.eat_punct(Punct::Eq) {
            let value = self.value()?;
            return Ok(StmtKind::Assign { place: e, value });
        }
        self.expect_punct(Punct::Semi)?;
        Ok(StmtKind::Expr(e))
    }

    /// At statement start `if` begins either `ifs` (`if e { }`) or the
    /// expression `if c then a else b`. The LALR parser decides after the
    /// condition; so does this: parse the wider `expr`, then look.
    fn if_stmt_or_cond(&mut self) -> PResult<IfOrCond> {
        let lo = self.bump().span;
        let cond = self.expr()?;
        if self.is_punct(Punct::LBrace) {
            return Ok(IfOrCond::Stmt(self.if_rest(lo, cond)?));
        }
        if !is_wexpr_level(&cond) {
            return Err(self.expected("`{`"));
        }
        let e = self.cond_rest(lo, cond)?;
        let e = self.alt_rest(e)?;
        let e = self.edge_rest(e)?;
        let e = self.expr_rest(e)?;
        Ok(IfOrCond::Cond(e))
    }

    /// `ifs: "if" expr "{" stmt* "}" ("else" (ifs | "{" stmt* "}"))?` after the condition.
    fn if_rest(&mut self, lo: crate::span::Span, cond: Expr) -> PResult<IfStmt> {
        let then = self.block()?;
        let els = if self.eat_kw("else") {
            if self.eat_kw_prefix("if") {
                let ilo = self.prev_span();
                let c = self.expr()?;
                Some(Else::If(Box::new(self.if_rest(ilo, c)?)))
            } else if self.is_punct(Punct::LBrace) {
                Some(Else::Block(self.block()?))
            } else {
                return Err(self.expected("`if` or `{`"));
            }
        } else {
            None
        };
        Ok(IfStmt { cond, then, els, span: self.span_from(lo) })
    }
}

enum IfOrCond {
    Stmt(IfStmt),
    Cond(Expr),
}

/// Whether an expression parsed as `expr` is also a `wexpr`: none of the
/// forms of the levels above `wexpr` at its top.
pub(super) fn is_wexpr_level(e: &Expr) -> bool {
    !matches!(
        e.kind,
        ExprKind::Lambda(..) | ExprKind::Arrow(..) | ExprKind::Else(..) | ExprKind::Cond(..) | ExprKind::Fail(..)
    )
}
