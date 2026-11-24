//! The store's bytes: strict JSON read into a tree, then into plain data by the states' declared types; and the
//! canonical writer. One line, no white space, keys in a fixed order, strings escaping only `"` and `\`, every
//! integer within 2^53 (what JSON numbers carry exactly). A `Map` whose values are records holding their key in
//! a field of the key's type is written as the array of its values; any other `Map` as an array of `[key, value]`.

use crate::rt::Int;
use crate::rt::data::{Data, Shape, Ty};

/// The largest integer the store carries: 2^53.
pub const LIMIT: Int = 1 << 53;

#[derive(Debug)]
enum Json {
    Null,
    Bool(bool),
    /// The literal as written.
    Number(String),
    Text(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

struct Reader<'a> {
    text: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn white(&mut self) {
        while matches!(self.text.get(self.at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn eat(&mut self, b: u8) -> bool {
        self.white();
        let hit = self.text.get(self.at) == Some(&b);
        if hit {
            self.at += 1;
        }
        hit
    }

    fn word(&mut self, w: &str) -> Option<()> {
        self.text[self.at..].starts_with(w.as_bytes()).then(|| self.at += w.len())
    }

    fn value(&mut self, depth: usize) -> Option<Json> {
        if depth > 64 {
            return None;
        }
        self.white();
        match self.text.get(self.at)? {
            b'n' => self.word("null").map(|()| Json::Null),
            b't' => self.word("true").map(|()| Json::Bool(true)),
            b'f' => self.word("false").map(|()| Json::Bool(false)),
            b'"' => self.string().map(Json::Text),
            b'[' => {
                self.at += 1;
                let mut items = Vec::new();
                if self.eat(b']') {
                    return Some(Json::Array(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    if self.eat(b']') {
                        return Some(Json::Array(items));
                    }
                    if !self.eat(b',') {
                        return None;
                    }
                }
            }
            b'{' => {
                self.at += 1;
                let mut members: Vec<(String, Json)> = Vec::new();
                if self.eat(b'}') {
                    return Some(Json::Object(members));
                }
                loop {
                    self.white();
                    let key = self.string()?;
                    if members.iter().any(|(k, _)| *k == key) || !self.eat(b':') {
                        return None;
                    }
                    members.push((key, self.value(depth + 1)?));
                    if self.eat(b'}') {
                        return Some(Json::Object(members));
                    }
                    if !self.eat(b',') {
                        return None;
                    }
                }
            }
            _ => self.number().map(Json::Number),
        }
    }

    fn number(&mut self) -> Option<String> {
        let start = self.at;
        if self.text.get(self.at) == Some(&b'-') {
            self.at += 1;
        }
        let digits = self.at;
        while self.text.get(self.at).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
        }
        if self.at == digits {
            return None;
        }
        if self.text.get(self.at) == Some(&b'.') {
            self.at += 1;
            while self.text.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
        }
        if matches!(self.text.get(self.at), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.text.get(self.at), Some(b'+' | b'-')) {
                self.at += 1;
            }
            while self.text.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
        }
        std::str::from_utf8(&self.text[start..self.at]).ok().map(str::to_string)
    }

    fn string(&mut self) -> Option<String> {
        if self.text.get(self.at) != Some(&b'"') {
            return None;
        }
        self.at += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            match *self.text.get(self.at)? {
                b'"' => {
                    self.at += 1;
                    return String::from_utf8(out).ok();
                }
                b'\\' => {
                    self.at += 1;
                    let c = match *self.text.get(self.at)? {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            self.at += 1;
                            let c = self.unicode()?;
                            out.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
                            continue;
                        }
                        _ => return None,
                    };
                    self.at += 1;
                    out.push(c as u8);
                }
                b if b < 0x20 => return None,
                b => {
                    out.push(b);
                    self.at += 1;
                }
            }
        }
    }

    fn hex4(&mut self) -> Option<u32> {
        let digits = std::str::from_utf8(self.text.get(self.at..self.at + 4)?).ok()?;
        if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        self.at += 4;
        u32::from_str_radix(digits, 16).ok()
    }

    /// `XXXX` or a surrogate pair `XXXX\uXXXX`; a lone surrogate names no character.
    fn unicode(&mut self) -> Option<char> {
        let first = self.hex4()?;
        let code = if (0xD800..0xDC00).contains(&first) {
            self.word("\\u")?;
            let second = self.hex4()?;
            if !(0xDC00..0xE000).contains(&second) {
                return None;
            }
            0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
        } else {
            first
        };
        char::from_u32(code)
    }
}

/// The document's top-level object, members by name, each still JSON.
pub struct Document {
    members: Vec<(String, Json)>,
}

impl Document {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let mut r = Reader { text: bytes, at: 0 };
        let v = r.value(0)?;
        r.white();
        if r.at != bytes.len() {
            return None;
        }
        match v {
            Json::Object(members) => Some(Document { members }),
            _ => None,
        }
    }

    pub fn keys(&self) -> Vec<&str> {
        self.members.iter().map(|(k, _)| k.as_str()).collect()
    }

    /// The member `key` as a value of `ty`.
    pub fn get(&self, key: &str, ty: &'static Ty) -> Option<Data> {
        let (_, v) = self.members.iter().find(|(k, _)| k == key)?;
        decode(v, ty)
    }
}

fn int(literal: &str) -> Option<Int> {
    let digits = literal.strip_prefix('-').unwrap_or(literal);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) || (digits.len() > 1 && digits.starts_with('0')) {
        return None;
    }
    let n: Int = literal.parse().ok()?;
    (n.abs() <= LIMIT).then_some(n)
}

