//! Tokens to AST, by recursive descent over the grammar's precedence levels.
//!
//! # Keywords are contextual
//!
//! The reference checker runs Lark's LALR parser with its contextual lexer:
//! in each parser state only the terminals that state can shift are matched.
//! Three consequences, all reproduced here:
//!
//! 1. A word is a keyword only where the grammar can take that keyword:
//!    `match` is a statement at the start of a statement and an ordinary
//!    name inside an expression (`f(match)`), `use` is a name after the first
//!    declaration, `not` is a name right after a comparison operator.
//! 2. Where a keyword can stand but a name cannot (after a complete operand:
//!    `else then where mod band bor bxor shl shr`, and `if` after `else` in a
//!    statement), the keyword matches as a prefix of a longer word, so
//!    `a elseb` is `a else b`. See [`Parser::eat_kw_prefix`].
//! 3. Where a name can stand but a text cannot, `re"x"` is the name `re`
//!    followed by the text `"x"`. See [`Parser::expect_name`].
//!
//! LALR(1) merges states with equal items, so a state's lookahead set is the
//! union over every context that reaches it. Two merges change what a word
//! means, and both are reproduced: `pub` is a keyword right after any path
//! (see `refuse_pub_after_path`), and right after a `value` ends, the words
//! that start a statement are keywords, so a declaration or match arm that
//! follows a value cannot begin with them (see [`Parser::refuse_after_value`]).

mod decl;
mod expr;
mod stmt;

use crate::ast::{Expr, File, Ident};
use crate::diag::Diagnostic;
use crate::lexer::{lex, lex_from};
use crate::span::{FileId, Span};
use crate::token::{LexError, Punct, Token, TokenKind};

pub type PResult<T> = Result<T, Diagnostic>;

/// Parse a whole `.slg` text.
pub fn parse_file(file: FileId, src: &str) -> PResult<File> {
    let toks = lex(file, src);
    let mut p = Parser { file, src, toks, pos: 0, value_end: usize::MAX };
    p.file()
}

/// Parse one expression that spans `src[start..end]` and nothing else: the
/// inside of a `{...}` interpolation. Offsets stay those of `src`.
pub fn parse_expr(file: FileId, src: &str, start: usize, end: usize) -> PResult<Expr> {
    let head = src.get(..end).unwrap_or("");
    let toks = lex_from(file, head, start);
    let mut p = Parser { file, src: head, toks, pos: 0, value_end: usize::MAX };
    let e = p.expr()?;
    if !p.at_eof() {
        return Err(p.expected("the end of the interpolated expression"));
    }
    Ok(e)
}

/// Words that start an expression at the `expr` level. Where a named
/// argument `NAME ":"` could start, these are keywords, not names.
const EXPR_START_KEYWORDS: &[&str] = &["if", "fail", "not", "own", "ref", "mut"];

/// Words that can start a statement: keywords in the state after a `value`.
const VALUE_FOLLOW_KEYWORDS: &[&str] =
    &["let", "var", "return", "assert", "for", "while", "if", "match", "fail", "not", "own", "ref", "mut"];

pub(crate) struct Parser<'a> {
    file: FileId,
    src: &'a str,
    toks: Vec<Token>,
    pos: usize,
    /// Token index right after the last complete `value`.
    value_end: usize,
}

impl<'a> Parser<'a> {
    fn tok(&self) -> Token {
        self.tok_at(0)
    }

    fn tok_at(&self, n: usize) -> Token {
        let last = self.toks.len().saturating_sub(1);
        self.toks
            .get((self.pos + n).min(last))
            .copied()
            .unwrap_or(Token { kind: TokenKind::Eof, span: Span::new(self.file, 0, 0) })
    }

