//! Statements and blocks. A block that is a body (of a `fn`, a rule, a test, a lambda) is printed as a Ruby
//! method or lambda body, where `return` means what it means in the source. A block that is an expression
//! elsewhere is written in place when its only `return`s end it, and as a lambda called at once otherwise.

use crate::code::{self, ATOM, Code};
use crate::expr::{repeatable, wrap};
use crate::gen_::{Fx, R};
use spec_lang_lir::node::*;
use spec_lang_lir::{LTy, walk};

/// Whether the block's last statement is a `return` on every path.
fn ends_in_return(b: &Block) -> bool {
    match b.stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Return(_)) => true,
        Some(StmtKind::If { then, els: Some(els), .. }) => ends_in_return(then) && ends_in_return(els),
        _ => false,
    }
}

fn returns_in(b: &Block) -> usize {
    b.stmts
        .iter()
        .map(|s| match &s.kind {
            StmtKind::Return(_) => 1,
            StmtKind::For { body, .. } | StmtKind::While { body, .. } => returns_in(body),
            StmtKind::If { then, els, .. } => returns_in(then) + els.as_ref().map_or(0, returns_in),
            _ => 0,
        })
        .sum()
}

fn ending_returns(b: &Block) -> usize {
    match b.stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Return(_)) => 1,
        Some(StmtKind::If { then, els: Some(els), .. }) if ends_in_return(then) && ends_in_return(els) => {
            ending_returns(then) + ending_returns(els)
        }
        _ => 0,
    }
}

/// Whether the block can be written where it stands: every `return` in it ends it.
pub(crate) fn inlinable(b: &Block) -> bool {
    returns_in(b) == ending_returns(b)
}

/// Whether evaluating `e` can change the world: it calls a rule, a lambda or a handler.
fn writes(e: &Expr) -> bool {
    let mut found = false;
    walk::exprs(e, &mut |x| {
        found |= matches!(x.kind, ExprKind::CallRule { .. } | ExprKind::CallLambda { .. } | ExprKind::Effect { .. });
    });
    found
}

