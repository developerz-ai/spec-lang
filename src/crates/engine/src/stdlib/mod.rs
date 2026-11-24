//! The core library (`docs/idea/28-standard-library.md`): properties are
//! read without parentheses (`xs.size`), methods called with them
//! (`xs.reversed()`). A name used the other way round is an error that says so.

mod seq;
mod text;

use crate::display::show;
use crate::error::{Halt, Halted};
use crate::eval::runtime;
use crate::interp::Interp;
use crate::value::Value;
use spec_lang_diag::Diag;
use spec_lang_ir::*;
use spec_lang_syntax::Span;
use std::rc::Rc;

/// Arguments of a method call: optional name, value.
pub(crate) type Args = Vec<(Option<String>, Value)>;

/// Names read without parentheses.
const PROPERTIES: &[&str] = &[
    "size", "bytes", "first", "last", "keys", "values", "list", "steps", "states", "event", "reply", "before", "after",
];

impl Interp<'_> {
    pub(crate) fn prop(&mut self, v: Value, name: &str, span: Span) -> Halted<Value> {
        let found = match &v {
            Value::Record(t, fields) => match &self.prog.ty(*t).def {
                TypeDef::Record(fs) => fs.iter().position(|f| f.name == name).and_then(|i| fields.get(i).cloned()),
                _ => None,
            },
            Value::Variant(t, tag, fields) => match &self.prog.ty(*t).def {
                TypeDef::Sum(vs) => vs
                    .get(*tag as usize)
                    .and_then(|x| x.fields.iter().position(|f| f.name == name))
                    .and_then(|i| fields.get(i).cloned()),
                _ => None,
            },
            Value::World(w) => {
                let ids: Vec<usize> =
                    self.prog.states.iter().enumerate().filter(|(_, s)| s.info.name == name).map(|(i, _)| i).collect();
                match ids.as_slice() {
                    [i] => w.states.get(*i).cloned(),
                    [] => None,
                    _ => return Err(runtime(span, format!("several modules have a state `{name}`"))),
                }
            }
            Value::Event(e) => {
                let r = self.prog.rule(e.rule);
                match r.params.iter().position(|p| p.name == name) {
                    Some(i) => e.args.get(i).cloned(),
                    None => {
                        return Err(Diag::error(
                            "E-TYPE-MISMATCH",
                            span,
                            format!("the event {} has no argument `{name}`", crate::display::call(self.prog, e)),
                        )
                        .into());
                    }
                }
            }
            Value::Step(s) => match name {
                "event" => Some(s.event.clone()),
                "reply" => Some(s.reply.clone()),
                "before" => Some(s.before.clone()),
                "after" => Some(s.after.clone()),
                "at" => return Err(Diag::unsupported(span, "`at` of a step: the engine has no clock").into()),
                _ => None,
            },
            Value::Trace(t) => match name {
                "steps" => Some(Value::list(t.steps.clone())),
                "states" => Some(Value::list(t.states.clone())),
                _ => None,
            },
            Value::Text(t) => match name {
                "size" => Some(Value::Int(t.chars().count() as i128)),
                "bytes" => Some(Value::list(t.bytes().map(|b| Value::Int(b.into())).collect())),
                _ => None,
            },
            Value::List(xs) => match name {
                "size" => Some(Value::Int(xs.len() as i128)),
                "first" => Some(xs.first().cloned().ok_or_else(Halt::missing)?),
                "last" => Some(xs.last().cloned().ok_or_else(Halt::missing)?),
                _ => None,
            },
            Value::Set(xs) => match name {
                "size" => Some(Value::Int(xs.len() as i128)),
                "list" => Some(Value::list(xs.iter().cloned().collect())),
                _ => None,
            },
            Value::Map(m) => match name {
                "size" => Some(Value::Int(m.len() as i128)),
                "keys" => Some(Value::list(m.keys().cloned().collect())),
                "values" => Some(Value::list(m.values().cloned().collect())),
                _ => None,
            },
            Value::Range(..) => match name {
                "size" => Some(Value::Int(self.items(v.clone(), span)?.len() as i128)),
                "list" => Some(Value::list(self.items(v.clone(), span)?)),
                _ => None,
            },
            _ => None,
        };
        found.ok_or_else(|| self.no_member(&v, name, false, span))
    }

    fn no_member(&self, v: &Value, name: &str, called: bool, span: Span) -> Halt {
        let mut d = Diag::error("E-TYPE-MISMATCH", span, format!("{} has no `{name}`", show(self.prog, v)));
        if called && PROPERTIES.contains(&name) {
            d = d.with_note(format!("`{name}` is read without parentheses: `x.{name}`"));
        } else if !called && !PROPERTIES.contains(&name) {
            d = d.with_note(format!("if `{name}` is a library method, call it: `x.{name}()`"));
        }
        d.into()
    }

    pub(crate) fn method(&mut self, v: Value, name: &str, args: Args, span: Span) -> Halted<Value> {
        let r = match &v {
            Value::Text(t) => self.text_method(t.clone(), name, &args, span)?,
            Value::List(xs) => self.seq_method(Seq::List, (**xs).clone(), name, args, span)?,
            Value::Set(xs) => self.seq_method(Seq::Set, xs.iter().cloned().collect(), name, args, span)?,
            Value::Range(..) => {
                let items = self.items(v.clone(), span)?;
                self.seq_method(Seq::Range, items, name, args, span)?
            }
            Value::Map(m) => match (name, args.as_slice()) {
                ("get", [(None, k)]) => Some(self.option(m.get(k).cloned())),
                _ => None,
            },
            Value::Int(n) => int_method(*n, name, &args),
            Value::Trace(_) if matches!(name, "eventually" | "within") => {
                return Err(Diag::unsupported(span, format!("`t.{name}`: liveness over traces")).into());
            }
            _ => None,
        };
        r.ok_or_else(|| self.no_member(&v, name, true, span))
    }

    pub(crate) fn option(&self, v: Option<Value>) -> Value {
        match v {
            Some(x) => Value::Variant(self.prog.option_type, SOME_TAG, Rc::new(vec![x])),
            None => Value::Variant(self.prog.option_type, NONE_TAG, Rc::new(Vec::new())),
        }
    }
}

/// Which collection a sequence method was called on: `map` of a `Set` is a `Set`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Seq {
    List,
    Set,
    Range,
}

fn int_method(n: i128, name: &str, args: &Args) -> Option<Value> {
    let ints: Option<Vec<i128>> = args
        .iter()
        .map(|(k, v)| if k.is_none() { if let Value::Int(x) = v { Some(*x) } else { None } } else { None })
        .collect();
    match (name, ints?.as_slice()) {
        ("abs", []) => n.checked_abs().map(Value::Int),
        ("min", [b]) => Some(Value::Int(n.min(*b))),
        ("max", [b]) => Some(Value::Int(n.max(*b))),
        ("clamp", [lo, hi]) if lo <= hi => Some(Value::Int(n.clamp(*lo, *hi))),
        _ => None,
    }
}

pub(crate) fn wrong_args(span: Span, name: &str, want: &str) -> Halt {
    Diag::error("E-TYPE-ARITY", span, format!("`{name}` takes {want}")).into()
}
