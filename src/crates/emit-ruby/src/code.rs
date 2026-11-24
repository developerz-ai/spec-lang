//! Ruby text with the precedence it has, so an operand gets parentheses only when it needs them.

/// How tightly a piece of Ruby binds, loosest first.
pub(crate) const LOW: u8 = 1; // `c ? a : b`, `if ... end`, `raise ...`, an assignment
pub(crate) const OR: u8 = 2;
pub(crate) const AND: u8 = 3;
pub(crate) const EQ: u8 = 4;
pub(crate) const CMP: u8 = 5;
pub(crate) const ADD: u8 = 6;
pub(crate) const MUL: u8 = 7;
pub(crate) const UNARY: u8 = 9;
pub(crate) const ATOM: u8 = 10;

/// The widest a line the printer joins may get before it is broken, not counting its indentation.
pub(crate) const WIDTH: usize = 100;

#[derive(Clone, Debug)]
pub(crate) struct Code {
    pub(crate) text: String,
    pub(crate) prec: u8,
}

impl Code {
    pub(crate) fn atom(text: impl Into<String>) -> Self {
        Code { text: text.into(), prec: ATOM }
    }

    pub(crate) fn new(text: impl Into<String>, prec: u8) -> Self {
        Code { text: text.into(), prec }
    }

    /// The text, in parentheses when it binds looser than `min`.
    pub(crate) fn at(&self, min: u8) -> String {
        if self.prec >= min { self.text.clone() } else { format!("({})", self.text) }
    }

    pub(crate) fn multi(&self) -> bool {
        self.text.contains('\n')
    }
}

/// Every line one level deeper.
pub(crate) fn indent(text: &str) -> String {
    text.lines().map(|l| if l.is_empty() { String::new() } else { format!("  {l}") }).collect::<Vec<_>>().join("\n")
}

pub(crate) fn indent_lines(lines: &[String]) -> String {
    indent(&lines.join("\n"))
}

/// `head(a, b)`, one item per line when the whole would be too wide. `open` and `close` are the brackets.
pub(crate) fn list(head: &str, open: &str, items: &[String], close: &str) -> String {
    let flat = format!("{head}{open}{}{close}", items.join(", "));
    if items.is_empty() || (flat.len() <= WIDTH && !flat.contains('\n')) {
        return flat;
    }
    format!("{head}{open}\n{}\n{close}", indent(&items.join(",\n")))
}

/// `recv.name(args)`.
pub(crate) fn call(target: &str, args: &[String]) -> Code {
    Code::atom(list(target, "(", args, ")"))
}

/// `head { |params| body }`, or `head do |params| ... end` when the body takes more than a line.
pub(crate) fn block(head: &str, params: &[String], body: &[String]) -> Code {
    let bars = if params.is_empty() { String::new() } else { format!(" |{}|", params.join(", ")) };
    if let [one] = body
        && !one.contains('\n')
        && head.len() + bars.len() + one.len() + 6 <= WIDTH
        && !head.contains('\n')
    {
        return Code::atom(format!("{head} {{{bars} {one} }}"));
    }
    Code::atom(format!("{head} do{bars}\n{}\nend", indent_lines(body)))
}

/// Operands joined by `&&` or `||`, one per line when the whole would be too wide.
pub(crate) fn chain(operands: &[String], op: &str) -> String {
    let flat = operands.join(&format!(" {op} "));
    if flat.len() <= WIDTH && !flat.contains('\n') {
        return flat;
    }
    let mut out = String::new();
    for (i, o) in operands.iter().enumerate() {
        if i > 0 {
            out.push_str(&format!(" {op}\n"));
        }
        out.push_str(&if i == 0 { o.clone() } else { indent(o) });
    }
    out
}

/// A double-quoted Ruby string body: no interpolation survives, nothing invisible stays raw.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '#' if matches!(chars.peek(), Some('{' | '$' | '@')) => out.push_str("\\#"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_ascii_graphic() || c == ' ' => out.push(c),
            c if c.is_alphanumeric() => out.push(c),
            c => out.push_str(&format!("\\u{{{:X}}}", c as u32)),
        }
    }
    out
}

pub(crate) fn string(text: &str) -> String {
    format!("\"{}\"", escape(text))
}

/// `%w[a b]` for plain words, else an array of strings.
pub(crate) fn words(items: &[String]) -> String {
    if items.is_empty() {
        return "[]".into();
    }
    if items.iter().all(|i| !i.is_empty() && i.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))) {
        let flat = format!("%w[{}]", items.join(" "));
        if flat.len() <= WIDTH {
            return flat;
        }
        return format!("%w[\n{}\n]", indent(&items.join("\n")));
    }
    list("", "[", &items.iter().map(|i| string(i)).collect::<Vec<_>>(), "]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_keep_no_interpolation_and_nothing_invisible() {
        assert_eq!(string("a#{b}\u{3000}é\"\n"), "\"a\\#{b}\\u{3000}é\\\"\\n\"");
    }

    #[test]
    fn a_wide_call_breaks_one_argument_per_line() {
        let args: Vec<String> = (0..12).map(|i| format!("argument_number_{i}")).collect();
        assert!(call("f", &args).text.contains("(\n  argument_number_0,\n"));
        assert_eq!(call("f", &["a".into()]).text, "f(a)");
    }
}
