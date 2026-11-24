//! `view --as pseudo`: the program as textbook pseudocode.
//!
//! No spec-lang punctuation is assumed of the reader: `←` assigns, `=` and
//! `≠` compare, a guard is `require p else fail E`, an outside effect is
//! `emit`, blocks close with `end`. Notes become plain lines under the header.
//! Printed from the IR, so every name is the declaration it resolves to and
//! every argument is in parameter order.

use crate::class::params_text;
use crate::facts::{Facts, Kind};
use spec_lang_ir::{
    Arg, Arm, BinOp, Block, Callee, CmpOp, DeclRef, Expect, Expr, ExprKind, Failure, InterpPart, LambdaBody, MutOp,
    PatArg, Pattern, Place, PlaceRoot, PlaceStep, Stmt, StmtKind, TypeDef, UnOp,
};
use std::fmt::Write as _;

struct Printer<'a, 'p> {
    f: &'a Facts<'p>,
    out: String,
    depth: usize,
}

/// Binding strength: a child printed inside a tighter parent gets brackets.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Lowest,
    Implies,
    Or,
    And,
    Not,
    Compare,
    Range,
    Add,
    Mul,
    Unary,
    Postfix,
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl Printer<'_, '_> {
    fn line(&mut self, text: &str) {
        for _ in 0..self.depth {
            self.out.push_str("  ");
        }
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn name(&self, r: DeclRef) -> String {
        self.f.short_of(r)
    }

    /// Arguments in parameter order; once one is left to its default, the rest are named.
    fn args(&self, args: &[Option<Expr>], names: &[String]) -> String {
        let mut out = Vec::new();
        let mut skipped = false;
        for (i, a) in args.iter().enumerate() {
            match a {
                None => skipped = true,
                Some(e) if skipped => {
                    let name = names.get(i).map_or("?", String::as_str);
                    out.push(format!("{name}: {}", self.expr(e, Level::Lowest)));
                }
                Some(e) => out.push(self.expr(e, Level::Lowest)),
            }
        }
        out.join(", ")
    }

    fn callee(&self, callee: Callee) -> (String, Vec<String>) {
        let p = self.f.p;
        match callee {
            Callee::Fn(id) => (self.name(DeclRef::Fn(id)), p.func(id).params.iter().map(|x| x.name.clone()).collect()),
            Callee::Rule(id) => {
                (self.name(DeclRef::Rule(id)), p.rule(id).params.iter().map(|x| x.name.clone()).collect())
            }
            Callee::Record(id) => {
                let names = match &p.ty(id).def {
                    TypeDef::Record(fields) => fields.iter().map(|x| x.name.clone()).collect(),
                    _ => Vec::new(),
                };
                (self.name(DeclRef::Type(id)), names)
            }
            Callee::Variant(id, tag) => {
                let names = match &p.ty(id).def {
                    TypeDef::Sum(vs) => vs
                        .get(tag as usize)
                        .map(|v| v.fields.iter().map(|x| x.name.clone()).collect())
                        .unwrap_or_default(),
                    _ => Vec::new(),
                };
                (self.f.variant_name(id, tag), names)
            }
            Callee::Effect(e) => (
                p.effect_name(e),
                p.effect_op(e).into_iter().flat_map(|o| o.params.iter().flatten()).map(|x| x.name.clone()).collect(),
            ),
        }
    }

    fn dyn_args(&self, args: &[Arg]) -> String {
        let one = |a: &Arg| match &a.name {
            Some(n) => format!("{n}: {}", self.expr(&a.value, Level::Lowest)),
            None => self.expr(&a.value, Level::Lowest),
        };
        args.iter().map(one).collect::<Vec<_>>().join(", ")
    }

    fn failure(&self, x: &Failure) -> String {
        if x.args.is_empty() {
            format!("fail {}", x.name)
        } else {
            let args: Vec<String> = x.args.iter().map(|a| self.expr(a, Level::Lowest)).collect();
            format!("fail {}({})", x.name, args.join(", "))
        }
    }

    fn pat_args(&self, args: &[PatArg]) -> String {
        let one = |a: &PatArg| match &a.name {
            Some(n) => format!("{n}: {}", self.pattern(&a.pattern)),
            None => self.pattern(&a.pattern),
        };
        args.iter().map(one).collect::<Vec<_>>().join(", ")
    }

    fn pattern(&self, pat: &Pattern) -> String {
        match pat {
            Pattern::Wild => "_".to_string(),
            Pattern::Bind(n) => n.clone(),
            Pattern::Value(e) => self.expr(e, Level::Lowest),
            Pattern::Variant { ty, tag, args } if args.is_empty() => self.f.variant_name(*ty, *tag),
            Pattern::Variant { ty, tag, args } => {
                format!("{}({})", self.f.variant_name(*ty, *tag), self.pat_args(args))
            }
            Pattern::Tuple(ps) => format!("({})", ps.iter().map(|x| self.pattern(x)).collect::<Vec<_>>().join(", ")),
            Pattern::Call { rule, args } => format!("{}({})", self.name(DeclRef::Rule(*rule)), self.pat_args(args)),
        }
    }

    fn place(&self, place: &Place) -> String {
        let mut s = match &place.root {
            PlaceRoot::State(id) => self.name(DeclRef::State(*id)),
            PlaceRoot::Local(n) => n.clone(),
        };
        for step in &place.path {
            match step {
                PlaceStep::Field(n) => {
                    let _ = write!(s, ".{n}");
                }
                PlaceStep::Index(k) => {
                    let _ = write!(s, "[{}]", self.expr(k, Level::Lowest));
                }
            }
        }
        s
    }

    /// Statements of a block on one line, for a block inside an expression.
    fn inline_block(&self, b: &Block) -> String {
        let mut inner = Printer { f: self.f, out: String::new(), depth: 0 };
        inner.block(b);
        let lines: Vec<&str> = inner.out.lines().map(str::trim).collect();
        format!("{{ {} }}", lines.join("; "))
    }

    fn expr(&self, e: &Expr, parent: Level) -> String {
        let (text, level) = self.expr_level(e);
        if level < parent { format!("({text})") } else { text }
    }

    fn expr_level(&self, e: &Expr) -> (String, Level) {
        let atom = |s: String| (s, Level::Postfix);
        match &e.kind {
            ExprKind::Unit => atom("()".to_string()),
            ExprKind::Bool(b) => atom(b.to_string()),
            ExprKind::Int(n) => atom(n.to_string()),
            ExprKind::Text(s) => atom(quote(s)),
            ExprKind::Interp(parts) => {
                let mut s = String::from("\"");
                for part in parts {
                    match part {
                        InterpPart::Lit(l) => {
                            let q = quote(l);
                            s.push_str(q.get(1..q.len() - 1).unwrap_or(""));
                        }
                        InterpPart::Expr(x) => {
                            let _ = write!(s, "{{{}}}", self.expr(x, Level::Lowest));
                        }
                    }
                }
                s.push('"');
                atom(s)
            }
            ExprKind::Local(n) => atom(n.clone()),
            ExprKind::Const(id) => atom(self.name(DeclRef::Const(*id))),
            ExprKind::State(id) => atom(self.name(DeclRef::State(*id))),
            ExprKind::Variant { ty, tag } => atom(self.f.variant_name(*ty, *tag)),
            ExprKind::Call { callee, args } => {
                let (name, names) = self.callee(*callee);
                atom(format!("{name}({})", self.args(args, &names)))
            }
            ExprKind::CallValue { callee, args } => {
                let args: Vec<String> = args.iter().map(|a| self.expr(a, Level::Lowest)).collect();
                atom(format!("{}({})", self.expr(callee, Level::Postfix), args.join(", ")))
            }
            ExprKind::Method { recv, name, args } => {
                atom(format!("{}.{name}({})", self.expr(recv, Level::Postfix), self.dyn_args(args)))
            }
            ExprKind::Prop { recv, name } => atom(format!("{}.{name}", self.expr(recv, Level::Postfix))),
            ExprKind::Index { recv, key } => {
                atom(format!("{}[{}]", self.expr(recv, Level::Postfix), self.expr(key, Level::Lowest)))
            }
            ExprKind::With { recv, fields } => {
                let fs: Vec<String> =
                    fields.iter().map(|(n, v)| format!("{n}: {}", self.expr(v, Level::Lowest))).collect();
                atom(format!("{}.with({})", self.expr(recv, Level::Postfix), fs.join(", ")))
            }
            ExprKind::Is { value, pattern } => {
                (format!("{} is {}", self.expr(value, Level::Range), self.pattern(pattern)), Level::Compare)
            }
            ExprKind::Unary(op, x) => match op {
                UnOp::Not => (format!("not {}", self.expr(x, Level::Not)), Level::Not),
                UnOp::Neg => (format!("-{}", self.expr(x, Level::Unary)), Level::Unary),
                UnOp::Pos => (format!("+{}", self.expr(x, Level::Unary)), Level::Unary),
                UnOp::Own => (format!("own {}", self.expr(x, Level::Unary)), Level::Unary),
                UnOp::Ref => (format!("ref {}", self.expr(x, Level::Unary)), Level::Unary),
                UnOp::Mut => (format!("mut {}", self.expr(x, Level::Unary)), Level::Unary),
            },
            ExprKind::Binary(op, a, b) => {
                let (word, level) = match op {
                    BinOp::Add => ("+", Level::Add),
                    BinOp::Sub => ("-", Level::Add),
                    BinOp::Mul => ("×", Level::Mul),
                    BinOp::Div => ("÷", Level::Mul),
                    BinOp::Mod => ("mod", Level::Mul),
                    BinOp::Pow => ("^", Level::Unary),
                    BinOp::Band => ("band", Level::Mul),
                    BinOp::Bor => ("bor", Level::Add),
                    BinOp::Bxor => ("bxor", Level::Add),
                    BinOp::Shl => ("shl", Level::Mul),
                    BinOp::Shr => ("shr", Level::Mul),
                    BinOp::And => ("and", Level::And),
                    BinOp::Or => ("or", Level::Or),
                    BinOp::Implies => ("implies", Level::Implies),
                };
                // Left operand may be of the same level; the right one must bind tighter.
                let right = match level {
                    Level::Implies => Level::Or,
                    Level::Or => Level::And,
                    Level::And => Level::Not,
                    Level::Add => Level::Mul,
                    _ => Level::Unary,
                };
                let right = if matches!(op, BinOp::And | BinOp::Or) { level } else { right };
                (format!("{} {word} {}", self.expr(a, level), self.expr(b, right)), level)
            }
            ExprKind::Compare(first, rest) => {
                let mut s = self.expr(first, Level::Range);
                for (op, x) in rest {
                    let word = match op {
                        CmpOp::Eq => "=",
                        CmpOp::Ne => "≠",
                        CmpOp::Lt => "<",
                        CmpOp::Le => "≤",
                        CmpOp::Gt => ">",
                        CmpOp::Ge => "≥",
                        CmpOp::In => "in",
                    };
                    let _ = write!(s, " {word} {}", self.expr(x, Level::Range));
                }
                (s, Level::Compare)
            }
            ExprKind::Range { lo, hi, inclusive } => {
                let word = if *inclusive { "..=" } else { "..<" };
                (format!("{}{word}{}", self.expr(lo, Level::Add), self.expr(hi, Level::Add)), Level::Range)
            }
            ExprKind::Tuple(xs) => {
                atom(format!("({})", xs.iter().map(|x| self.expr(x, Level::Lowest)).collect::<Vec<_>>().join(", ")))
            }
            ExprKind::List(xs) => {
                atom(format!("[{}]", xs.iter().map(|x| self.expr(x, Level::Lowest)).collect::<Vec<_>>().join(", ")))
            }
            ExprKind::SetOf(xs) if xs.is_empty() => atom("empty set".to_string()),
            ExprKind::SetOf(xs) => atom(format!(
                "set {{{}}}",
                xs.iter().map(|x| self.expr(x, Level::Lowest)).collect::<Vec<_>>().join(", ")
            )),
            ExprKind::MapOf(pairs) if pairs.is_empty() => atom("empty map".to_string()),
            ExprKind::MapOf(pairs) => {
                let ps: Vec<String> = pairs
                    .iter()
                    .map(|(k, v)| format!("{} ↦ {}", self.expr(k, Level::Lowest), self.expr(v, Level::Lowest)))
                    .collect();
                atom(format!("map {{{}}}", ps.join(", ")))
            }
            ExprKind::Lambda(id) => {
                let l = self.f.p.lambda(*id);
                let params = match l.params.as_slice() {
                    [one] => one.clone(),
                    many => format!("({})", many.join(", ")),
                };
                let body = match &l.body {
                    LambdaBody::Expr(x) => self.expr(x, Level::Implies),
                    LambdaBody::Block(b) => self.inline_block(b),
                };
                (format!("{params} ↦ {body}"), Level::Lowest)
            }
            ExprKind::If { cond, then, els } => (
                format!(
                    "if {} then {} else {}",
                    self.expr(cond, Level::Implies),
                    self.expr(then, Level::Implies),
                    self.expr(els, Level::Implies)
                ),
                Level::Lowest,
            ),
            ExprKind::Else { value, fallback, .. } => (
                format!("{} else {}", self.expr(value, Level::Implies), self.expr(fallback, Level::Implies)),
                Level::Lowest,
            ),
            ExprKind::Fail(x) => (self.failure(x), Level::Lowest),
            ExprKind::Match { scrutinee, arms } => {
                let arms: Vec<String> = arms
                    .iter()
                    .map(|a| format!("{} ⇒ {}", self.pattern(&a.pattern), self.expr(&a.value, Level::Implies)))
                    .collect();
                atom(format!("match {} {{ {} }}", self.expr(scrutinee, Level::Implies), arms.join("; ")))
            }
            ExprKind::Block(b) => atom(self.inline_block(b)),
            ExprKind::EffectRead(e) => atom(self.f.p.effect_name(*e)),
            ExprKind::Hole(n) => atom(format!("?{n}")),
        }
    }

    fn arms(&mut self, scrutinee: &Expr, arms: &[Arm], leaf: &dyn Fn(&Self, &Expr) -> String) {
        let head = format!("match {}", self.expr(scrutinee, Level::Lowest));
        self.line(&head);
        self.depth += 1;
        for a in arms {
            let pattern = self.pattern(&a.pattern);
            match &a.value.kind {
                ExprKind::Block(b) => {
                    self.line(&format!("case {pattern}:"));
                    self.depth += 1;
                    self.block(b);
                    self.depth -= 1;
                }
                _ => {
                    let value = leaf(self, &a.value);
                    self.line(&format!("case {pattern}: {value}"));
                }
            }
        }
        self.depth -= 1;
        self.line("end");
    }

    /// The body of a `fn` or `rule`: a block is its statements, a `match` is
    /// a case list, anything else is the value returned.
    fn body(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Block(b) => self.block(b),
            ExprKind::Match { scrutinee, arms } => {
                self.arms(scrutinee, arms, &|p, v| format!("return {}", p.expr(v, Level::Lowest)));
            }
            _ => {
                let text = format!("return {}", self.expr(e, Level::Lowest));
                self.line(&text);
            }
        }
    }

    fn block(&mut self, b: &Block) {
        for s in &b.stmts {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Let { name, mutable, value, .. } => {
                let word = if *mutable { "var" } else { "let" };
                let text = format!("{word} {name} ← {}", self.expr(value, Level::Lowest));
                self.line(&text);
            }
            StmtKind::Assign { place, value } => {
                let text = format!("{} ← {}", self.place(place), self.expr(value, Level::Lowest));
                self.line(&text);
            }
            StmtKind::Mutate { place, op, args } => {
                let args: Vec<String> = args.iter().map(|a| self.expr(a, Level::Lowest)).collect();
                let (args, place) = (args.join(", "), self.place(place));
                let text = match op {
                    MutOp::Push => format!("append {args} to {place}"),
                    MutOp::Insert => format!("insert {args} into {place}"),
                    MutOp::Remove => format!("remove {args} from {place}"),
                };
                self.line(&text);
            }
            StmtKind::Expr(e) => match &e.kind {
                ExprKind::Call { callee: Callee::Effect(_), .. } => {
                    let text = format!("emit {}", self.expr(e, Level::Lowest));
                    self.line(&text);
                }
                ExprKind::Match { scrutinee, arms } => self.arms(scrutinee, arms, &|p, v| p.expr(v, Level::Lowest)),
                ExprKind::Fail(_) => {
                    let text = self.expr(e, Level::Lowest);
                    self.line(&text);
                }
                _ => {
                    let text = format!("do {}", self.expr(e, Level::Lowest));
                    self.line(&text);
                }
            },
            StmtKind::Return(None) => self.line("return"),
            StmtKind::Return(Some(e)) => {
                let text = format!("return {}", self.expr(e, Level::Lowest));
                self.line(&text);
            }
            StmtKind::Assert(e) => {
                let text = format!("assert {}", self.expr(e, Level::Lowest));
                self.line(&text);
            }
            StmtKind::For { var, iter, body } => {
                let text = format!("for each {var} in {} do", self.expr(iter, Level::Lowest));
                self.line(&text);
                self.depth += 1;
                self.block(body);
                self.depth -= 1;
                self.line("end");
            }
            StmtKind::While { cond, body } => {
                let text = format!("while {} do", self.expr(cond, Level::Lowest));
                self.line(&text);
                self.depth += 1;
                self.block(body);
                self.depth -= 1;
                self.line("end");
            }
            StmtKind::If { cond, then, els } => {
                let text = format!("if {} then", self.expr(cond, Level::Lowest));
                self.line(&text);
                self.depth += 1;
                self.block(then);
                self.depth -= 1;
                if let Some(els) = els {
                    self.line("else");
                    self.depth += 1;
                    self.block(els);
                    self.depth -= 1;
                }
                self.line("end");
            }
            StmtKind::Expect { subject, expect } => {
                let subject = self.expr(subject, Level::Lowest);
                let text = match expect {
                    Expect::Value(v) => format!("expect {subject} gives {}", self.expr(v, Level::Lowest)),
                    Expect::Fail(x) => format!("expect {subject} refused with {}", x.name),
                };
                self.line(&text);
            }
        }
    }

    fn why(&mut self, i: usize) {
        if let Some(why) = &self.f.decls[i].why {
            let text = format!("because: {why}");
            self.line(&text);
        }
    }

    fn decl(&mut self, i: usize) {
        let f = self.f;
        let d = &f.decls[i];
        match d.r {
            DeclRef::Fn(id) => {
                let x = f.p.func(id);
                let result = x.result.as_ref().map(|t| format!(" → {}", f.ty_text(t))).unwrap_or_default();
                self.line(&format!("function {}({}){result}", d.name, params_text(&x.params, f)));
                self.depth += 1;
                self.why(i);
                self.body(&x.body);
                self.depth -= 1;
                self.line("end");
            }
            DeclRef::Rule(id) => {
                let x = f.p.rule(id);
                let result = x.result.as_ref().map(|t| format!(" → {}", f.ty_text(t))).unwrap_or_default();
                self.line(&format!("rule {}({}){result}", d.name, params_text(&x.params, f)));
                self.depth += 1;
                self.why(i);
                for t in &x.triggers {
                    // `on command "add"`: each argument after its parameter's name.
                    let args: Vec<String> = t
                        .args
                        .iter()
                        .map(|a| match &a.value {
                            spec_lang_ir::NoteValue::Text(s) => format!("{} {}", a.param, quote(s)),
                            v => format!("{} {v}", a.param),
                        })
                        .collect();
                    self.line(&format!("on {}", args.join(", ")));
                }
                if !x.uses.is_empty() {
                    self.line(&format!("uses {}", x.uses.join(", ")));
                }
                for g in &x.guards {
                    let text = format!("require {} else fail {}", self.expr(&g.cond, Level::Implies), g.error);
                    self.line(&text);
                }
                self.body(&x.body);
                self.depth -= 1;
                self.line("end");
            }
            DeclRef::State(id) => {
                let x = f.p.state(id);
                let init = match &x.init {
                    Some(e) => format!(", initially {}", self.expr(e, Level::Lowest)),
                    None => ", initially empty".to_string(),
                };
                self.line(&format!("state {}: {}{init}", d.name, f.ty_text(&x.ty)));
            }
            DeclRef::Const(id) => {
                let text = format!("constant {} = {}", d.name, self.expr(&f.p.constant(id).value, Level::Lowest));
                self.line(&text);
            }
            DeclRef::Type(_) | DeclRef::Test(_) | DeclRef::Law(_) | DeclRef::App(_) => {}
        }
    }
}

/// One declaration as pseudocode; empty for a kind that has none (types, tests, laws, apps).
pub fn pseudo_decl(f: &Facts<'_>, i: usize) -> String {
    let mut p = Printer { f, out: String::new(), depth: 0 };
    p.decl(i);
    p.out
}

/// One module: its constants and states, then every `fn` and `rule` in source order.
/// Empty when the module has none of them.
pub fn pseudo_module(f: &Facts<'_>, module: &str) -> String {
    let of = |kinds: &[Kind]| -> Vec<usize> {
        (0..f.decls.len()).filter(|&i| f.decls[i].module == module && kinds.contains(&f.decls[i].kind)).collect()
    };
    let (data, code) = (of(&[Kind::Const, Kind::State]), of(&[Kind::Fn, Kind::Rule]));
    if data.is_empty() && code.is_empty() {
        return String::new();
    }
    let mut out = format!("module {module}\n");
    for i in data {
        out.push('\n');
        out.push_str(&pseudo_decl(f, i));
    }
    for i in code {
        out.push('\n');
        out.push_str(&pseudo_decl(f, i));
    }
    out
}