    fn text_of(&self, t: Token) -> &'a str {
        self.src.get(t.span.lo as usize..t.span.hi as usize).unwrap_or("")
    }

    fn bump(&mut self) -> Token {
        let t = self.tok();
        if t.kind != TokenKind::Eof {
            self.pos += 1;
        }
        t
    }

    fn at_eof(&self) -> bool {
        self.tok().kind == TokenKind::Eof
    }

    fn prev_span(&self) -> Span {
        match self.pos.checked_sub(1).and_then(|i| self.toks.get(i)) {
            Some(t) => t.span,
            None => self.tok().span,
        }
    }

    fn span_from(&self, lo: Span) -> Span {
        lo.to(self.prev_span())
    }

    fn is_punct(&self, p: Punct) -> bool {
        self.tok().kind == TokenKind::Punct(p)
    }

    fn eat_punct(&mut self, p: Punct) -> bool {
        if self.is_punct(p) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_punct(&mut self, p: Punct) -> PResult<Span> {
        if self.is_punct(p) { Ok(self.bump().span) } else { Err(self.expected(&format!("`{}`", p.ascii()))) }
    }

    /// The current token is exactly the word `kw`.
    fn is_kw(&self, kw: &str) -> bool {
        let t = self.tok();
        t.kind == TokenKind::Word && self.text_of(t) == kw
    }

    fn eat_kw(&mut self, kw: &str) -> bool {
        if self.is_kw(kw) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Take keyword `kw` where no name can stand: the whole word, or its
    /// first `kw.len()` bytes, after which lexing restarts (consequence 2).
    fn eat_kw_prefix(&mut self, kw: &str) -> bool {
        let t = self.tok();
        if !matches!(t.kind, TokenKind::Word | TokenKind::Text) || !self.text_of(t).starts_with(kw) {
            return false;
        }
        if self.text_of(t).len() == kw.len() {
            self.pos += 1;
            return true;
        }
        let cut = t.span.lo as usize + kw.len();
        self.split_at(cut);
        true
    }

    /// Replace the current token by a word ending at byte `cut`, then re-lex
    /// everything after it.
    fn split_at(&mut self, cut: usize) {
        let t = self.tok();
        let head = Token { kind: TokenKind::Word, span: Span::new(self.file, t.span.lo, crate::span::to_u32(cut)) };
        let tail = lex_from(self.file, self.src, cut);
        self.toks.truncate(self.pos);
        self.toks.push(head);
        self.toks.extend(tail);
        self.pos += 1;
    }

    /// A `NAME` where only a name can stand. Any word is one, keywords
    /// included; a prefixed text `re"x"` gives up its prefix (consequence 3).
    fn expect_name(&mut self) -> PResult<Ident> {
        let t = self.tok();
        match t.kind {
            TokenKind::Word => {
                self.pos += 1;
                Ok(Ident { name: self.text_of(t).to_string(), span: t.span })
            }
            TokenKind::Text if !self.text_of(t).starts_with('"') => {
                let cut = t.span.lo as usize + self.text_of(t).find('"').unwrap_or(0);
                self.split_at(cut);
                let span = self.prev_span();
                Ok(Ident { name: self.src.get(span.lo as usize..span.hi as usize).unwrap_or("").to_string(), span })
            }
            _ => Err(self.expected("a name")),
        }
    }

    /// See the module documentation: after a value, statement keywords are
    /// keywords even where a statement cannot start.
    fn refuse_after_value(&self, what: &str) -> PResult<()> {
        let t = self.tok();
        if self.pos == self.value_end && t.kind == TokenKind::Word && VALUE_FOLLOW_KEYWORDS.contains(&self.text_of(t)) {
            return Err(self.expected(what));
        }
        Ok(())
    }

    fn describe(&self, t: Token) -> String {
        match t.kind {
            TokenKind::Eof => "end of input".to_string(),
            TokenKind::Word => format!("name `{}`", self.text_of(t)),
            TokenKind::Num => format!("number `{}`", self.text_of(t)),
            TokenKind::Text => format!("text {}", self.text_of(t)),
            TokenKind::Punct(_) | TokenKind::Error(_) => format!("`{}`", self.text_of(t)),
        }
    }

    fn expected(&self, what: &str) -> Diagnostic {
        let t = self.tok();
        match t.kind {
            TokenKind::Error(LexError::UnexpectedChar) => {
                let c = self.text_of(t).chars().next().unwrap_or('?');
                return Diagnostic::lex(t.span, format!("unexpected character {c:?}"));
            }
            TokenKind::Error(LexError::UnterminatedText) => {
                return Diagnostic::lex(t.span, "text literal does not end on its line");
            }
            _ => {}
        }
        Diagnostic::parse(t.span, format!("expected {what}, found {}", self.describe(t)))
    }

    fn file(&mut self) -> PResult<File> {
        let lo = self.tok().span;
        let mut uses = Vec::new();
        while self.is_kw("use") {
            uses.push(self.use_decl()?);
        }
        let mut decls = Vec::new();
        while !self.at_eof() {
            decls.push(self.decl()?);
        }
        Ok(File { uses, decls, span: lo.to(self.tok().span) })
    }
}

#[cfg(test)]
mod tests;
