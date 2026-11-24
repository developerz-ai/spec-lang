//! Patterns: steps run in order against the scrutinee; the first failing test is no match.

use super::stmt::{always_returns, contains_return};
use super::{Code, Fx, Ret};
use spec_lang_lir::node::*;
use spec_lang_lir::{LTy, TypeDef};
use std::fmt::Write;

/// The static type at the end of a path: known, or a value whose type the pattern states.
enum At {
    Known(LTy),
    Dyn,
}

enum Step {
    Test(String),
    Bind(String),
}

impl<'g, 'a> Fx<'g, 'a> {
    /// The part of the scrutinee a path names.
    fn path(&mut self, scrut: &str, ty: &LTy, path: &[PathStep]) -> (String, At) {
        let mut cur = scrut.to_string();
        let mut at = At::Known(ty.clone());
        for step in path {
            (cur, at) = match (step, &at) {
                (PathStep::VariantField { .. }, At::Known(LTy::Option(t))) => {
                    (format!("(*{cur})"), At::Known((**t).clone()))
                }
                (PathStep::VariantField { tag, .. }, At::Known(LTy::Reply(rules))) => {
                    if *tag == spec_lang_ir::ERR_TAG {
                        (format!("{cur}.failure()"), At::Known(LTy::Failure))
                    } else {
                        let mut results = rules.iter().map(|r| &self.g.lir.rule(*r).result);
                        match results.next() {
                            Some(first) if results.all(|r| r == first) => {
                                (format!("{cur}.value().as<{}>()", self.g.ty(first)), At::Known(first.clone()))
                            }
                            _ => (format!("{cur}.value()"), At::Dyn),
                        }
                    }
                }
                (PathStep::VariantField { ty, tag, index }, At::Known(_)) => {
                    let f = &self.g.fields(*ty, Some(*tag))[*index as usize];
                    let alt = format!("std::get<{}>({cur}.alt)", self.g.variant_name(*ty, *tag));
                    (self.g.field(&alt, *ty, Some(*tag), *index), At::Known(f.ty.clone()))
                }
                (PathStep::TupleItem(i), At::Known(LTy::Tuple(ts))) => {
                    (format!("std::get<{i}>({cur})"), At::Known(ts[*i as usize].clone()))
                }
                (PathStep::EventArg { rule, index }, _) => {
                    let t = self.g.lir.rule(*rule).params[*index as usize].ty.clone();
                    (format!("{cur}.args.at({index}).as<{}>()", self.g.ty(&t)), At::Known(t))
                }
                _ => {
                    self.g.unsupported(self.span, "a pattern into a value whose type is not known");
                    (cur, At::Dyn)
                }
            };
        }
        (cur, at)
    }

    fn steps(&mut self, scrut: &str, ty: &LTy, pat: &Pat) -> Vec<Step> {
        let mut out = Vec::new();
        for s in &pat.steps {
            match s {
                PatStep::Tag { path, ty: sum, tag } => {
                    let (cur, at) = self.path(scrut, ty, path);
                    let first = *tag == 0;
                    out.push(Step::Test(match at {
                        At::Known(LTy::Option(_)) => format!("{}{cur}.has_value()", if first { "" } else { "!" }),
                        At::Known(LTy::Reply(_)) => format!("{}{cur}.ok()", if first { "" } else { "!" }),
                        At::Known(_) if self.g.is_enum(*sum) => format!("{cur} == {}", self.g.variant_name(*sum, *tag)),
                        At::Known(_) => {
                            format!("std::holds_alternative<{}>({cur}.alt)", self.g.variant_name(*sum, *tag))
                        }
                        At::Dyn => {
                            self.g
                                .unsupported(self.span, "a variant pattern on a reply of rules with different results");
                            "false".into()
                        }
                    }));
                }
                PatStep::Event { path, rule } => {
                    let (cur, _) = self.path(scrut, ty, path);
                    out.push(Step::Test(format!("{cur}.rule == {}", self.g.rule_const(*rule))));
                }
                PatStep::Equals { path, value } => {
                    let (cur, at) = self.path(scrut, ty, path);
                    let cur = match at {
                        At::Known(_) => cur,
                        At::Dyn => format!("{cur}.as<{}>()", self.g.ty(&value.ty)),
                    };
                    out.push(Step::Test(format!("{cur} == {}", self.expr(value).at(10))));
                }
                PatStep::Never => out.push(Step::Test("false".into())),
                PatStep::Bind { name, path, ty: bound } => {
                    let (cur, at) = self.path(scrut, ty, path);
                    let cur = match at {
                        At::Known(_) => cur,
                        At::Dyn => format!("{cur}.as<{}>()", self.g.ty(bound)),
                    };
                    let cpp = self.declare(name);
                    let unused = self.mark(&cpp);
                    out.push(Step::Bind(format!("{unused}const auto {cpp} = {cur};")));
                }
            }
        }
        out
    }

