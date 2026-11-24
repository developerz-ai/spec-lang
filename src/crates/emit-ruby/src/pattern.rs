//! `match` and `.is(...)`: a compiled pattern's tests become one Ruby condition, its bindings assignments.

use crate::code::{self, ATOM, Code};
use crate::gen_::{Fx, R};
use crate::stmt::assign;
use spec_lang_lir::node::*;
use spec_lang_lir::{LTy, TypeDef, walk};

/// One arm as Ruby: the condition (none: it always matches), then what to bind.
struct Tests {
    cond: Option<String>,
    binds: Vec<String>,
}

impl Fx<'_> {
    /// The value at `path` of the scrutinee.
    fn access(&self, subject: &str, path: &[PathStep]) -> String {
        let mut out = subject.to_string();
        for step in path {
            match step {
                PathStep::VariantField { ty, tag, index } => {
                    out.push_str(&format!(".{}", self.member(*ty, Some(*tag), *index)));
                }
                PathStep::TupleItem(i) => out.push_str(&format!("[{i}]")),
                PathStep::EventArg { index, .. } => out.push_str(&format!(".args[{index}]")),
            }
        }
        out
    }

    /// Whether the variant is one value (no fields) or a class of values.
    fn variant_is_class(&self, ty: spec_lang_ir::TypeId, tag: u32) -> bool {
        let lir = self.g.lir;
        if ty == lir.option_type {
            return tag == spec_lang_ir::SOME_TAG;
        }
        if ty == lir.result_type {
            return true;
        }
        match &lir.ty(ty).def {
            TypeDef::Sum { variants } => variants.get(tag as usize).is_some_and(|v| !v.fields.is_empty()),
            _ => false,
        }
    }

    fn tag_test(&self, value: &str, ty: spec_lang_ir::TypeId, tag: u32) -> String {
        let variant = self.variant_ref(ty, tag);
        if self.variant_is_class(ty, tag) {
            format!("{value}.is_a?({variant})")
        } else {
            format!("{value} == {variant}")
        }
    }

    /// The steps of a pattern against `subject`. Bindings are made in the current scope.
    fn tests(&mut self, pat: &Pat, subject: &str) -> R<Tests> {
        let bound: Vec<&str> = pat
            .steps
            .iter()
            .filter_map(|s| if let PatStep::Bind { name, .. } = s { Some(name.as_str()) } else { None })
            .collect();
        // A value compared against may name an earlier binding of the same pattern: then they bind on the way.
        let entangled = pat.steps.iter().any(|s| match s {
            PatStep::Equals { value, .. } => {
                let mut found = false;
                walk::exprs(value, &mut |x| {
                    found |= matches!(&x.kind, ExprKind::Local(n) if bound.contains(&n.as_str()))
                });
                found
            }
            _ => false,
        });
        let mut conds = Vec::new();
        let mut binds = Vec::new();
        for step in &pat.steps {
            match step {
                PatStep::Tag { path, ty, tag } => conds.push(self.tag_test(&self.access(subject, path), *ty, *tag)),
                PatStep::Event { path, rule } => {
                    let name = code::string(&self.g.lir.rule(*rule).info.qualified);
                    conds.push(format!("{}.call_of?({name})", self.access(subject, path)));
                }
                PatStep::Equals { path, value } => {
                    let v = self.expr(value)?;
                    conds.push(format!("{} == {}", self.access(subject, path), v.at(code::CMP)));
                }
                PatStep::Never => conds.push("false".into()),
                PatStep::Bind { name, path, .. } => {
                    let at = self.access(subject, path);
                    let ruby = self.bind(name, false);
                    if entangled {
                        conds.push(format!("({ruby} = {at}; true)"));
                    } else {
                        binds.push(assign(&ruby, &at));
                    }
                }
            }
        }
        Ok(Tests { cond: if conds.is_empty() { None } else { Some(conds.join(" && ")) }, binds })
    }

    /// `x.is(pattern)`.
    pub(crate) fn is(&mut self, value: &Expr, pat: &Pat) -> R<Code> {
        let v = self.expr(value)?;
        let once = pat.steps.len() <= 1 || matches!(value.kind, ExprKind::Local(_));
        let subject = if once { v.at(ATOM) } else { self.fresh("subject") };
        self.push();
        let tests = self.tests(pat, &subject);
        self.pop();
        let cond = tests?.cond.unwrap_or_else(|| "true".into());
        Ok(match (once, pat.steps.len()) {
            (true, 0) => Code::atom(format!("({}; true)", v.text)),
            (true, 1) => Code::new(cond.clone(), if cond.contains(" == ") { code::EQ } else { ATOM }),
            (true, _) => Code::new(cond, code::AND),
            (false, _) => Code::atom(format!("({subject} = {}; {cond})", v.text)),
        })
    }

    /// The `when` of an arm when the whole `match` can be a `case`: one test of the scrutinee itself against a
    /// variant or a literal (`None`: the arm always matches).
    fn when_of(&self, pat: &Pat) -> Option<Option<String>> {
        let mut tests = pat.steps.iter().filter(|s| !matches!(s, PatStep::Bind { .. }));
        let when = match (tests.next(), tests.next()) {
            (None, _) => None,
            (Some(PatStep::Tag { path, ty, tag }), None) if path.is_empty() => Some(self.variant_ref(*ty, *tag)),
            (Some(PatStep::Equals { path, value }), None) if path.is_empty() => match &value.kind {
                ExprKind::Int(n) => Some(n.to_string()),
                ExprKind::Text(t) => Some(code::string(t)),
                ExprKind::Bool(b) => Some(b.to_string()),
                ExprKind::Variant { ty, tag } => Some(self.variant_ref(*ty, *tag)),
                _ => return None,
            },
            _ => return None,
        };
        Some(when)
    }

    /// Whether the arms name every variant of the scrutinee's sum, each by one test.
    fn exhaustive(&self, scrutinee: &Expr, arms: &[Arm]) -> bool {
        let count = match &scrutinee.ty {
            LTy::Option(_) => 2,
            LTy::Named(t) => match &self.g.lir.ty(*t).def {
                TypeDef::Sum { variants } => variants.len(),
                _ => return false,
            },
            _ => return false,
        };
        let mut tags: Vec<u32> = arms
            .iter()
            .filter_map(|a| match a.pat.steps.iter().find(|s| !matches!(s, PatStep::Bind { .. })) {
                Some(PatStep::Tag { path, tag, .. }) if path.is_empty() => Some(*tag),
                _ => None,
            })
            .collect();
        tags.sort_unstable();
        tags.dedup();
        tags.len() == count
    }

    /// A `match`: the first arm whose pattern matches. `value` asks for the arm's value.
    pub(crate) fn match_lines(&mut self, scrutinee: &Expr, arms: &[Arm], value: bool) -> R<Vec<String>> {
        let mut lines = Vec::new();
        let subject = match &scrutinee.kind {
            ExprKind::Local(n) => self.lookup(n),
            _ => {
                let v = self.expr(scrutinee)?.text;
                let name = self.fresh("subject");
                lines.push(assign(&name, &v));
                name
            }
        };
        let whens: Option<Vec<Option<String>>> = arms.iter().map(|a| self.when_of(&a.pat)).collect();
        let none = format!("Spec.stop!(\"E-RUNTIME\", \"no arm matches #{{Spec.show({subject})}}\")");
        let mut branches: Vec<(Option<String>, Vec<String>)> = Vec::new();
        for (i, arm) in arms.iter().enumerate() {
            self.push();
            let tests = self.tests(&arm.pat, &subject);
            let body = tests.and_then(|t| {
                let mut body = t.binds;
                body.extend(if value { self.value_lines(&arm.value)? } else { self.effect_lines(&arm.value)? });
                if value && body.is_empty() {
                    body.push("nil".into());
                }
                Ok((t.cond, body))
            });
            self.pop();
            let (cond, body) = body?;
            let cond = match &whens {
                Some(w) => w.get(i).cloned().flatten(),
                None => cond,
            };
            let always = cond.is_none();
            branches.push((cond, body));
            if always {
                break; // later arms are never reached
            }
        }
        let closed = branches.last().is_some_and(|(c, _)| c.is_none()) || self.exhaustive(scrutinee, arms);
        if !closed {
            branches.push((None, vec![none]));
        }
        if let [(None, body)] = branches.as_slice() {
            lines.extend(body.iter().cloned());
            return Ok(lines);
        }
        lines.push(if whens.is_some() { case_text(&subject, &branches) } else { chain_text(&branches) });
        Ok(lines)
    }
}

