//! A JSON value with one canonical printing, and a reader for `provenance.json`.
//!
//! Objects keep insertion order, so the same projection always prints the
//! same bytes. Numbers are integers: nothing a projection counts is a fraction.

use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn str(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }

    pub fn count(n: usize) -> Json {
        Json::Int(i64::try_from(n).unwrap_or(i64::MAX))
    }

    pub fn strs<S: AsRef<str>>(items: impl IntoIterator<Item = S>) -> Json {
        Json::Arr(items.into_iter().map(|s| Json::str(s.as_ref())).collect())
    }

    pub fn obj<K: Into<String>>(fields: impl IntoIterator<Item = (K, Json)>) -> Json {
        Json::Obj(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// A field of an object.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Json::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }

    /// Two-space indented, one scalar-only array or object per line, `\n` at the end.
    pub fn pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }

    fn is_scalar(&self) -> bool {
        !matches!(self, Json::Arr(_) | Json::Obj(_))
    }

    fn write(&self, out: &mut String, depth: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(n) => {
                let _ = write!(out, "{n}");
            }
            Json::Str(s) => write_str(out, s),
            Json::Arr(items) if items.iter().all(Json::is_scalar) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    v.write(out, depth);
                }
                out.push(']');
            }
            Json::Arr(items) => {
                out.push_str("[\n");
                for (i, v) in items.iter().enumerate() {
                    indent(out, depth + 1);
                    v.write(out, depth + 1);
                    out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
                }
                indent(out, depth);
                out.push(']');
            }
            Json::Obj(fields) if fields.iter().all(|(_, v)| v.is_scalar()) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    write_str(out, k);
                    out.push_str(": ");
                    v.write(out, depth);
                }
                out.push('}');
            }
            Json::Obj(fields) => {
                out.push_str("{\n");
                for (i, (k, v)) in fields.iter().enumerate() {
                    indent(out, depth + 1);
                    write_str(out, k);
                    out.push_str(": ");
                    v.write(out, depth + 1);
                    out.push_str(if i + 1 < fields.len() { ",\n" } else { "\n" });
                }
                indent(out, depth);
                out.push('}');
            }
        }
    }

    /// Read a JSON text. Numbers with a fraction or exponent are refused: no
    /// input of this crate has one.
    pub fn parse(text: &str) -> Result<Json, String> {
        let mut r = Reader { src: text.as_bytes(), text, at: 0 };
        let v = r.value()?;
        r.space();
        if r.at != r.src.len() {
            return Err(r.error("text after the value"));
        }
        Ok(v)
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Reader<'a> {
    src: &'a [u8],
    text: &'a str,
    at: usize,
}

impl Reader<'_> {
    fn error(&self, what: &str) -> String {
        format!("json: {what} at byte {}", self.at)
    }

    fn space(&mut self) {
        while self.src.get(self.at).is_some_and(|b| b.is_ascii_whitespace()) {
            self.at += 1;
        }
    }

    fn eat(&mut self, word: &str) -> bool {
        if self.text.get(self.at..).is_some_and(|rest| rest.starts_with(word)) {
            self.at += word.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.space();
        match self.src.get(self.at) {
            None => Err(self.error("unexpected end")),
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::Str),
            Some(b't') if self.eat("true") => Ok(Json::Bool(true)),
            Some(b'f') if self.eat("false") => Ok(Json::Bool(false)),
            Some(b'n') if self.eat("null") => Ok(Json::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error("unexpected character")),
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.at;
        if self.src.get(self.at) == Some(&b'-') {
            self.at += 1;
        }
        while self.src.get(self.at).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
        }
        if matches!(self.src.get(self.at), Some(b'.' | b'e' | b'E')) {
            return Err(self.error("a number that is not an integer"));
        }
        let digits = self.text.get(start..self.at).unwrap_or("");
        digits.parse().map(Json::Int).map_err(|_| self.error("a number out of range"))
    }

    fn string(&mut self) -> Result<String, String> {
        self.at += 1;
        let mut out = String::new();
        loop {
            let rest = self.text.get(self.at..).unwrap_or("");
            let Some(c) = rest.chars().next() else {
                return Err(self.error("unclosed string"));
            };
            self.at += c.len_utf8();
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let Some(e) = self.src.get(self.at).copied() else {
                        return Err(self.error("unclosed escape"));
                    };
                    self.at += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'u' => {
                            let hex = self.text.get(self.at..self.at + 4).unwrap_or("");
                            let code = u32::from_str_radix(hex, 16).map_err(|_| self.error("a bad \\u escape"))?;
                            self.at += 4;
                            out.push(char::from_u32(code).ok_or_else(|| self.error("a surrogate \\u escape"))?);
                        }
                        _ => return Err(self.error("an unknown escape")),
                    }
                }
                c => out.push(c),
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.at += 1;
        let mut items = Vec::new();
        self.space();
        if self.eat("]") {
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.value()?);
            self.space();
            if self.eat("]") {
                return Ok(Json::Arr(items));
            }
            if !self.eat(",") {
                return Err(self.error("expected , or ]"));
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.at += 1;
        let mut fields = Vec::new();
        self.space();
        if self.eat("}") {
            return Ok(Json::Obj(fields));
        }
        loop {
            self.space();
            if self.src.get(self.at) != Some(&b'"') {
                return Err(self.error("expected a key"));
            }
            let key = self.string()?;
            self.space();
            if !self.eat(":") {
                return Err(self.error("expected :"));
            }
            fields.push((key, self.value()?));
            self.space();
            if self.eat("}") {
                return Ok(Json::Obj(fields));
            }
            if !self.eat(",") {
                return Err(self.error("expected , or }"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_one_canonical_form() {
        let v = Json::obj([
            ("view", Json::str("calls")),
            ("nodes", Json::Arr(vec![Json::obj([("id", Json::str("a\"b")), ("n", Json::Int(2))])])),
            ("tags", Json::strs(["x", "y"])),
            ("empty", Json::Arr(vec![])),
        ]);
        let want = "{\n  \"view\": \"calls\",\n  \"nodes\": [\n    {\"id\": \"a\\\"b\", \"n\": 2}\n  ],\n  \"tags\": [\"x\", \"y\"],\n  \"empty\": []\n}\n";
        assert_eq!(v.pretty(), want);
    }

    #[test]
    fn reads_what_it_prints() {
        let v = Json::obj([
            ("a", Json::Arr(vec![Json::Int(-3), Json::Null, Json::Bool(true)])),
            ("s", Json::str("tab\t \u{e9} \\ \"q\"")),
            ("o", Json::obj([("k", Json::Arr(vec![Json::obj([("x", Json::Int(1))])]))])),
        ]);
        assert_eq!(Json::parse(&v.pretty()), Ok(v));
    }

    #[test]
    fn refuses_bad_input() {
        assert!(Json::parse("{\"a\": 1.5}").is_err());
        assert!(Json::parse("[1, 2").is_err());
        assert!(Json::parse("{} x").is_err());
        assert_eq!(Json::parse("\"\\u00e9\""), Ok(Json::str("\u{e9}")));
    }
}