    /// The scrutinee as something a pattern can name more than once: itself when it is a place, else a copy.
    fn scrutinee(&mut self, e: &Expr, out: &mut String) -> String {
        if is_place(e) {
            return self.expr(e).at(16);
        }
        let name = self.fresh("subject");
        let _ = writeln!(out, "const auto {name} = {};", self.expr(e).at(2));
        name
    }

    /// `x.is(pattern)`: bindings do not escape.
    pub fn is(&mut self, value: &Expr, pat: &Pat) -> Code {
        let mut pre = String::new();
        self.push_scope();
        let scrut = self.scrutinee(value, &mut pre);
        let steps = self.steps(&scrut, &value.ty, pat);
        self.pop_scope();
        let tests_only = steps.iter().all(|s| matches!(s, Step::Test(_)));
        if tests_only && pre.is_empty() {
            let tests: Vec<&str> = steps
                .iter()
                .filter_map(|s| match s {
                    Step::Test(t) => Some(t.as_str()),
                    Step::Bind(_) => None,
                })
                .collect();
            return match tests.as_slice() {
                [] => Code::atom("true"),
                [one] => Code::new(*one, if one.contains(" == ") { 8 } else { 15 }),
                many => Code::new(many.join(" && "), 4),
            };
        }
        let (open, close) = nest(&steps);
        Code::atom(format!("[&]() -> bool {{\n{pre}{open}return true;\n{close}return false;\n}}()"))
    }

    /// A match as statements: the first arm whose pattern matches. `tail`: its value is the body's.
    pub fn match_(&mut self, scrutinee: &Expr, arms: &[Arm], out: &mut String, ret: Ret, tail: bool) {
        if !tail {
            // As a statement the arms' values are dropped and control goes on after the match.
            let leaves = arms.iter().any(|a| match &a.value.kind {
                ExprKind::Block(b) => contains_return(&b.stmts),
                _ => false,
            });
            let mut body = String::new();
            if leaves || !self.chain(scrutinee, arms, &mut body) {
                body.clear();
                self.match_(scrutinee, arms, &mut body, Ret::Void, true);
                let _ = writeln!(out, "[&] {{\n{body}}}();");
            } else {
                out.push_str(&body);
            }
            return;
        }
        let scrut = self.scrutinee(scrutinee, out);
        if self.switch(&scrut, scrutinee, arms, out, ret, true) {
            return;
        }
        let mut total = false;
        for arm in arms {
            self.push_scope();
            let steps = self.steps(&scrut, &scrutinee.ty, &arm.pat);
            let (open, close) = nest(&steps);
            total = steps.iter().all(|s| matches!(s, Step::Bind(_)));
            let braces = total && !steps.is_empty();
            if braces {
                out.push_str("{\n");
            }
            out.push_str(&open);
            self.arm(&arm.value, out, ret);
            out.push_str(&close);
            if braces {
                out.push_str("}\n");
            }
            self.pop_scope();
            if total {
                break;
            }
        }
        if !total {
            let _ = writeln!(out, "rt::no_arm(rt::str({scrut}));");
        }
    }

    /// An arm's value at the end of a body: after it, nothing else of the match runs.
    fn arm(&mut self, value: &Expr, out: &mut String, ret: Ret) {
        self.tail(value, out, ret);
        let returned = match &value.kind {
            ExprKind::Block(b) => always_returns(&b.stmts),
            _ => matches!(value.ty, LTy::Never),
        };
        if ret == Ret::Void && !returned {
            out.push_str("return;\n");
        }
    }

