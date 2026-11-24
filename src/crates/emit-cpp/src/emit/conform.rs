//! `conform`: whether a value is of a declared type, claims included, as a
//! C++ condition. A type with nothing to check gives no condition.

use super::Fx;
use spec_lang_lir::DTy;

impl<'g, 'a> Fx<'g, 'a> {
    /// The condition that `v` (a C++ expression safe to repeat) conforms to `d`, or `None` when it always does.
    pub fn conform(&mut self, d: &DTy, v: &str) -> Option<String> {
        if !self.g.facts.checks(d) {
            return None;
        }
        match d {
            DTy::Unit | DTy::Bool | DTy::Int | DTy::Text | DTy::Trace(_) => None,
            DTy::Named(t) => Some(format!("{}({v})", self.g.valid_name(*t))),
            DTy::Range { lo, hi, inclusive } => {
                let lo = super::expr::int_literal(*lo).at(10);
                let hi = super::expr::int_literal(*hi).at(10);
                Some(format!("({v} >= {lo} && {v} {} {hi})", if *inclusive { "<=" } else { "<" }))
            }
            DTy::Option(inner) => {
                let c = self.conform(inner, &format!("(*{v})"))?;
                Some(format!("(!{v}.has_value() || {c})"))
            }
            DTy::List(t) | DTy::Set(t) => {
                let x = self.fresh("item");
                let c = self.conform(t, &x)?;
                Some(format!("std::ranges::all_of({v}, [](const auto& {x}) {{ return {c}; }})"))
            }
            DTy::Map(k, t) => {
                let x = self.fresh("entry");
                let mut both = Vec::new();
                both.extend(self.conform(k, &format!("{x}.first")));
                both.extend(self.conform(t, &format!("{x}.second")));
                // Keys first, then values.
                let parts: Vec<String> = both
                    .iter()
                    .map(|c| format!("std::ranges::all_of({v}, [](const auto& {x}) {{ return {c}; }})"))
                    .collect();
                Some(format!("({})", parts.join(" && ")))
            }
            DTy::Tuple(ts) => {
                let parts: Vec<String> = ts
                    .iter()
                    .enumerate()
                    .filter_map(|(i, t)| self.conform(t, &format!("std::get<{i}>({v})")))
                    .collect();
                Some(format!("({})", parts.join(" && ")))
            }
            DTy::Where(base, claim) => {
                let c = self.g.lir.claim(*claim);
                let mut parts: Vec<String> = self.conform(base, v).into_iter().collect();
                self.push_scope();
                self.bind("self", v);
                let body = self.expr(&c.body);
                self.pop_scope();
                // True holds; false or a failure does not.
                if self.g.facts.effects(&c.body).can_fail() {
                    parts.push(format!("rt::holds([&] {{ return {}; }})", body.s));
                } else {
                    parts.push(body.at(5));
                }
                Some(if parts.len() == 1 { parts.remove(0) } else { format!("({})", parts.join(" && ")) })
            }
        }
    }
}
