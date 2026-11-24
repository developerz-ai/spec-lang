// Projected by hand from note @store: "Accepted on read", the JSON under the schema (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

use std::borrow::Cow;

use super::Unreadable;
use crate::model::TaskNo;

/// A strict JSON reader that is asked for what the schema expects next, so no
/// tree of values is ever built and a string without escapes is a slice of
/// the input. Anything the schema does not expect is an error where it stands,
/// which is why there is no code here to skip a value.
pub(super) struct Cursor<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Cursor<'a> {
    /// JSON is UTF-8; checking once here lets every later slice be a `&str`.
    pub fn new(bytes: &'a [u8]) -> Result<Self, Unreadable> {
        let text = std::str::from_utf8(bytes).map_err(|_| Unreadable::NotUtf8)?;
        Ok(Self { text, at: 0 })
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn skip_white_space(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.peek() {
            self.at += 1;
        }
    }

    /// White space, then `byte` if it is next.
    fn eat(&mut self, byte: u8) -> bool {
        self.skip_white_space();
        let found = self.peek() == Some(byte);
        if found {
            self.at += 1;
        }
        found
    }

    /// White space, then exactly `byte`.
    fn expect(&mut self, byte: u8) -> Result<(), Unreadable> {
        if self.eat(byte) {
            Ok(())
        } else {
            Err(Unreadable::Syntax(self.at))
        }
    }

    /// Nothing but white space is left.
    pub fn end(&mut self) -> Result<(), Unreadable> {
        self.skip_white_space();
        if self.at == self.text.len() {
            Ok(())
        } else {
            Err(Unreadable::Syntax(self.at))
        }
    }

    /// An object; `member` is called at each value with its key and must read that value.
    pub fn object(
        &mut self,
        mut member: impl FnMut(&mut Self, &str) -> Result<(), Unreadable>,
    ) -> Result<(), Unreadable> {
        self.expect(b'{')?;
        if self.eat(b'}') {
            return Ok(());
        }
        loop {
            let key = self.string()?;
            self.expect(b':')?;
            member(self, &key)?;
            if !self.eat(b',') {
                return self.expect(b'}');
            }
        }
    }

    /// An array; `item` is called at each element and must read it.
    pub fn array(
        &mut self,
        mut item: impl FnMut(&mut Self) -> Result<(), Unreadable>,
    ) -> Result<(), Unreadable> {
        self.expect(b'[')?;
        if self.eat(b']') {
            return Ok(());
        }
        loop {
            item(self)?;
            if !self.eat(b',') {
                return self.expect(b']');
            }
        }
    }

    /// An integer from 0 to 2^53, read exactly: no fraction, no exponent, no sign, never rounded.
    pub fn integer(&mut self) -> Result<u64, Unreadable> {
        self.skip_white_space();
        let start = self.at;
        let mut value: u64 = 0;
        while let Some(digit @ b'0'..=b'9') = self.peek() {
            // At most 2^53 before this step, so the arithmetic cannot wrap.
            value = value * 10 + u64::from(digit - b'0');
            self.at += 1;
            let leading_zero =
                self.at - start > 1 && self.text.as_bytes().get(start) == Some(&b'0');
            if value > TaskNo::LIMIT.get() || leading_zero {
                return Err(Unreadable::Integer);
            }
        }
        if self.at == start || matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            return Err(Unreadable::Integer);
        }
        Ok(value)
    }

    /// A string. Borrowed from the input unless it holds an escape.
    pub fn string(&mut self) -> Result<Cow<'a, str>, Unreadable> {
        self.expect(b'"')?;
        let start = self.at;
        let mut unescaped: Option<String> = None;
        let mut run = start;
        loop {
            match self.peek() {
                Some(b'"') => {
                    let tail = self.slice(run);
                    self.at += 1;
                    return Ok(match unescaped {
                        None => Cow::Borrowed(tail),
                        Some(mut text) => {
                            text.push_str(tail);
                            Cow::Owned(text)
                        }
                    });
                }
                Some(b'\\') => {
                    let text = unescaped.get_or_insert_with(String::new);
                    text.push_str(self.slice(run));
                    self.at += 1;
                    let c = self.escape()?;
                    text.push(c);
                    run = self.at;
                }
                Some(byte) if byte >= 0x20 => self.at += 1,
                _ => return Err(Unreadable::Syntax(self.at)),
            }
        }
    }

    /// The input from `from` to here. Both ends sit beside an ASCII byte, so both are character boundaries.
    fn slice(&self, from: usize) -> &'a str {
        self.text.get(from..self.at).unwrap_or_default()
    }

    /// What follows a backslash.
    fn escape(&mut self) -> Result<char, Unreadable> {
        let at = self.at;
        let c = match self.peek() {
            Some(b'"') => '"',
            Some(b'\\') => '\\',
            Some(b'/') => '/',
            Some(b'b') => '\u{0008}',
            Some(b'f') => '\u{000C}',
            Some(b'n') => '\n',
            Some(b'r') => '\r',
            Some(b't') => '\t',
            Some(b'u') => {
                self.at += 1;
                return self.unicode_escape(at);
            }
            _ => return Err(Unreadable::Escape(at)),
        };
        self.at += 1;
        Ok(c)
    }

    /// `XXXX`, or a surrogate pair `XXXX\uXXXX`. A surrogate on its own names no character.
    fn unicode_escape(&mut self, at: usize) -> Result<char, Unreadable> {
        let first = self.hex4(at)?;
        let code = match first {
            0xD800..=0xDBFF => {
                if !(self.peek() == Some(b'\\')
                    && self.text.as_bytes().get(self.at + 1) == Some(&b'u'))
                {
                    return Err(Unreadable::Escape(at));
                }
                self.at += 2;
                let second = self.hex4(at)?;
                if !(0xDC00..=0xDFFF).contains(&second) {
                    return Err(Unreadable::Escape(at));
                }
                0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
            }
            _ => first,
        };
        char::from_u32(code).ok_or(Unreadable::Escape(at))
    }

    fn hex4(&mut self, at: usize) -> Result<u32, Unreadable> {
        let mut value = 0;
        for _ in 0..4 {
            let digit = self
                .peek()
                .and_then(|byte| char::from(byte).to_digit(16))
                .ok_or(Unreadable::Escape(at))?;
            value = value * 16 + digit;
            self.at += 1;
        }
        Ok(value)
    }
}