impl Fx<'_> {
    /// The lines of a body: `check` is what every `return` value must meet; `value` asks for `nil` where a
    /// body of type `()` ends without a `return`.
    pub(crate) fn body_lines(&mut self, b: &Block, check: Option<String>, value: bool) -> R<Vec<String>> {
        let saved = std::mem::replace(&mut self.result_check, check);
        let out = self.block_lines(b, value);
        self.result_check = saved;
        out
    }

    fn block_lines(&mut self, b: &Block, value: bool) -> R<Vec<String>> {
        self.push();
        let out = self.block_lines_in(b, value);
        self.pop();
        out
    }

    fn block_lines_in(&mut self, b: &Block, value: bool) -> R<Vec<String>> {
        let mut lines = Vec::new();
        let Some((last, rest)) = b.stmts.split_last() else {
            return Ok(if value { vec!["nil".into()] } else { lines });
        };
        for s in rest {
            lines.extend(self.stmt(s)?);
        }
        if !ends_in_return(b) {
            lines.extend(self.stmt(last)?);
            if value && b.ty == LTy::Unit {
                lines.push("nil".into());
            }
            return Ok(lines);
        }
        match &last.kind {
            StmtKind::Return(Some(e)) => lines.extend(self.returned(e)?),
            StmtKind::If { cond, then, els: Some(els) } => {
                let cond = self.expr(cond)?.text;
                let then = self.block_lines(then, true)?;
                let els = self.block_lines(els, true)?;
                lines.push(if_text(&cond, &then, Some(&els)));
            }
            _ => lines.push("nil".into()),
        }
        Ok(lines)
    }

    /// The value a `return` gives, checked against the body's result type.
    fn returned(&mut self, e: &Expr) -> R<Vec<String>> {
        match self.result_check.clone() {
            Some(t) => {
                let v = self.expr(e)?.text;
                Ok(vec![code::call(&format!("{t}.check"), &[v, code::string("the result")]).text])
            }
            None => self.value_lines(e),
        }
    }

    /// `e` as statements whose last one is its value.
    pub(crate) fn value_lines(&mut self, e: &Expr) -> R<Vec<String>> {
        match &e.kind {
            ExprKind::If { cond, then, els } => {
                let c = self.expr(cond)?;
                let then = self.scoped(|fx| fx.value_lines(then))?;
                let els = self.scoped(|fx| fx.value_lines(els))?;
                if let ([t], [f]) = (then.as_slice(), els.as_slice())
                    && !t.contains('\n')
                    && !f.contains('\n')
                    && !c.multi()
                    && c.text.len() + t.len() + f.len() + 6 <= code::WIDTH
                    && !f.contains(" ? ")
                    && !t.contains(" ? ")
                {
                    return Ok(vec![format!("{} ? {} : {}", c.at(code::OR), paren_low(t), paren_low(f))]);
                }
                Ok(vec![if_text(&c.text, &then, Some(&els))])
            }
            ExprKind::Match { scrutinee, arms } => self.match_lines(scrutinee, arms, true),
            ExprKind::Block(b) if inlinable(b) => self.body_lines(b, None, true),
            ExprKind::Block(b) => {
                let body = self.body_lines(b, None, true)?;
                Ok(vec![format!("lambda do\n{}\nend.call", code::indent_lines(&body))])
            }
            ExprKind::Else { value, fallback, unwrap: true } => {
                let v = self.scoped(|fx| fx.value_lines(value))?;
                let f = self.scoped(|fx| fx.value_lines(fallback))?;
                let tried = code::block("Spec.try", &[], &v);
                Ok(vec![code::block(&format!("{}.or", tried.text), &[], &f).text])
            }
            ExprKind::Else { value, fallback, unwrap: false } => {
                let v = self.scoped(|fx| fx.value_lines(value))?;
                let f = self.scoped(|fx| fx.value_lines(fallback))?;
                Ok(vec![format!(
                    "begin\n{}\nrescue Spec::Failure\n{}\nend",
                    code::indent_lines(&v),
                    code::indent_lines(&f)
                )])
            }
            _ => Ok(vec![self.expr(e)?.text]),
        }
    }

    /// `e` as statements, its value unused.
    pub(crate) fn effect_lines(&mut self, e: &Expr) -> R<Vec<String>> {
        match &e.kind {
            ExprKind::If { cond, then, els } => {
                let c = self.expr(cond)?.text;
                let then = self.scoped(|fx| fx.effect_lines(then))?;
                let els = self.scoped(|fx| fx.effect_lines(els))?;
                Ok(vec![if_text(&c, &then, if els.is_empty() { None } else { Some(&els) })])
            }
            ExprKind::Match { scrutinee, arms } => self.match_lines(scrutinee, arms, false),
            ExprKind::Block(b) if inlinable(b) => self.body_lines(b, None, false),
            ExprKind::Unit => Ok(Vec::new()),
            _ => self.value_lines(e),
        }
    }

    pub(crate) fn scoped<T>(&mut self, f: impl FnOnce(&mut Self) -> R<T>) -> R<T> {
        self.push();
        let out = f(self);
        self.pop();
        out
    }

    fn stmts(&mut self, b: &Block) -> R<Vec<String>> {
        self.scoped(|fx| {
            let mut lines = Vec::new();
            for s in &b.stmts {
                lines.extend(fx.stmt(s)?);
            }
            Ok(lines)
        })
    }

    /// One statement that does not end its body.
    pub(crate) fn stmt(&mut self, s: &Stmt) -> R<Vec<String>> {
        Ok(match &s.kind {
            StmtKind::Let { name, mutable, check, value, .. } => {
                let mut v = self.expr(value)?.text;
                if let Some(t) = check {
                    v = self.checked(t, v, &format!("`{name}`"))?;
                }
                let ruby = self.bind(name, *mutable);
                vec![assign(&ruby, &v)]
            }
            StmtKind::Assign { place, value } => self.assign(place, value)?,
            StmtKind::Mutate { place, op, args } => self.mutate(place, *op, args)?,
            StmtKind::Expr(e) => self.effect_lines(e)?,
            StmtKind::Return(None) => vec!["return".into()],
            StmtKind::Return(Some(e)) => {
                let v = wrap(self.returned(e)?);
                vec![format!("return {}", if v.multi() { format!("({})", v.text) } else { v.text })]
            }
            StmtKind::Assert(e) => vec![code::call("Spec.assert", &[self.expr(e)?.text]).text],
            StmtKind::For { var, iter, body, .. } => {
                let iter = self.expr(iter)?.at(ATOM);
                self.push();
                let var = self.bind_param(var);
                let body = self.stmts(body);
                self.pop();
                vec![format!("{iter}.each do |{var}|\n{}\nend", code::indent_lines(&body?)).replace("\n\nend", "\nend")]
            }
            StmtKind::While { cond, body } => {
                let cond = self.expr(cond)?.text;
                self.whiles += 1;
                let body = self.stmts(body);
                self.whiles -= 1;
                vec![format!("while {cond}\n{}\nend", code::indent_lines(&body?)).replace("\n\nend", "\nend")]
            }
            StmtKind::If { cond, then, els } => {
                let cond = self.expr(cond)?.text;
                let then = self.stmts(then)?;
                let els = match els {
                    Some(b) => Some(self.stmts(b)?),
                    None => None,
                };
                vec![if_text(&cond, &then, els.as_deref())]
            }
            StmtKind::Expect { subject, expect } => vec![self.expect(subject, expect)?],
        })
    }

    fn expect(&mut self, subject: &Expr, expect: &Expected) -> R<String> {
        let subject = wrap(self.scoped(|fx| fx.value_lines(subject))?);
        let subject = if subject.multi() {
            format!("lambda do\n{}\nend", code::indent(&subject.text))
        } else {
            format!("-> {{ {} }}", subject.text)
        };
        Ok(match expect {
            Expected::Value(v) => {
                let v = self.scoped(|fx| fx.value_lines(v))?;
                code::block(&format!("Spec::Testing.gives({subject})"), &[], &v).text
            }
            Expected::Fail { name, args } => {
                let head = format!("Spec::Testing.fails({}, {subject})", code::string(name));
                if args.is_empty() {
                    head
                } else {
                    let data = code::list("", "[", &self.exprs(args)?, "]");
                    code::block(&head, &[], &[data]).text
                }
            }
        })
    }

    /// The place's root as it is read, and how a new value of it is stored.
    fn root(&mut self, place: &Place) -> (String, String) {
        match &place.root {
            PlaceRoot::Local(n) => {
                let ruby = self.lookup(n);
                (ruby.clone(), ruby)
            }
            PlaceRoot::State(s) => {
                let info = &self.g.lir.state(*s).info;
                let read = self.callee(info.module, &info.name, false);
                let write = if self.bare && self.module == Some(info.module) {
                    format!("self.{}", crate::names::method(&info.name))
                } else {
                    format!("{}.{}", self.module_ref(info.module), crate::names::method(&info.name))
                };
                (read, write)
            }
        }
    }

    fn written(&self, place: &Place) -> String {
        match &place.root {
            PlaceRoot::Local(n) => format!("`{n}`"),
            PlaceRoot::State(s) => format!("a write to {}", self.g.lir.state(*s).info.qualified),
        }
    }

    /// The steps of a place; a key that is not a literal or a local is evaluated once, into `lines`.
    fn steps(&mut self, place: &Place, hold: bool, lines: &mut Vec<String>) -> R<Vec<String>> {
        let mut steps = Vec::new();
        for step in &place.path {
            steps.push(match step {
                PlaceStep::Field { ty, index } => format!(":{}", self.member(*ty, None, *index)),
                PlaceStep::MapKey(k) => format!("Spec.key({})", self.held(k, hold, "key", lines)?),
                PlaceStep::ListIndex(k) => format!("Spec.index({})", self.held(k, hold, "index", lines)?),
            });
        }
        Ok(steps)
    }

    /// `e`, or a local that holds it when it must be evaluated here and once.
    fn held(&mut self, e: &Expr, hold: bool, name: &str, lines: &mut Vec<String>) -> R<String> {
        let c = self.expr(e)?.text;
        if !hold || repeatable(e) {
            return Ok(c);
        }
        let local = self.fresh(name);
        lines.push(assign(&local, &c));
        Ok(local)
    }

    /// `place = value`: the value first, then the keys, then the check, then the write.
    fn assign(&mut self, place: &Place, value: &Expr) -> R<Vec<String>> {
        let mut lines = Vec::new();
        let keyed = place.path.iter().any(|s| match s {
            PlaceStep::MapKey(k) | PlaceStep::ListIndex(k) => !repeatable(k),
            PlaceStep::Field { .. } => false,
        });
        let hold = !place.path.is_empty() && (writes(value) || keyed);
        let mut v = self.held(value, hold, "value", &mut lines)?;
        // A parameter the prologue checked against this very type needs no second check.
        let proved = matches!(&value.kind, ExprKind::Local(n) if !self.changes(n) && self.proved.iter().any(|(p, t)| p == n && Some(t) == place.check.as_ref()));
        if let Some(t) = place.check.as_ref().filter(|_| !proved) {
            v = self.checked(t, v, &self.written(place))?;
        }
        if let Some(k) = place.path.iter().find_map(|s| match s {
            PlaceStep::MapKey(k) | PlaceStep::ListIndex(k) if writes(k) => Some(k),
            _ => None,
        }) {
            return Err(self.g.unsupported(k.span, "a key of a written place that calls a rule"));
        }
        let steps = self.steps(place, false, &mut lines)?;
        let (read, write) = self.root(place);
        let new = match (steps.as_slice(), place.path.first()) {
            ([], _) => v,
            ([_], Some(PlaceStep::MapKey(k))) => {
                let key = self.expr(k)?.text;
                code::call("Spec::Maps.put", &[read, key, v]).text
            }
            _ => code::call("Spec.put", &[read, format!("[{}]", steps.join(", ")), v]).text,
        };
        lines.push(assign(&write, &new));
        Ok(lines)
    }

    /// `place.push(x)`: the arguments, the keys, the place's value, the primitive, the check, the write.
    fn mutate(&mut self, place: &Place, op: spec_lang_lir::Prim, args: &[Expr]) -> R<Vec<String>> {
        let mut lines = Vec::new();
        let deep = !place.path.is_empty();
        let mut codes = Vec::new();
        for a in args {
            let hold = deep || writes(a);
            codes.push(Code::atom(self.held(a, hold, "item", &mut lines)?));
        }
        let steps = self.steps(place, true, &mut lines)?;
        let (read, write) = self.root(place);
        let path = format!("[{}]", steps.join(", "));
        let current =
            if deep { code::call("Spec.dig", &[read.clone(), path.clone()]) } else { Code::atom(read.clone()) };
        codes.insert(0, current);
        let mut new = self.prim_of(op, codes)?.text;
        // Taking an item out of a collection leaves every other item as it was: nothing new to check.
        let removal = !deep
            && matches!(
                op,
                spec_lang_lir::Prim::MapRemove | spec_lang_lir::Prim::SetRemove | spec_lang_lir::Prim::ListRemove
            )
            && matches!(
                place.check,
                Some(spec_lang_lir::DTy::Map(..) | spec_lang_lir::DTy::Set(_) | spec_lang_lir::DTy::List(_))
            );
        if let Some(t) = place.check.as_ref().filter(|_| !removal) {
            new = self.checked(t, new, &self.written(place))?;
        }
        let stored = if deep { code::call("Spec.put", &[read, path, new]).text } else { new };
        lines.push(assign(&write, &stored));
        Ok(lines)
    }
}