fn case_text(subject: &str, branches: &[(Option<String>, Vec<String>)]) -> String {
    let short = branches.iter().all(|(c, b)| {
        b.len() == 1 && !b[0].contains('\n') && c.as_ref().map_or(0, String::len) + b[0].len() + 11 <= code::WIDTH
    });
    let mut out = format!("case {subject}");
    for (cond, body) in branches {
        match (cond, short) {
            (Some(c), true) => out.push_str(&format!("\nwhen {c} then {}", body[0])),
            (Some(c), false) => out.push_str(&format!("\nwhen {c}\n{}", code::indent_lines(body))),
            (None, true) => out.push_str(&format!("\nelse {}", body[0])),
            (None, false) => out.push_str(&format!("\nelse\n{}", code::indent_lines(body))),
        }
    }
    out.push_str("\nend");
    out
}

fn chain_text(branches: &[(Option<String>, Vec<String>)]) -> String {
    let Some(((first, body), rest)) = branches.split_first() else { return "nil".into() };
    let mut out = format!("if {}\n{}", first.as_deref().unwrap_or("true"), code::indent_lines(body));
    for (cond, body) in rest {
        match cond {
            Some(c) => out.push_str(&format!("\nelsif {c}\n{}", code::indent_lines(body))),
            None => out.push_str(&format!("\nelse\n{}", code::indent_lines(body))),
        }
    }
    out.push_str("\nend");
    out
}
