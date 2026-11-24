//! C++ names for spec-lang names: identifiers that are not keywords, a
//! namespace and a path per module.

/// Words a generated identifier may not be: C++ keywords, macros of the C
/// library, and the names the printer itself uses.
const RESERVED: &[&str] = &[
    "alignas",
    "alignof",
    "and",
    "and_eq",
    "asm",
    "auto",
    "bitand",
    "bitor",
    "bool",
    "break",
    "case",
    "catch",
    "char",
    "char8_t",
    "char16_t",
    "char32_t",
    "class",
    "compl",
    "concept",
    "const",
    "consteval",
    "constexpr",
    "constinit",
    "const_cast",
    "continue",
    "co_await",
    "co_return",
    "co_yield",
    "decltype",
    "default",
    "delete",
    "do",
    "double",
    "dynamic_cast",
    "else",
    "enum",
    "explicit",
    "export",
    "extern",
    "false",
    "float",
    "for",
    "friend",
    "goto",
    "if",
    "inline",
    "int",
    "long",
    "mutable",
    "namespace",
    "slg_program",
    "new",
    "noexcept",
    "not",
    "not_eq",
    "nullptr",
    "operator",
    "or",
    "or_eq",
    "private",
    "protected",
    "public",
    "register",
    "reinterpret_cast",
    "requires",
    "return",
    "short",
    "signed",
    "sizeof",
    "static",
    "static_assert",
    "static_cast",
    "struct",
    "switch",
    "template",
    "this",
    "thread_local",
    "throw",
    "true",
    "try",
    "typedef",
    "typeid",
    "typename",
    "union",
    "unsigned",
    "using",
    "virtual",
    "void",
    "volatile",
    "wchar_t",
    "while",
    "xor",
    "xor_eq",
    "assert",
    "errno",
    "stdin",
    "stdout",
    "stderr",
    "NULL",
    "EOF",
    "ctx",
    "rt",
    "std",
    "program",
    "foreign",
    "lib",
    "main",
    "value",
    "alt",
    "show",
];

/// A name as a C++ identifier: itself, or with `_` after it when C++ or the printer owns the word.
pub fn ident(name: &str) -> String {
    if RESERVED.contains(&name) || name.is_empty() || name.contains("__") || name.starts_with('_') {
        format!("{}_", name.trim_start_matches('_').replace("__", "_"))
    } else {
        name.to_string()
    }
}

/// `TaskNo` as `task_no`.
pub fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 && !out.ends_with('_') {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Every module's namespace sits inside this one: a module may be called
/// `access` or `time`, and the C library declares those at global scope.
const ROOT: &str = "slg_program";

/// `todo.board` as `slg_program::todo::board`.
pub fn namespace(module: &str) -> String {
    std::iter::once(ROOT.to_string()).chain(module.split('.').map(ident)).collect::<Vec<_>>().join("::")
}

/// `todo.board` as `todo/board`.
pub fn path(module: &str) -> String {
    module.split('.').map(ident).collect::<Vec<_>>().join("/")
}

/// `todo.board.add` as `todo_board_add`.
pub fn flat(qualified: &str) -> String {
    qualified.replace('.', "_")
}

/// A Text as a C++ string literal body with its quotes: printable text raw, the rest escaped.
pub fn string_literal(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '?' => out.push_str("\\?"),
            c if (c as u32) < 0x20 || c as u32 == 0x7F => out.push_str(&format!("\\{:03o}", c as u32)),
            c if (c as u32) < 0x7F => out.push(c),
            // Raw UTF-8 for what a reader can see; the rest by its bytes, so no editor changes it.
            c if c.is_alphanumeric() || (!c.is_whitespace() && !c.is_control() && (c as u32) < 0x2000) => out.push(c),
            c if c as u32 >= 0x1_0000 || (0x2E80..0xD800).contains(&(c as u32)) => out.push(c),
            c => {
                let mut buf = [0u8; 4];
                for b in c.encode_utf8(&mut buf).bytes() {
                    out.push_str(&format!("\\{b:03o}"));
                }
            }
        }
    }
    out.push('"');
    out
}
