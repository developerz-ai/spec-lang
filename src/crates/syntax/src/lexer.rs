//! Text to tokens. Implements the terminals of `docs/idea/25-grammar.md`:
//! `NUM`, `TEXT`, `NAME`, the symbols, `COMMENT` and white space.

use crate::span::{FileId, Span, to_u32};
use crate::token::{LexError, PUNCTS, Token, TokenKind};
use crate::unicode::{is_name_continue, is_name_start, is_space};

/// Lex `src` from byte `start` to its end. Offsets in the tokens are offsets
/// into `src`, so a caller re-lexing the tail of a file keeps its spans.
/// Never fails: unmatched text becomes a [`TokenKind::Error`] token.
pub fn lex_from(file: FileId, src: &str, start: usize) -> Vec<Token> {
    let mut lx = Lexer { file, src, pos: start, toks: Vec::new() };
    lx.run();
    lx.toks
}

/// Lex a whole text.
pub fn lex(file: FileId, src: &str) -> Vec<Token> {
    lex_from(file, src, 0)
}

struct Lexer<'a> {
    file: FileId,
    src: &'a str,
    pos: usize,
    toks: Vec<Token>,
}

impl Lexer<'_> {
    fn rest(&self) -> &str {
        self.src.get(self.pos..).unwrap_or("")
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.rest().chars().nth(n)
    }

    fn span(&self, lo: usize) -> Span {
        Span::new(self.file, to_u32(lo), to_u32(self.pos))
    }

    fn push(&mut self, kind: TokenKind, lo: usize) {
        let span = self.span(lo);
        self.toks.push(Token { kind, span });
    }

    fn run(&mut self) {
        while let Some(c) = self.peek() {
            let lo = self.pos;
            if is_space(c) {
                self.pos += c.len_utf8();
            } else if self.rest().starts_with("//") {
                self.pos += self.rest().find('\n').unwrap_or(self.rest().len());
            } else if c.is_ascii_digit() {
                self.number();
                self.push(TokenKind::Num, lo);
            } else if c == '"' {
                if self.text().is_ok() {
                    self.push(TokenKind::Text, lo);
                } else {
                    self.push(TokenKind::Error(LexError::UnterminatedText), lo);
                }
            } else if is_name_start(c) {
                self.word_or_prefixed_text(lo);
            } else if let Some(&(spelling, p)) = PUNCTS.iter().find(|(s, _)| self.rest().starts_with(s)) {
                self.pos += spelling.len();
                self.push(TokenKind::Punct(p), lo);
            } else {
                self.pos += c.len_utf8();
                self.push(TokenKind::Error(LexError::UnexpectedChar), lo);
            }
        }
        let end = self.span(self.pos);
        self.toks.push(Token { kind: TokenKind::Eof, span: end });
    }

    fn eat_while(&mut self, f: impl Fn(char) -> bool) -> usize {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if !f(c) {
                break;
            }
            self.pos += c.len_utf8();
        }
        self.pos - start
    }

    /// `0x[0-9a-fA-F_]+ | [0-9][0-9_]*(\.[0-9]+)?(e-?[0-9]+)?([°µμ]?[A-Za-z][A-Za-z0-9]*|%)?`,
    /// alternatives tried in order as Python's `re` does.
    fn number(&mut self) {
        if self.rest().starts_with("0x") && self.peek_at(2).is_some_and(|c| c.is_ascii_hexdigit() || c == '_') {
            self.pos += 2;
            self.eat_while(|c| c.is_ascii_hexdigit() || c == '_');
            return;
        }
        self.pos += 1;
        self.eat_while(|c| c.is_ascii_digit() || c == '_');
        if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
            self.eat_while(|c| c.is_ascii_digit());
        }
        if self.peek() == Some('e') {
            let digit_at = if self.peek_at(1) == Some('-') { 2 } else { 1 };
            if self.peek_at(digit_at).is_some_and(|c| c.is_ascii_digit()) {
                self.pos += digit_at;
                self.eat_while(|c| c.is_ascii_digit());
            }
        }
        if self.peek() == Some('%') {
            self.pos += 1;
            return;
        }
        let (mark_len, letter_at) = match self.peek() {
            Some(c @ ('°' | 'µ' | 'μ')) => (c.len_utf8(), 1),
            _ => (0, 0),
        };
        if self.peek_at(letter_at).is_some_and(|c| c.is_ascii_alphabetic()) {
            self.pos += mark_len;
            self.eat_while(|c| c.is_ascii_alphanumeric());
        }
    }

    /// `"([^"\\\n]|\\.)*"` from the opening quote at `self.pos`. On failure
    /// the cursor stops at the end of the line.
    fn text(&mut self) -> Result<(), ()> {
        self.pos += 1;
        loop {
            match self.peek() {
                Some('"') => {
                    self.pos += 1;
                    return Ok(());
                }
                Some('\\') if self.peek_at(1).is_some_and(|c| c != '\n') => {
                    self.pos += 1;
                    let c = self.peek().unwrap_or('\\');
                    self.pos += c.len_utf8();
                }
                Some(c) if c != '\n' && c != '\\' => self.pos += c.len_utf8(),
                _ => return Err(()),
            }
        }
    }

    /// A `NAME`, or `TEXT` when the word is `[a-z]+` directly followed by a
    /// closed text literal (`re"^a+$"`). The parser splits that back into a
    /// name and a text where only a name can stand.
    fn word_or_prefixed_text(&mut self, lo: usize) {
        self.eat_while(is_name_continue);
        let word = self.src.get(lo..self.pos).unwrap_or("");
        if self.peek() == Some('"') && word.bytes().all(|b| b.is_ascii_lowercase()) {
            let save = self.pos;
            if self.text().is_ok() {
                self.push(TokenKind::Text, lo);
                return;
            }
            self.pos = save;
        }
        self.push(TokenKind::Word, lo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Punct;

    fn kinds(src: &str) -> Vec<(TokenKind, String)> {
        lex(FileId(0), src)
            .into_iter()
            .map(|t| (t.kind, src[t.span.lo as usize..t.span.hi as usize].to_string()))
            .collect()
    }

    #[test]
    fn numbers_take_units_like_the_reference_regex() {
        let k = kinds("10ms 1..=n 0xg 2em 1e5m 80°C 70% 5u64");
        let nums: Vec<_> = k.iter().filter(|(k, _)| *k == TokenKind::Num).map(|(_, s)| s.as_str()).collect();
        assert_eq!(nums, ["10ms", "1", "0xg", "2em", "1e5m", "80°C", "70%", "5u64"]);
    }

    #[test]
    fn prefixed_text_is_one_token() {
        let k = kinds(r#"re"a\"b" Re"x""#);
        assert_eq!(k[0], (TokenKind::Text, r#"re"a\"b""#.to_string()));
        assert_eq!(k[1], (TokenKind::Word, "Re".to_string()));
        assert_eq!(k[2].0, TokenKind::Text);
    }

    #[test]
    fn unicode_operators_are_their_ascii_twins() {
        let k = kinds("a ≤ b → c ∈ d");
        assert_eq!(k[1].0, TokenKind::Punct(Punct::Le));
        assert_eq!(k[3].0, TokenKind::Punct(Punct::Arrow));
        assert_eq!(k[5].0, TokenKind::Punct(Punct::Elem));
    }

    #[test]
    fn comments_and_unterminated_text() {
        assert_eq!(kinds("a // b\nc").len(), 3);
        let err = |src| lex(FileId(0), src).iter().any(|t| matches!(t.kind, TokenKind::Error(_)));
        assert!(err("\"abc\nd\""));
        assert!(err("a ' b"));
    }
}
