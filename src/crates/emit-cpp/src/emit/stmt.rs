//! Statements, places, and bodies that end by giving a value.

use super::expr::needs_statements;
use super::{Code, Fx, prims};
use crate::facts::any_order;
use crate::names::string_literal;
use spec_lang_lir::LTy;
use spec_lang_lir::node::*;
use std::fmt::Write;

/// What `return` means where a body is printed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ret {
    /// The C++ function returns the value.
    Value,
    /// The C++ function is `void`: the value is evaluated and dropped.
    Void,
}

impl<'g, 'a> Fx<'g, 'a> {
    /// The statements that end with the value of `e`.
    pub fn tail(&mut self, e: &Expr, out: &mut String, ret: Ret) {
        match &e.kind {
            ExprKind::Block(b) => self.block(b, out, ret),
            ExprKind::Match { scrutinee, arms } => self.match_(scrutinee, arms, out, ret, true),
            ExprKind::If { cond, then, els } if needs_statements(e) => {
                let _ = writeln!(out, "if ({}) {{", self.expr(cond).s);
                self.push_scope();
                self.tail(then, out, ret);
                self.pop_scope();
                out.push_str("} else {\n");
                self.push_scope();
                self.tail(els, out, ret);
                self.pop_scope();
                out.push_str("}\n");
            }
            _ => self.give(e, out, ret),
        }
    }

    /// `return e;`, as the body's kind of return.
    fn give(&mut self, e: &Expr, out: &mut String, ret: Ret) {
        match ret {
            Ret::Value => {
                let _ = writeln!(out, "return {};", self.expr(e).at(2));
            }
            Ret::Void => {
                self.discard(e, out);
            }
        }
    }

    /// `e;` for its effects.
    fn discard(&mut self, e: &Expr, out: &mut String) {
        if matches!(e.kind, ExprKind::Unit) {
            return;
        }
        let code = self.expr(e);
        // A call of a void function is printed as `call, rt::unit` where a value is wanted.
        if let Some(call) = code.s.strip_suffix(", rt::unit")
            && code.p == 1
        {
            let _ = writeln!(out, "{call};");
        } else if matches!(e.kind, ExprKind::CallFn { .. } | ExprKind::CallRule { .. } | ExprKind::Effect { .. } | ExprKind::Fail { .. } | ExprKind::Hole(_))
        {
            let _ = writeln!(out, "{};", code.s);
        } else {
            let _ = writeln!(out, "static_cast<void>({});", code.s);
        }
    }

    /// The statements of a block, in a scope of their own. A block that can end without `return` gives `()`.
    pub fn block(&mut self, b: &Block, out: &mut String, ret: Ret) {
        self.push_scope();
        for s in &b.stmts {
            self.stmt(s, out, ret);
        }
        self.pop_scope();
        if ret == Ret::Value && !always_returns(&b.stmts) {
            if b.ty == LTy::Unit {
                out.push_str("return rt::unit;\n");
            } else {
                out.push_str("rt::stop(\"E-RUNTIME\", \"a block ended without the value it gives\");\n");
            }
        }
    }

    fn nested(&mut self, b: &Block, out: &mut String, ret: Ret) {
        self.push_scope();
        for s in &b.stmts {
            self.stmt(s, out, ret);
        }
        self.pop_scope();
    }

