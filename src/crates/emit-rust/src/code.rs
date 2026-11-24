//! A printed expression and what the printer must know to place it: statements that run before it, whether it
//! is a place (read with `.clone()`), whether it touches the machine, whether it can fail. Argument lists keep
//! the LIR's left-to-right order: an argument whose evaluation must come before a later one's is bound first.

use crate::cx::Cx;
use spec_lang_ir::ModuleId;
use spec_lang_lir::LTy;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default)]
pub struct Code {
    /// Statements to run first, in order.
    pub pre: Vec<String>,
    pub text: String,
    /// A place: a local, a field of one, a state. Its owned value needs `.clone()` unless `Copy`.
    pub place: bool,
    /// Usable as an operand without parentheses.
    pub atomic: bool,
    /// Mentions the machine `m`.
    pub m: bool,
    /// Can halt (contains `?` or `return Err`).
    pub fail: bool,
    /// A literal or a local: evaluating it has no effect and cannot fail.
    pub simple: bool,
    /// A place of a `Copy` type: read without `.clone()`.
    pub copy: bool,
}

impl Code {
    pub fn lit(text: impl Into<String>) -> Self {
        Code { text: text.into(), atomic: true, simple: true, ..Code::default() }
    }

    /// A value computed by `text`.
    pub fn val(text: impl Into<String>) -> Self {
        Code { text: text.into(), atomic: true, ..Code::default() }
    }

    pub fn op(text: impl Into<String>) -> Self {
        Code { text: text.into(), atomic: false, ..Code::default() }
    }

    pub fn with(mut self, from: &[&Code]) -> Self {
        for c in from {
            self.m |= c.m;
            self.fail |= c.fail;
        }
        self
    }

    pub fn m(mut self, m: bool) -> Self {
        self.m |= m;
        self
    }

    pub fn failing(mut self) -> Self {
        self.fail = true;
        self
    }

    /// The text, parenthesised unless atomic.
    pub fn operand(&self) -> String {
        if self.atomic { self.text.clone() } else { format!("({})", self.text) }
    }

    /// A shared reference to the value.
    pub fn reference(&self) -> String {
        format!("&{}", self.operand())
    }

    /// The statements and the value as one Rust expression.
    pub fn seal(&self) -> String {
        if self.pre.is_empty() { self.text.clone() } else { format!("{{ {} {} }}", self.pre.join(" "), self.text) }
    }

    pub fn sealed(&self) -> Code {
        Code { pre: Vec::new(), text: self.seal(), atomic: self.atomic || !self.pre.is_empty(), ..self.clone() }
    }
}

/// Where a `return` goes.
#[derive(Clone, Debug)]
pub enum Target {
    /// Out of the Rust function or closure: `return Ok(v)`.
    Fn,
    /// Out of a labelled block: `break 'l v`.
    Label(String),
}

/// The printer of one declaration's body.
#[derive(Debug)]
pub struct Fx<'c, 'a> {
    pub cx: &'c mut Cx<'a>,
    /// The module whose file this is (`None`: a crate-root file).
    pub module: Option<ModuleId>,
    /// Whether `m` is in scope.
    pub has_m: bool,
    /// Every local name of the body: a call of a same-module item by such a name is written `self::name`.
    pub locals: BTreeSet<String>,
    /// Locals some statement assigns: declared `mut`.
    pub assigned: BTreeSet<String>,
    pub targets: Vec<Target>,
    /// A test body: a `return`'s value is dropped and the test passes on.
    pub discard: bool,
    tmp: usize,
    label: usize,
}

impl<'c, 'a> Fx<'c, 'a> {
    pub fn new(cx: &'c mut Cx<'a>, module: Option<ModuleId>, has_m: bool) -> Self {
        Fx {
            cx,
            module,
            has_m,
            locals: BTreeSet::new(),
            assigned: BTreeSet::new(),
            discard: false,
            targets: vec![Target::Fn],
            tmp: 0,
            label: 0,
        }
    }

    pub fn tmp(&mut self) -> String {
        self.tmp += 1;
        format!("tmp{}", self.tmp)
    }

    pub fn label(&mut self) -> String {
        self.label += 1;
        format!("'b{}", self.label)
    }

    pub fn ty(&self, t: &LTy) -> String {
        self.cx.ty(t, self.module)
    }

