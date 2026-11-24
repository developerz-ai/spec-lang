//! Expressions, one function per precedence level of the grammar, loosest first.

use super::{PResult, Parser};
use crate::ast::{BinOp, CmpOp, Expr, ExprKind, Ident, LambdaBody, TextLit, UnOp};
use crate::span::Span;
use crate::token::{Punct, TokenKind};

impl Parser<'_> {
    /// `?expr: edge | edge "=>" expr | edge "=>" "{" stmt* "}"`.
    pub(crate) fn expr(&mut self) -> PResult<Expr> {
        let e = self.edge()?;
        self.expr_rest(e)
    }

    pub(super) fn expr_rest(&mut self, e: Expr) -> PResult<Expr> {
        if !self.eat_punct(Punct::FatArrow) {
            return Ok(e);
        }
        let body = if self.is_punct(Punct::LBrace) {
            LambdaBody::Block(self.block()?)
        } else {
            LambdaBody::Expr(Box::new(self.expr()?))
        };
        let span = self.span_from(e.span);
        Ok(Expr::new(ExprKind::Lambda(Box::new(e), body), span))
    }

    /// `?edge: alt | edge ARROW alt`.
    fn edge(&mut self) -> PResult<Expr> {
        let e = self.alt()?;
        self.edge_rest(e)
    }

    pub(super) fn edge_rest(&mut self, mut e: Expr) -> PResult<Expr> {
        while self.eat_punct(Punct::Arrow) {
            let r = self.alt()?;
            let span = e.span.to(r.span);
            e = Expr::new(ExprKind::Arrow(Box::new(e), Box::new(r)), span);
        }
        Ok(e)
    }

    /// `?alt: wexpr | alt "else" last | cond | fail`.
    fn alt(&mut self) -> PResult<Expr> {
        let e = self.last()?;
        self.alt_rest(e)
    }

    pub(super) fn alt_rest(&mut self, mut e: Expr) -> PResult<Expr> {
        while self.eat_kw_prefix("else") {
            let r = self.last()?;
            let span = e.span.to(r.span);
            e = Expr::new(ExprKind::Else(Box::new(e), Box::new(r)), span);
        }
        Ok(e)
    }

    /// `?last: wexpr | fail | cond`.
    fn last(&mut self) -> PResult<Expr> {
        if self.is_kw("if") {
            let lo = self.bump().span;
            let c = self.wexpr()?;
            return self.cond_rest(lo, c);
        }
        if self.is_kw("fail") {
            let lo = self.bump().span;
            let e = self.wexpr()?;
            let span = lo.to(e.span);
            return Ok(Expr::new(ExprKind::Fail(Box::new(e)), span));
        }
        self.wexpr()
    }

    /// `cond: "if" wexpr "then" wexpr "else" last`, after the condition.
    pub(super) fn cond_rest(&mut self, lo: Span, c: Expr) -> PResult<Expr> {
        if !self.eat_kw_prefix("then") {
            return Err(self.expected("`then`"));
        }
        let a = self.wexpr()?;
        if !self.eat_kw_prefix("else") {
            return Err(self.expected("`else`"));
        }
        let b = self.last()?;
        let span = lo.to(b.span);
        Ok(Expr::new(ExprKind::Cond(Box::new(c), Box::new(a), Box::new(b)), span))
    }

    /// `?wexpr: imp | wexpr "where" imp`.
    pub(super) fn wexpr(&mut self) -> PResult<Expr> {
        let mut e = self.imp()?;
        while self.eat_kw_prefix("where") {
            let r = self.imp()?;
            e = bin(BinOp::Where, e, r);
        }
        Ok(e)
    }

    /// `?imp: or_ | or_ IMPLIES imp`.
    fn imp(&mut self) -> PResult<Expr> {
        let e = self.or()?;
        if self.eat_kw("implies") || self.eat_punct(Punct::ImpliesSym) {
            let r = self.imp()?;
            return Ok(bin(BinOp::Implies, e, r));
        }
        Ok(e)
    }

    /// `?or_: and_ | or_ OR and_`.
    fn or(&mut self) -> PResult<Expr> {
        let mut e = self.and()?;
        while self.eat_kw("or") || self.eat_punct(Punct::OrSym) {
            let r = self.and()?;
            e = bin(BinOp::Or, e, r);
        }
        Ok(e)
    }

    /// `?and_: not_ | and_ AND not_`.
    fn and(&mut self) -> PResult<Expr> {
        let mut e = self.not()?;
        while self.eat_kw("and") || self.eat_punct(Punct::AndSym) {
            let r = self.not()?;
            e = bin(BinOp::And, e, r);
        }
        Ok(e)
    }

    /// `?not_: cmp | NOT not_`.
    fn not(&mut self) -> PResult<Expr> {
        if self.is_kw("not") || self.is_punct(Punct::NotSym) {
            let lo = self.bump().span;
            let e = self.not()?;
            let span = lo.to(e.span);
            return Ok(Expr::new(ExprKind::Unary(UnOp::Not, Box::new(e)), span));
        }
        self.cmp()
    }

    /// `?cmp: bar ((CMP|IN) bar)*`.
    fn cmp(&mut self) -> PResult<Expr> {
        let first = self.bar()?;
        let mut rest = Vec::new();
        while let Some(op) = self.cmp_op() {
            rest.push((op, self.bar()?));
        }
        if rest.is_empty() {
            return Ok(first);
        }
        let span = rest.last().map_or(first.span, |(_, e)| first.span.to(e.span));
        Ok(Expr::new(ExprKind::Compare(Box::new(first), rest), span))
    }

    fn cmp_op(&mut self) -> Option<CmpOp> {
        if self.eat_kw("in") {
            return Some(CmpOp::In);
        }
        let op = match self.tok().kind {
            TokenKind::Punct(Punct::EqEq) => CmpOp::Eq,
            TokenKind::Punct(Punct::NotEq) => CmpOp::Ne,
            TokenKind::Punct(Punct::Lt) => CmpOp::Lt,
            TokenKind::Punct(Punct::Le) => CmpOp::Le,
            TokenKind::Punct(Punct::Gt) => CmpOp::Gt,
            TokenKind::Punct(Punct::Ge) => CmpOp::Ge,
            TokenKind::Punct(Punct::Elem) => CmpOp::In,
            _ => return None,
        };
        self.bump();
        Some(op)
    }

    /// `?bar: bits | bar "|" bits`.
    pub(super) fn bar(&mut self) -> PResult<Expr> {
        let mut e = self.bits()?;
        while self.eat_punct(Punct::Bar) {
            let r = self.bits()?;
            e = bin(BinOp::Bar, e, r);
        }
        Ok(e)
    }

    /// `?bits: range | range ("band"|"bor"|"bxor"|"shl"|"shr") range`.
    fn bits(&mut self) -> PResult<Expr> {
        let e = self.range()?;
        for (kw, op) in [
            ("band", BinOp::Band),
            ("bor", BinOp::Bor),
            ("bxor", BinOp::Bxor),
            ("shl", BinOp::Shl),
            ("shr", BinOp::Shr),
        ] {
            if self.eat_kw_prefix(kw) {
                let r = self.range()?;
                return Ok(bin(op, e, r));
            }
        }
        Ok(e)
    }

    /// `?range: sum | sum (RNG|PM) sum`.
    fn range(&mut self) -> PResult<Expr> {
        let e = self.sum()?;
        let op = match self.tok().kind {
            TokenKind::Punct(Punct::RangeIncl) => BinOp::RangeIncl,
            TokenKind::Punct(Punct::RangeExcl) => BinOp::RangeExcl,
            TokenKind::Punct(Punct::PlusMinus) => BinOp::PlusMinus,
            _ => return Ok(e),
        };
        self.bump();
        let r = self.sum()?;
        Ok(bin(op, e, r))
    }

    /// `?sum: prod | sum SIGN prod`.
    fn sum(&mut self) -> PResult<Expr> {
        let mut e = self.prod()?;
        loop {
            let op = if self.eat_punct(Punct::Plus) {
                BinOp::Add
            } else if self.eat_punct(Punct::Minus) {
                BinOp::Sub
            } else {
                return Ok(e);
            };
            let r = self.prod()?;
            e = bin(op, e, r);
        }
    }

    /// `?prod: un | prod (MULOP|"mod") un`.
    fn prod(&mut self) -> PResult<Expr> {
        let mut e = self.un()?;
        loop {
            let op = if self.eat_punct(Punct::Star) {
                BinOp::Mul
            } else if self.eat_punct(Punct::Slash) {
                BinOp::Div
            } else if self.eat_kw_prefix("mod") {
                BinOp::Mod
            } else {
                return Ok(e);
            };
            let r = self.un()?;
            e = bin(op, e, r);
        }
    }

    /// `?un: pow | SIGN un | ("own"|"ref"|"mut") un`.
    fn un(&mut self) -> PResult<Expr> {
        let op = if self.is_punct(Punct::Minus) {
            Some(UnOp::Neg)
        } else if self.is_punct(Punct::Plus) {
            Some(UnOp::Pos)
        } else if self.is_kw("own") {
            Some(UnOp::Own)
        } else if self.is_kw("ref") {
            Some(UnOp::Ref)
        } else if self.is_kw("mut") {
            Some(UnOp::Mut)
        } else {
            None
        };
        if let Some(op) = op {
            let lo = self.bump().span;
            let e = self.un()?;
            let span = lo.to(e.span);
            return Ok(Expr::new(ExprKind::Unary(op, Box::new(e)), span));
        }
        self.pow()
    }

    /// `?pow: post | post "^" un | post SUP`.
    fn pow(&mut self) -> PResult<Expr> {
        let e = self.post()?;
        if self.eat_punct(Punct::Caret) {
            let r = self.un()?;
            return Ok(bin(BinOp::Pow, e, r));
        }
        for (p, n) in [(Punct::Sup2, 2), (Punct::Sup3, 3)] {
            if self.eat_punct(p) {
                let span = self.span_from(e.span);
                return Ok(Expr::new(ExprKind::Sup(Box::new(e), n), span));
            }
        }
        Ok(e)
    }

    /// `?post: atom | post "." NAME | post "(" args? ")" | post "[" args "]"`.
    fn post(&mut self) -> PResult<Expr> {
        let mut e = self.atom()?;
        loop {
            if self.eat_punct(Punct::Dot) {
                let name = self.expect_name()?;
                let span = e.span.to(name.span);
                e = Expr::new(ExprKind::Field(Box::new(e), name), span);
            } else if self.eat_punct(Punct::LParen) {
                let args = if self.is_punct(Punct::RParen) { Vec::new() } else { self.args()? };
                self.expect_punct(Punct::RParen)?;
                let span = self.span_from(e.span);
                e = Expr::new(ExprKind::Call(Box::new(e), args), span);
            } else if self.eat_punct(Punct::LBracket) {
                let args = self.args()?;
                self.expect_punct(Punct::RBracket)?;
                let span = self.span_from(e.span);
                e = Expr::new(ExprKind::Index(Box::new(e), args), span);
            } else {
                return Ok(e);
            }
        }
    }

    /// `?atom: NAME | NUM | TEXT | "?" NAME | "(" args? ")" | "[" args? "]"`.
    fn atom(&mut self) -> PResult<Expr> {
        let t = self.tok();
        match t.kind {
            TokenKind::Word => {
                self.bump();
                Ok(Expr::new(ExprKind::Name(self.text_of(t).to_string()), t.span))
            }
            TokenKind::Num => {
                self.bump();
                Ok(Expr::new(ExprKind::Num(self.text_of(t).to_string()), t.span))
            }
            TokenKind::Text => {
                let lit = self.text_lit()?;
                Ok(Expr::new(ExprKind::Text(lit), t.span))
            }
            TokenKind::Punct(Punct::Question) => {
                self.bump();
                let name: Ident = self.expect_name()?;
                Ok(Expr::new(ExprKind::Hole(name), self.span_from(t.span)))
            }
            TokenKind::Punct(Punct::LParen) => {
                self.bump();
                let (args, trailing_comma) =
                    if self.is_punct(Punct::RParen) { (Vec::new(), false) } else { self.args_with_trailing()? };
                self.expect_punct(Punct::RParen)?;
                Ok(Expr::new(ExprKind::Group { args, trailing_comma }, self.span_from(t.span)))
            }
            TokenKind::Punct(Punct::LBracket) => {
                self.bump();
                let args = if self.is_punct(Punct::RBracket) { Vec::new() } else { self.args()? };
                self.expect_punct(Punct::RBracket)?;
                Ok(Expr::new(ExprKind::List(args), self.span_from(t.span)))
            }
            _ => Err(self.expected("an expression")),
        }
    }

    /// The `TEXT` token at the cursor.
    pub(super) fn text_lit(&mut self) -> PResult<TextLit> {
        let t = self.tok();
        if t.kind != TokenKind::Text {
            return Err(self.expected("a text literal"));
        }
        self.bump();
        let s = self.text_of(t);
        let q = s.find('"').unwrap_or(0);
        let prefix = if q == 0 { None } else { s.get(..q).map(str::to_string) };
        let raw = s.get(q + 1..s.len().saturating_sub(1)).unwrap_or("").to_string();
        let raw_offset = t.span.lo + crate::span::to_u32(q + 1);
        Ok(TextLit { prefix, raw, raw_offset, span: t.span })
    }
}

fn bin(op: BinOp, l: Expr, r: Expr) -> Expr {
    let span = l.span.to(r.span);
    Expr::new(ExprKind::Binary(op, Box::new(l), Box::new(r)), span)
}
