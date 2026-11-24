//! Declarations: `use`, `decl`, notes, parameters, tails and bodies.

use super::{EXPR_START_KEYWORDS, PResult, Parser};
use crate::ast::{Arg, Body, Decl, Note, Param, Path, ResultSep, ResultType, Tail, Use, Value};
use crate::token::{Punct, TokenKind};

impl Parser<'_> {
    /// `use: "use" path ";"`.
    pub(super) fn use_decl(&mut self) -> PResult<Use> {
        let lo = self.bump().span;
        let path = self.path()?;
        self.expect_punct(Punct::Semi)?;
        Ok(Use { path, span: self.span_from(lo) })
    }

    pub(super) fn path(&mut self) -> PResult<Path> {
        let first = self.expect_name()?;
        let lo = first.span;
        let mut segments = vec![first];
        while self.eat_punct(Punct::Dot) {
            segments.push(self.expect_name()?);
        }
        Ok(Path { segments, span: self.span_from(lo) })
    }

    /// In the LALR automaton the state after a `path` is shared by `use`,
    /// notes and declarations, so `pub` (which may follow a note) is a keyword
    /// right after any path, and a name `pub` there does not parse.
    fn refuse_pub_after_path(&self) -> PResult<()> {
        if self.is_kw("pub") { Err(self.expected("a name")) } else { Ok(()) }
    }

    /// `decl: note* "pub"? path NAME generics? params? result? tail`.
    pub(super) fn decl(&mut self) -> PResult<Decl> {
        self.refuse_after_value("a declaration")?;
        let lo = self.tok().span;
        let mut notes = Vec::new();
        while self.is_punct(Punct::At) {
            notes.push(self.note()?);
        }
        let is_pub = self.eat_kw("pub");
        let kind = self.path()?;
        self.refuse_pub_after_path()?;
        let name = self.expect_name()?;
        let generics = if self.is_punct(Punct::LBracket) { Some(self.generics()?) } else { None };
        let params = if self.is_punct(Punct::LParen) { Some(self.params()?) } else { None };
        let result = if self.eat_punct(Punct::Colon) {
            Some(ResultType { sep: ResultSep::Colon, ty: self.wexpr()? })
        } else if self.eat_punct(Punct::Arrow) {
            Some(ResultType { sep: ResultSep::Arrow, ty: self.wexpr()? })
        } else {
            None
        };
        let tail = self.tail()?;
        Ok(Decl { notes, is_pub, kind, name, generics, params, result, tail, span: self.span_from(lo) })
    }

    /// `note: "@" path ("(" args? ")")?`.
    fn note(&mut self) -> PResult<Note> {
        let lo = self.bump().span;
        let path = self.path()?;
        let args = if self.eat_punct(Punct::LParen) {
            let args = if self.is_punct(Punct::RParen) { Vec::new() } else { self.args()? };
            self.expect_punct(Punct::RParen)?;
            Some(args)
        } else {
            None
        };
        Ok(Note { path, args, span: self.span_from(lo) })
    }

    /// `generics: "[" param ("," param)* ","? "]"`.
    fn generics(&mut self) -> PResult<Vec<Param>> {
        self.bump();
        let mut out = vec![self.param()?];
        while self.eat_punct(Punct::Comma) {
            if self.is_punct(Punct::RBracket) {
                break;
            }
            out.push(self.param()?);
        }
        self.expect_punct(Punct::RBracket)?;
        Ok(out)
    }

    /// `params: "(" (param ("," param)* ","?)? ")"`.
    fn params(&mut self) -> PResult<Vec<Param>> {
        self.bump();
        let mut out = Vec::new();
        if !self.is_punct(Punct::RParen) {
            out.push(self.param()?);
            while self.eat_punct(Punct::Comma) {
                if self.is_punct(Punct::RParen) {
                    break;
                }
                out.push(self.param()?);
            }
        }
        self.expect_punct(Punct::RParen)?;
        Ok(out)
    }

    /// `param: note* NAME (":" expr)? ("=" expr)?`.
    fn param(&mut self) -> PResult<Param> {
        let lo = self.tok().span;
        let mut notes = Vec::new();
        while self.is_punct(Punct::At) {
            notes.push(self.note()?);
        }
        if !notes.is_empty() {
            self.refuse_pub_after_path()?;
        }
        let name = self.expect_name()?;
        let ty = if self.eat_punct(Punct::Colon) { Some(self.expr()?) } else { None };
        let default = if self.eat_punct(Punct::Eq) { Some(self.expr()?) } else { None };
        Ok(Param { notes, name, ty, default, span: self.span_from(lo) })
    }

    /// `args: arg ("," arg)* ","?`; the caller has checked one can start.
    pub(super) fn args(&mut self) -> PResult<Vec<Arg>> {
        Ok(self.args_with_trailing()?.0)
    }

    pub(super) fn args_with_trailing(&mut self) -> PResult<(Vec<Arg>, bool)> {
        let mut out = vec![self.arg()?];
        let mut trailing = false;
        while self.eat_punct(Punct::Comma) {
            if self.is_punct(Punct::RParen) || self.is_punct(Punct::RBracket) {
                trailing = true;
                break;
            }
            out.push(self.arg()?);
        }
        Ok((out, trailing))
    }

    /// `arg: (NAME ":")? expr`.
    fn arg(&mut self) -> PResult<Arg> {
        let lo = self.tok().span;
        let t = self.tok();
        let named = t.kind == TokenKind::Word
            && self.tok_at(1).kind == TokenKind::Punct(Punct::Colon)
            && !EXPR_START_KEYWORDS.contains(&self.text_of(t));
        let name = if named {
            let n = self.expect_name()?;
            self.bump();
            Some(n)
        } else {
            None
        };
        let value = self.expr()?;
        Ok(Arg { name, value, span: self.span_from(lo) })
    }

    /// `tail: ";" | children | "=" body`.
    fn tail(&mut self) -> PResult<Tail> {
        if self.eat_punct(Punct::Semi) {
            Ok(Tail::Semi)
        } else if self.eat_punct(Punct::LBrace) {
            let mut decls = Vec::new();
            while !self.is_punct(Punct::RBrace) {
                if self.at_eof() {
                    return Err(self.expected("a declaration or `}`"));
                }
                decls.push(self.decl()?);
            }
            self.bump();
            Ok(Tail::Children(decls))
        } else if self.eat_punct(Punct::Eq) {
            Ok(Tail::Body(self.body()?))
        } else {
            Err(self.expected("`;`, `{` or `=`"))
        }
    }

    /// `body: value | "machine" "{" stmt* "}" | "foreign" NAME TEXT ";"`.
    fn body(&mut self) -> PResult<Body> {
        if self.eat_kw("machine") {
            return Ok(Body::Machine(self.block()?));
        }
        if self.eat_kw("foreign") {
            let lang = self.expect_name()?;
            let symbol = self.text_lit()?;
            self.expect_punct(Punct::Semi)?;
            return Ok(Body::Foreign { lang, symbol });
        }
        Ok(Body::Value(self.value()?))
    }

    /// `value: expr ";" | blocky`.
    pub(super) fn value(&mut self) -> PResult<Value> {
        let v = if self.is_punct(Punct::LBrace) {
            Value::Block(self.block()?)
        } else if self.is_kw("match") {
            Value::Match(self.match_expr()?)
        } else {
            let e = self.expr()?;
            self.expect_punct(Punct::Semi)?;
            Value::Expr(e)
        };
        self.value_end = self.pos;
        Ok(v)
    }
}
