//! A minimal JSON value and writer: the toolchain has no dependencies, and
//! its output must be byte-stable (keys in insertion order).

/// A JSON value. Objects keep their keys in insertion order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonWriter {
    Null,
    Bool(bool),
    Int(i128),
    Str(String),
    Arr(Vec<JsonWriter>),
    Obj(Vec<(String, JsonWriter)>),
}

impl JsonWriter {
    pub fn obj() -> Self {
        JsonWriter::Obj(Vec::new())
    }

    pub fn str(s: impl Into<String>) -> Self {
        JsonWriter::Str(s.into())
    }

    /// Add a key to an object (ignored on other values).
    pub fn set(mut self, key: &str, value: JsonWriter) -> Self {
        if let JsonWriter::Obj(items) = &mut self {
            items.push((key.to_string(), value));
        }
        self
    }

    /// Add a key only when `value` is `Some`.
    pub fn set_opt(self, key: &str, value: Option<JsonWriter>) -> Self {
        match value {
            Some(v) => self.set(key, v),
            None => self,
        }
    }

    pub fn write(&self, out: &mut String) {
        match self {
            JsonWriter::Null => out.push_str("null"),
            JsonWriter::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            JsonWriter::Int(n) => out.push_str(&n.to_string()),
            JsonWriter::Str(s) => write_str(s, out),
            JsonWriter::Arr(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out);
                }
                out.push(']');
            }
            JsonWriter::Obj(items) => {
                out.push('{');
                for (i, (k, v)) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }
}

impl std::fmt::Display for JsonWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        self.write(&mut s);
        f.write_str(&s)
    }
}

fn write_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