    pub fn stmt(&mut self, s: &Stmt, out: &mut String, ret: Ret) {
        match &s.kind {
            StmtKind::Let { name, mutable, check, value, ty } => {
                let v = self.expr(value);
                let cpp = self.declare(name);
                let unused = self.mark(&cpp);
                let konst = if *mutable { "" } else { "const " };
                // A variable that is assigned later keeps its declared type, whatever the first value's C++ type.
                let decl = if *mutable || matches!(value.ty, LTy::Never) { self.g.ty(ty) } else { "auto".into() };
                let _ = writeln!(out, "{unused}{konst}{decl} {cpp} = {};", v.at(2));
                if let Some(c) = check.as_ref().and_then(|c| self.conform(c, &cpp)) {
                    let _ = writeln!(out, "rt::claim({c}, {});", string_literal(&format!("`{name}`")));
                }
            }
            StmtKind::Assign { place, value } => self.assign(place, value, out),
            StmtKind::Mutate { place, op, args } => self.mutate(place, *op, args, out),
            StmtKind::Expr(e) => match &e.kind {
                ExprKind::Match { scrutinee, arms } => self.match_(scrutinee, arms, out, Ret::Void, false),
                ExprKind::Block(b) if !contains_return(&b.stmts) => {
                    out.push_str("{\n");
                    self.nested(b, out, Ret::Void);
                    out.push_str("}\n");
                }
                ExprKind::Block(_) => {
                    let mut body = String::new();
                    self.tail(e, &mut body, Ret::Void);
                    let _ = writeln!(out, "[&] {{\n{body}}}();");
                }
                _ => self.discard(e, out),
            },
            StmtKind::Return(v) => match (v, ret) {
                (Some(e), Ret::Value) => {
                    let _ = writeln!(out, "return {};", self.expr(e).at(2));
                }
                (Some(e), Ret::Void) => {
                    self.discard(e, out);
                    out.push_str("return;\n");
                }
                (None, Ret::Value) => out.push_str("return rt::unit;\n"),
                (None, Ret::Void) => out.push_str("return;\n"),
            },
            StmtKind::Assert(e) => {
                let _ = writeln!(out, "rt::check({});", self.expr(e).s);
            }
            StmtKind::For { var, iter, body, .. } => {
                // The list is read once, before the loop: a place the body may change is copied first.
                let fx = self.g.facts.block(body);
                let place = matches!(iter.kind, ExprKind::Local(_) | ExprKind::State(_) | ExprKind::Field { .. });
                let mut items = self.expr(iter).at(2);
                if place && (fx.writes_local || fx.writes_state) {
                    let copy = self.fresh("items");
                    let _ = writeln!(out, "const auto {copy} = {items};");
                    items = copy;
                }
                self.push_scope();
                let name = self.declare(var);
                let unused = self.mark(&name);
                let _ = writeln!(out, "for ({unused}const auto& {name} : {items}) {{");
                self.nested(body, out, ret);
                self.pop_scope();
                out.push_str("}\n");
            }
            StmtKind::While { cond, body } => {
                let _ = writeln!(out, "while ({}) {{", self.expr(cond).s);
                out.push_str("rt::tick();\n");
                self.nested(body, out, ret);
                out.push_str("}\n");
            }
            StmtKind::If { cond, then, els } => {
                let _ = writeln!(out, "if ({}) {{", self.expr(cond).s);
                self.nested(then, out, ret);
                let mut els = els.as_ref();
                // `else { if c { .. } }` reads as `else if`.
                while let Some(b) = els {
                    match b.stmts.as_slice() {
                        [Stmt { kind: StmtKind::If { cond, then, els: next }, .. }] => {
                            let _ = writeln!(out, "}} else if ({}) {{", self.expr(cond).s);
                            self.nested(then, out, ret);
                            els = next.as_ref();
                        }
                        _ => {
                            out.push_str("} else {\n");
                            self.nested(b, out, ret);
                            els = None;
                        }
                    }
                }
                out.push_str("}\n");
            }
            StmtKind::Expect { subject, expect } => self.expect(subject, expect, out),
        }
    }

    fn expect(&mut self, subject: &Expr, expect: &Expected, out: &mut String) {
        match expect {
            Expected::Value(x) => {
                let s = self.thunk(subject);
                let want = self.thunk(x);
                let _ = writeln!(out, "rt::test::expect_value({s}, {want});");
            }
            Expected::Fail { name, args } => {
                let mut body = String::new();
                self.push_scope();
                self.tail(subject, &mut body, Ret::Void);
                self.pop_scope();
                let data: Vec<String> = args.iter().map(|a| format!("rt::Dyn{{{}}}", self.expr(a).at(2))).collect();
                let wanted = if data.is_empty() {
                    string_literal(name)
                } else {
                    format!("[&] {{ return rt::Failure{{{}, {{{}}}}}; }}", string_literal(name), data.join(", "))
                };
                let _ = writeln!(out, "rt::test::expect_fail([&] {{\n{body}}}, {wanted});");
            }
        }
    }

    /// The place as a C++ lvalue: every key but a last map key must be there (Missing).
    fn place(&mut self, p: &Place, keys: &[Code], read: bool) -> (String, Option<Code>) {
        let mut at = match &p.root {
            PlaceRoot::State(s) => format!("{}.world.{}", self.ctx(self.span), self.g.state_field(*s)),
            PlaceRoot::Local(n) => self.local(n),
        };
        let mut keys = keys.iter();
        for (i, step) in p.path.iter().enumerate() {
            let last = i + 1 == p.path.len();
            match step {
                PlaceStep::Field { ty, index } => {
                    at = self.g.field(&at, *ty, None, *index);
                }
                PlaceStep::MapKey(_) if last && !read => return (at, keys.next().cloned()),
                PlaceStep::MapKey(_) | PlaceStep::ListIndex(_) => {
                    let k = keys.next().map(|k| k.at(2)).unwrap_or_default();
                    at = format!("rt::at({at}, {k})");
                }
            }
        }
        (at, None)
    }

