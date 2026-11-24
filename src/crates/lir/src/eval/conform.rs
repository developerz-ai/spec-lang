//! `conform`: whether a value is of a declared type, claims included. The
//! base is checked first, then the claim; collections item by item (a map's
//! keys, then its values); records and variants field by field.

use super::machine::{Env, Halt, Halted, Machine};
use super::show::show;
use super::value::Value;
use crate::program::TypeDef;
use crate::ty::DTy;

impl<'l> Machine<'l> {
    /// `Ok(None)`: it is; `Ok(Some(reason))`: it is not.
    pub fn conform(&mut self, v: &Value<'l>, ty: &DTy) -> Halted<'l, Option<String>> {
        let lir = self.lir;
        let no = |what: &str| Ok(Some(format!("{} is not {what}", show(lir, v))));
        match (ty, v) {
            (DTy::Unit, Value::Unit)
            | (DTy::Bool, Value::Bool(_))
            | (DTy::Int, Value::Int(_))
            | (DTy::Text, Value::Text(_)) => Ok(None),
            (DTy::Unit, _) => no("()"),
            (DTy::Bool, _) => no("a Bool"),
            (DTy::Int, _) => no("an Int"),
            (DTy::Text, _) => no("Text"),
            (DTy::Range { lo, hi, inclusive }, Value::Int(n)) => {
                let ok = *n >= *lo && if *inclusive { *n <= *hi } else { *n < *hi };
                if ok { Ok(None) } else { no(&format!("in {lo}{}{hi}", if *inclusive { "..=" } else { "..<" })) }
            }
            (DTy::Range { .. }, _) => no("an Int"),
            (DTy::Option(inner), Value::Variant(t, tag, fields)) if *t == lir.option_type => {
                if *tag == spec_lang_ir::SOME_TAG {
                    match fields.first() {
                        Some(x) => self.conform(x, inner),
                        None => Ok(Some("Some without a value".into())),
                    }
                } else {
                    Ok(None)
                }
            }
            (DTy::Option(_), _) => no("an Option"),
            (DTy::List(t), Value::List(xs)) => self.all(xs.iter(), t),
            (DTy::Set(t), Value::Set(xs)) => self.all(xs.iter(), t),
            (DTy::Map(k, t), Value::Map(m)) => {
                if let Some(r) = self.all(m.keys(), k)? {
                    return Ok(Some(r));
                }
                self.all(m.values(), t)
            }
            (DTy::List(_), _) => no("a List"),
            (DTy::Set(_), _) => no("a Set"),
            (DTy::Map(..), _) => no("a Map"),
            (DTy::Tuple(ts), Value::Tuple(xs)) if ts.len() == xs.len() => {
                for (t, x) in ts.iter().zip(xs.iter()) {
                    if let Some(r) = self.conform(x, t)? {
                        return Ok(Some(r));
                    }
                }
                Ok(None)
            }
            (DTy::Tuple(ts), _) => no(&format!("a tuple of {}", ts.len())),
            (DTy::Where(base, claim), _) => {
                if let Some(r) = self.conform(v, base)? {
                    return Ok(Some(r));
                }
                let c = lir.claim(*claim);
                let mut env = Env::default();
                env.push("self", v.clone());
                match self.eval(&mut env, &c.body) {
                    Ok(Value::Bool(true)) => Ok(None),
                    Ok(Value::Bool(false)) => Ok(Some(format!("`{}` is false for {}", c.text, show(lir, v)))),
                    Ok(other) => Ok(Some(format!("the claim gave {} instead of a Bool", show(lir, &other)))),
                    Err(Halt::Fail(f)) => Ok(Some(format!("the claim failed with {}", f.name))),
                    Err(h) => Err(h),
                }
            }
            (DTy::Named(t), _) => {
                let decl = lir.ty(*t);
                match &decl.def {
                    TypeDef::Alias { of } => self.conform(v, of),
                    TypeDef::Record { fields } => match v {
                        Value::Record(u, vals) if u == t => {
                            for (f, x) in fields.iter().zip(vals.iter()) {
                                if let Some(r) = self.conform(x, &f.check)? {
                                    return Ok(Some(format!("field `{}`: {r}", f.name)));
                                }
                            }
                            Ok(None)
                        }
                        _ => no(&format!("a {}", decl.info.name)),
                    },
                    TypeDef::Sum { variants } => match v {
                        Value::Variant(u, tag, vals) if u == t => match variants.get(*tag as usize) {
                            // Option and Result carry their payload's type at the use site.
                            Some(_) if decl.builtin => Ok(None),
                            Some(var) => {
                                for (f, x) in var.fields.iter().zip(vals.iter()) {
                                    if let Some(r) = self.conform(x, &f.check)? {
                                        return Ok(Some(format!("field `{}`: {r}", f.name)));
                                    }
                                }
                                Ok(None)
                            }
                            None => no("a declared variant"),
                        },
                        _ => no(&format!("a {}", decl.info.name)),
                    },
                    TypeDef::RuleSet { rules } => match v {
                        Value::Event(e) if rules.contains(&e.rule) => Ok(None),
                        _ => no("a call of one of its rules"),
                    },
                }
            }
            (DTy::Trace(_), Value::Trace(_)) => Ok(None),
            (DTy::Trace(_), _) => no("a Trace"),
        }
    }

    fn all<'a>(&mut self, xs: impl Iterator<Item = &'a Value<'l>>, t: &DTy) -> Halted<'l, Option<String>>
    where
        'l: 'a,
    {
        for x in xs {
            if let Some(r) = self.conform(x, t)? {
                return Ok(Some(r));
            }
        }
        Ok(None)
    }
}