fn paren_low(text: &str) -> String {
    if text.contains(" ? ") || text.starts_with("raise") { format!("({text})") } else { text.to_string() }
}

/// `name = value`, the value on its own lines when it is a statement that spans several.
pub(crate) fn assign(name: &str, value: &str) -> String {
    let opens = ["if ", "case ", "begin\n", "while "].iter().any(|k| value.starts_with(k));
    if value.contains('\n') && opens {
        format!("{name} =\n{}", code::indent(value))
    } else {
        format!("{name} = {value}")
    }
}

/// `if c ... elsif ... else ... end`; an `else` that is itself one `if` becomes `elsif`.
pub(crate) fn if_text(cond: &str, then: &[String], els: Option<&[String]>) -> String {
    let mut out = format!("if {cond}\n{}", code::indent_lines(then));
    match els {
        Some([one]) if one.starts_with("if ") && one.ends_with("\nend") => {
            out.push_str("\nels");
            out.push_str(one);
            return out.replace("\n\n", "\n");
        }
        Some(lines) if !lines.is_empty() => out.push_str(&format!("\nelse\n{}", code::indent_lines(lines))),
        _ => {}
    }
    out.push_str("\nend");
    out.replace("\n\nend", "\nend").replace("\n\nelse", "\nelse")
}