fn decode(v: &Json, ty: &'static Ty) -> Option<Data> {
    let d = match (&ty.shape, v) {
        (Shape::Unit, Json::Null) => Data::Unit,
        (Shape::Bool, Json::Bool(b)) => Data::Bool(*b),
        (Shape::Int, Json::Number(n)) => Data::Int(int(n)?),
        (Shape::Text, Json::Text(t)) => Data::Text(t.clone()),
        (Shape::Option(_), Json::Null) => Data::None,
        (Shape::Option(t), v) => Data::Some(Box::new(decode(v, t)?)),
        (Shape::List(t), Json::Array(xs)) => Data::List(xs.iter().map(|x| decode(x, t)).collect::<Option<_>>()?),
        (Shape::Set(t), Json::Array(xs)) => Data::Set(xs.iter().map(|x| decode(x, t)).collect::<Option<_>>()?),
        (Shape::Tuple(ts), Json::Array(xs)) if ts.len() == xs.len() => {
            Data::Tuple(ts.iter().zip(xs).map(|(t, x)| decode(x, t)).collect::<Option<_>>()?)
        }
        (Shape::Map(k, val), Json::Array(xs)) => match key_field(k, val) {
            Some(f) => Data::Map(
                xs.iter()
                    .map(|x| {
                        let d = decode(x, val)?;
                        Some((d.field(f)?.clone(), d))
                    })
                    .collect::<Option<_>>()?,
            ),
            None => Data::Map(
                xs.iter()
                    .map(|x| match x {
                        Json::Array(kv) if kv.len() == 2 => Some((decode(&kv[0], k)?, decode(&kv[1], val)?)),
                        _ => None,
                    })
                    .collect::<Option<_>>()?,
            ),
        },
        (Shape::Record(fields), Json::Object(members)) => {
            if members.len() != fields.len() {
                return None;
            }
            let mut out = Vec::new();
            for (name, t) in *fields {
                let (_, x) = members.iter().find(|(k, _)| k == name)?;
                out.push((*name, decode(x, t)?));
            }
            Data::Record(out)
        }
        (Shape::Sum(variants), Json::Text(name)) => {
            let (n, fields) = variants.iter().find(|(n, _)| n == name)?;
            if !fields.is_empty() {
                return None;
            }
            Data::Variant(n, Vec::new())
        }
        (Shape::Sum(variants), Json::Object(members)) if members.len() == 1 => {
            let (name, Json::Object(inner)) = &members[0] else { return None };
            let (n, fields) = variants.iter().find(|(n, _)| n == name)?;
            if inner.len() != fields.len() || fields.is_empty() {
                return None;
            }
            let mut out = Vec::new();
            for (f, t) in *fields {
                let (_, x) = inner.iter().find(|(k, _)| k == f)?;
                out.push((*f, decode(x, t)?));
            }
            Data::Variant(n, out)
        }
        _ => return None,
    };
    Some(d)
}