    /// A match on an `enum class` whose arms each name one variant: a `switch`.
    fn switch(&mut self, scrut: &str, scrutinee: &Expr, arms: &[Arm], out: &mut String, ret: Ret, tail: bool) -> bool {
        let LTy::Named(sum) = &scrutinee.ty else { return false };
        if !self.g.is_enum(*sum) {
            return false;
        }
        let mut cases = Vec::new();
        for (i, arm) in arms.iter().enumerate() {
            match arm.pat.steps.as_slice() {
                [PatStep::Tag { path, tag, .. }] if path.is_empty() => cases.push(Some(*tag)),
                [] if i + 1 == arms.len() => cases.push(None),
                _ => return false,
            }
        }
        let mut seen: Vec<u32> = Vec::new();
        for tag in cases.iter().flatten() {
            if seen.contains(tag) {
                return false;
            }
            seen.push(*tag);
        }
        let _ = writeln!(out, "switch ({scrut}) {{");
        for (arm, case) in arms.iter().zip(&cases) {
            match case {
                Some(tag) => {
                    let _ = writeln!(out, "case {}: {{", self.g.variant_name(*sum, *tag));
                }
                None => out.push_str("default: {\n"),
            }
            self.push_scope();
            if tail {
                self.arm(&arm.value, out, ret);
            } else {
                self.tail(&arm.value, out, Ret::Void);
                out.push_str("break;\n");
            }
            self.pop_scope();
            out.push_str("}\n");
        }
        let TypeDef::Sum { variants } = &self.g.lir.ty(*sum).def else { return true };
        let covered = cases.contains(&None) || seen.len() == variants.len();
        if !covered && !tail {
            let _ = writeln!(out, "default:\nrt::no_arm(rt::str({scrut}));");
        }
        out.push_str("}\n");
        if tail && !cases.contains(&None) {
            let _ = writeln!(out, "rt::no_arm(rt::str({scrut}));");
        }
        true
    }

    /// A statement match as `if / else if`: possible when every arm tests before it binds.
    fn chain(&mut self, scrutinee: &Expr, arms: &[Arm], out: &mut String) -> bool {
        let scrut = self.scrutinee(scrutinee, out);
        if self.switch(&scrut, scrutinee, arms, out, Ret::Void, false) {
            return true;
        }
        let mut total = false;
        for (i, arm) in arms.iter().enumerate() {
            self.push_scope();
            let steps = self.steps(&scrut, &scrutinee.ty, &arm.pat);
            let tests: Vec<&str> = steps
                .iter()
                .map_while(|s| match s {
                    Step::Test(t) => Some(t.as_str()),
                    Step::Bind(_) => None,
                })
                .collect();
            if steps[tests.len()..].iter().any(|s| matches!(s, Step::Test(_))) {
                self.pop_scope();
                return false;
            }
            let keyword = if i == 0 { "" } else { "} else " };
            if tests.is_empty() {
                total = true;
                out.push_str(if i == 0 { "{\n" } else { "} else {\n" });
            } else {
                let _ = writeln!(out, "{keyword}if ({}) {{", tests.join(" && "));
            }
            for s in &steps[tests.len()..] {
                if let Step::Bind(b) = s {
                    let _ = writeln!(out, "{b}");
                }
            }
            self.tail(&arm.value, out, Ret::Void);
            self.pop_scope();
            if total {
                break;
            }
        }
        if !total {
            let _ = writeln!(out, "}} else {{\nrt::no_arm(rt::str({scrut}));");
        }
        out.push_str("}\n");
        true
    }
}

/// The steps as nested `if`s and declarations: what opens them, and what closes them.
fn nest(steps: &[Step]) -> (String, String) {
    let mut open = String::new();
    let mut close = String::new();
    let mut tests: Vec<&str> = Vec::new();
    let flush = |tests: &mut Vec<&str>, open: &mut String, close: &mut String| {
        if !tests.is_empty() {
            let _ = writeln!(open, "if ({}) {{", tests.join(" && "));
            close.push_str("}\n");
            tests.clear();
        }
    };
    for s in steps {
        match s {
            Step::Test(t) => tests.push(t),
            Step::Bind(b) => {
                flush(&mut tests, &mut open, &mut close);
                let _ = writeln!(open, "{b}");
            }
        }
    }
    flush(&mut tests, &mut open, &mut close);
    (open, close)
}

/// Whether an expression names a value that stays where it is: reading it twice costs nothing and changes nothing.
pub fn is_place(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Local(_) | ExprKind::State(_) | ExprKind::Const(_) => true,
        ExprKind::Field { recv, .. } | ExprKind::WorldState { recv, .. } => is_place(recv),
        ExprKind::Prim { op, args } => {
            matches!(
                op,
                spec_lang_lir::Prim::StepEvent
                    | spec_lang_lir::Prim::StepReply
                    | spec_lang_lir::Prim::StepBefore
                    | spec_lang_lir::Prim::StepAfter
                    | spec_lang_lir::Prim::TraceSteps
                    | spec_lang_lir::Prim::TraceStates
            ) && args.first().is_some_and(is_place)
        }
        _ => false,
    }
}