    /// The owned value: places are cloned unless `Copy`.
    pub fn owned(&self, c: Code, t: &LTy) -> Code {
        if c.place && !self.cx.copy(t) {
            Code { text: format!("{}.clone()", c.operand()), place: false, atomic: true, simple: false, ..c }
        } else {
            Code { place: false, ..c }
        }
    }

    /// Prepares arguments evaluated left to right for one call. Statements of later arguments move ahead of
    /// earlier ones, so an earlier argument that is not simple is bound to a temporary first; with `callee_m`
    /// (the callee takes `m` first), every argument that touches `m` is bound too. Returns the statements and the
    /// arguments, now free of statements.
    pub fn args(&mut self, items: Vec<Code>, callee_m: bool) -> (Vec<String>, Vec<Code>) {
        let last = items.iter().rposition(|c| !c.pre.is_empty() || c.m);
        let mut pre = Vec::new();
        let mut out = Vec::with_capacity(items.len());
        for (i, mut c) in items.into_iter().enumerate() {
            let before_last = last.is_some_and(|l| i < l);
            let hoist =
                (before_last && !(c.simple && c.pre.is_empty() && !c.m)) || (last == Some(i) && callee_m && c.m);
            pre.append(&mut c.pre);
            if hoist {
                let t = self.tmp();
                let value = if c.place && !c.copy { format!("{}.clone()", c.operand()) } else { c.text };
                pre.push(format!("let {t} = {value};"));
                out.push(Code { text: t, simple: true, atomic: true, ..Code::default() });
            } else {
                out.push(c);
            }
        }
        (pre, out)
    }

    /// A call `path(m?, args)?`; `args` are owned already.
    pub fn call(&mut self, path: String, callee_m: bool, args: Vec<Code>) -> Code {
        let any_m = args.iter().any(|a| a.m);
        let (pre, args) = self.args(args, callee_m);
        let mut list: Vec<String> = args.iter().map(|a| a.text.clone()).collect();
        if callee_m {
            list.insert(0, "m".into());
        }
        Code {
            pre,
            text: format!("{path}({})?", list.join(", ")),
            atomic: true,
            m: callee_m || any_m,
            fail: true,
            ..Code::default()
        }
    }
}

/// `Ok(text)`, or the call itself when `text` is one call ending in `?` (`Ok(f(x)?)` is `f(x)`).
pub fn ok(text: &str) -> String {
    if let Some(call) = text.strip_suffix(")?") {
        let mut depth = 0i32;
        let mut open = None;
        for (i, c) in call.char_indices().rev() {
            match c {
                ')' => depth += 1,
                '(' if depth == 0 => {
                    open = Some(i);
                    break;
                }
                '(' => depth -= 1,
                _ => {}
            }
        }
        if let Some(i) = open {
            let head = &call[..i];
            if !head.is_empty()
                && head.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | ':' | '.' | '<' | '>' | '#'))
            {
                return format!("{call})");
            }
        }
    }
    format!("Ok({text})")
}

/// Whether `text` reads the name: the word itself, not a field (`x.name`) or a path segment (`a::name`).
pub fn uses(text: &str, name: &str) -> bool {
    let b = text.as_bytes();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'#' || c >= 0x80;
    let mut from = 0;
    while let Some(i) = text[from..].find(name) {
        let at = from + i;
        let end = at + name.len();
        let before = at.checked_sub(1).map(|j| b[j]);
        let field = before == Some(b'.') && at.checked_sub(2).is_none_or(|j| b[j] != b'.');
        if !field && !before.is_some_and(|c| word(c) || c == b':') && !b.get(end).copied().is_some_and(word) {
            return true;
        }
        from = end;
    }
    false
}

/// A parameter or local as it is declared: `_x` when the code after it never reads it.
pub fn declared(name: &str, text: &str) -> String {
    if uses(text, name) { name.to_string() } else { format!("_{}", name.trim_start_matches("r#")) }
}

/// A parameter list with the names `body` never reads marked as such.
pub fn params(list: &[String], body: &str) -> String {
    let one = |p: &String| match p.split_once(": ") {
        Some((n, t)) => format!("{}: {t}", declared(n, body)),
        None => p.clone(),
    };
    list.iter().map(one).collect::<Vec<_>>().join(", ")
}