    fn key_exprs<'p>(p: &'p Place) -> Vec<&'p Expr> {
        p.path
            .iter()
            .filter_map(|s| match s {
                PlaceStep::MapKey(k) | PlaceStep::ListIndex(k) => Some(k),
                PlaceStep::Field { .. } => None,
            })
            .collect()
    }

    fn place_name(&self, p: &Place) -> String {
        match &p.root {
            PlaceRoot::State(s) => self.g.lir.state(*s).info.qualified.clone(),
            PlaceRoot::Local(n) => n.clone(),
        }
    }

    /// The value, then the keys, then `conform`, then the write.
    fn assign(&mut self, p: &Place, value: &Expr, out: &mut String) {
        let keys = Self::key_exprs(p);
        let mut parts = vec![value];
        parts.extend(keys.iter().copied());
        let fx: Vec<_> = parts.iter().map(|e| self.g.facts.effects(e)).collect();
        let full = self.expr(value);
        let mut v = Code::atom(self.expr_typed(value));
        let mut key_codes: Vec<Code> = keys.iter().map(|k| self.expr(k)).collect();
        let checked = p.check.as_ref().filter(|c| self.g.facts.checks(c));
        let hoist_keys = checked.is_some() && fx[1..].iter().any(|k| k.can_fail() || k.hard);
        if checked.is_some() || !any_order(&fx) {
            let name = self.fresh("value");
            let _ = writeln!(out, "{} {name} = {};", self.g.ty(&value_ty(p, value)), v.s);
            v = Code::atom(format!("std::move({name})"));
            if hoist_keys {
                for k in &mut key_codes {
                    let key = self.fresh("key");
                    let _ = writeln!(out, "const auto {key} = {};", k.at(2));
                    *k = Code::atom(key);
                }
            }
            if let Some(c) = checked.and_then(|c| self.conform(c, &name)) {
                let what = string_literal(&format!("a write to {}", self.place_name(p)));
                let _ = writeln!(out, "rt::claim({c}, {what});");
            }
        }
        match self.place(p, &key_codes, false) {
            (map, Some(key)) => {
                let v = if v.s == "{}" { full.at(2) } else { v.s };
                let _ = writeln!(out, "{map}.insert_or_assign({}, {v});", key.at(2));
            }
            (at, None) => {
                let _ = writeln!(out, "{at} = {};", v.s);
            }
        }
    }

    /// The arguments, then the keys, the place read, the primitive, `conform`, the write: done in place.
    fn mutate(&mut self, p: &Place, op: spec_lang_lir::Prim, args: &[Expr], out: &mut String) {
        let keys = Self::key_exprs(p);
        let mut parts: Vec<&Expr> = args.iter().collect();
        parts.extend(keys.iter().copied());
        let fx: Vec<_> = parts.iter().map(|e| self.g.facts.effects(e)).collect();
        let mut arg_codes: Vec<Code> = args.iter().map(|a| self.expr(a)).collect();
        if !any_order(&fx) {
            for a in &mut arg_codes {
                let name = self.fresh("arg");
                let _ = writeln!(out, "const auto {name} = {};", a.at(2));
                *a = Code::atom(name);
            }
        }
        let key_codes: Vec<Code> = keys.iter().map(|k| self.expr(k)).collect();
        let (mut at, _) = self.place(p, &key_codes, true);
        let checked = p.check.as_ref().filter(|c| self.g.facts.checks(c));
        if checked.is_some() && !keys.is_empty() {
            let name = self.fresh("place");
            let _ = writeln!(out, "auto& {name} = {at};");
            at = name;
        }
        match prims::in_place(op, &at, &arg_codes) {
            Some(stmt) => {
                let _ = writeln!(out, "{stmt}");
            }
            None => {
                let mut all = vec![Code::atom(at.clone())];
                all.extend(arg_codes);
                let _ = writeln!(out, "{at} = {};", prims::apply(op, &all).at(2));
            }
        }
        if let Some(c) = checked.and_then(|c| self.conform(c, &at)) {
            let what = string_literal(&format!("a write to {}", self.place_name(p)));
            let _ = writeln!(out, "rt::claim({c}, {what});");
        }
    }
}

/// The type a written value has: the place's.
fn value_ty(p: &Place, value: &Expr) -> LTy {
    if matches!(value.ty, LTy::Never) { p.ty.clone() } else { value.ty.clone() }
}

/// Whether every path through the statements ends in a `return` or never ends.
pub fn always_returns(stmts: &[Stmt]) -> bool {
    stmts.last().is_some_and(|s| match &s.kind {
        StmtKind::Return(_) => true,
        StmtKind::Expr(e) => matches!(e.ty, LTy::Never),
        StmtKind::If { then, els: Some(els), .. } => always_returns(&then.stmts) && always_returns(&els.stmts),
        _ => false,
    })
}

/// Whether a `return` of these statements would leave them (not one inside a nested block expression).
pub fn contains_return(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| match &s.kind {
        StmtKind::Return(_) => true,
        StmtKind::For { body, .. } | StmtKind::While { body, .. } => contains_return(&body.stmts),
        StmtKind::If { then, els, .. } => contains_return(&then.stmts) || els.as_ref().is_some_and(|b| contains_return(&b.stmts)),
        _ => false,
    })
}
