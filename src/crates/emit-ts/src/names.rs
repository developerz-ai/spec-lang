//! spec-lang names to TypeScript names. A name comes from the declaration alone, so adding a declaration
//! never renames its neighbours; a clash inside one scope is resolved by a suffix.

/// Words of a name: `next_no` and `nextNo` both give `["next", "no"]`.
fn words(name: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in name.chars() {
        if c == '_' || c == '-' || c == '.' {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        prev_lower = c.is_lowercase() || c.is_ascii_digit();
        cur.extend(c.to_lowercase());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn capital(w: &str) -> String {
    let mut cs = w.chars();
    match cs.next() {
        Some(c) => c.to_uppercase().chain(cs).collect(),
        None => String::new(),
    }
}

/// `next_no` gives `nextNo`.
pub fn camel(name: &str) -> String {
    let ws = words(name);
    let s: String = ws.iter().enumerate().map(|(i, w)| if i == 0 { w.clone() } else { capital(w) }).collect();
    safe(if s.is_empty() { "x".to_string() } else { s })
}

/// `task_no` gives `TaskNo`; a name already in that form stays.
pub fn pascal(name: &str) -> String {
    if name.chars().next().is_some_and(char::is_uppercase) && !name.contains('_') {
        return safe(name.to_string());
    }
    safe(words(name).iter().map(|w| capital(w)).collect())
}

/// `todo.board` gives `todo-board`; a file name in the project's style.
pub fn kebab(name: &str) -> String {
    words(name).join("-")
}

/// Words TypeScript reserves, and names the generated code itself uses at every level.
const RESERVED: &[&str] = &[
    "arguments", "await", "break", "case", "catch", "class", "const", "continue", "debugger", "default", "delete",
    "do", "else", "enum", "eval", "export", "extends", "false", "finally", "for", "function", "if", "implements",
    "import", "in", "instanceof", "interface", "let", "new", "null", "package", "private", "protected", "public",
    "return", "static", "super", "switch", "this", "throw", "true", "try", "typeof", "undefined", "var", "void",
    "while", "with", "yield", "as", "any", "boolean", "number", "string", "symbol", "never", "unknown", "object",
    "bigint", "type", "of", "Object", "Array", "Map", "Set", "String", "Number", "BigInt", "Boolean", "Symbol",
    "Error", "Math", "JSON", "Promise", "Date", "rt", "w", "check", "World", "WorldValue", "Handlers",
];

/// The name, or the name with `_` when TypeScript or the generated code reserves it.
pub fn safe(name: String) -> String {
    if RESERVED.contains(&name.as_str()) { format!("{name}_") } else { name }
}

/// A JavaScript string literal, double-quoted.
pub fn string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 || c == '\u{7f}' || c == '\u{2028}' || c == '\u{2029}' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Text inside a template literal: backquote, backslash and `${` escaped.
pub fn template_part(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '`' => out.push_str("\\`"),
            '\\' => out.push_str("\\\\"),
            '$' if chars.peek() == Some(&'{') => out.push_str("\\$"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 || c == '\u{7f}' || c == '\u{2028}' || c == '\u{2029}' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// A `bigint` literal.
pub fn int_literal(n: i128) -> String {
    format!("{n}n")
}

/// The path from the directory of `from` to `to`, both relative to the project root, as an import specifier.
pub fn relative(from: &str, to: &str) -> String {
    let from_dir: Vec<&str> = from.split('/').collect();
    let from_dir = &from_dir[..from_dir.len().saturating_sub(1)];
    let to_parts: Vec<&str> = to.split('/').collect();
    let common = from_dir.iter().zip(to_parts.iter()).take_while(|(a, b)| a == b).count();
    let mut out = String::new();
    if common == from_dir.len() {
        out.push_str("./");
    } else {
        for _ in common..from_dir.len() {
            out.push_str("../");
        }
    }
    out.push_str(&to_parts[common..].join("/"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cases() {
        assert_eq!(camel("next_no"), "nextNo");
        assert_eq!(camel("new"), "new_");
        assert_eq!(pascal("Task"), "Task");
        assert_eq!(pascal("task_no"), "TaskNo");
        assert_eq!(relative("src/todo/board.ts", "src/runtime/index.ts"), "../runtime/index.ts");
        assert_eq!(relative("src/sums.ts", "src/runtime/index.ts"), "./runtime/index.ts");
        assert_eq!(relative("test/run.ts", "src/machine.ts"), "../src/machine.ts");
        assert_eq!(template_part("a${b}`"), "a\\${b}\\`");
    }
}