/// The field of a map's record values that holds its key: the first of the key's type.
fn key_field(key: &'static Ty, value: &'static Ty) -> Option<&'static str> {
    match &value.shape {
        Shape::Record(fields) => fields.iter().find(|(_, t)| std::ptr::eq(*t, key)).map(|(n, _)| *n),
        _ => None,
    }
}

/// A value too large for the store.
#[derive(Debug)]
pub struct Full;

/// Writes `d`, a value of `ty`, canonically.
pub fn encode(out: &mut String, d: &Data, ty: &'static Ty) -> Result<(), Full> {
    match (&ty.shape, d) {
        (_, Data::Unit | Data::None) => out.push_str("null"),
        (_, Data::Bool(b)) => out.push_str(if *b { "true" } else { "false" }),
        (_, Data::Int(n)) => {
            if n.abs() > LIMIT {
                return Err(Full);
            }
            out.push_str(&n.to_string());
        }
        (_, Data::Text(t)) => text(out, t),
        (Shape::Option(t), Data::Some(v)) => encode(out, v, t)?,
        (Shape::List(t) | Shape::Set(t), Data::List(xs) | Data::Set(xs)) => array(out, xs.iter(), |out, x| encode(out, x, t))?,
        (Shape::Tuple(ts), Data::Tuple(xs)) => array(out, ts.iter().zip(xs), |out, (t, x)| encode(out, x, t))?,
        (Shape::Map(k, v), Data::Map(entries)) => match key_field(k, v) {
            Some(_) => array(out, entries.iter(), |out, (_, x)| encode(out, x, v))?,
            None => array(out, entries.iter(), |out, (kx, vx)| {
                out.push('[');
                encode(out, kx, k)?;
                out.push(',');
                encode(out, vx, v)?;
                out.push(']');
                Ok(())
            })?,
        },
        (Shape::Record(fields), Data::Record(values)) => object(out, fields, values)?,
        (Shape::Sum(variants), Data::Variant(name, values)) => {
            let fields = variants.iter().find(|(n, _)| n == name).map_or(&[][..], |(_, f)| *f);
            if values.is_empty() {
                text(out, name);
            } else {
                out.push('{');
                text(out, name);
                out.push(':');
                object(out, fields, values)?;
                out.push('}');
            }
        }
        _ => out.push_str("null"),
    }
    Ok(())
}

fn object(out: &mut String, fields: &[(&str, &'static Ty)], values: &[(&str, Data)]) -> Result<(), Full> {
    out.push('{');
    for (i, ((name, t), (_, v))) in fields.iter().zip(values).enumerate() {
        if i > 0 {
            out.push(',');
        }
        text(out, name);
        out.push(':');
        encode(out, v, t)?;
    }
    out.push('}');
    Ok(())
}

fn array<T>(out: &mut String, items: impl Iterator<Item = T>, mut each: impl FnMut(&mut String, T) -> Result<(), Full>) -> Result<(), Full> {
    out.push('[');
    for (i, x) in items.enumerate() {
        if i > 0 {
            out.push(',');
        }
        each(out, x)?;
    }
    out.push(']');
    Ok(())
}

/// A string: only `"` and `\` escaped, everything else raw UTF-8.
pub fn text(out: &mut String, t: &str) {
    out.push('"');
    for c in t.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
}
