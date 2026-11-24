//! Whether a value is of a type, claims included. Claims are checked where a
//! value enters a type from outside: arguments, results, fields at
//! construction, state writes and `@given` values.

use crate::display::show;
use crate::error::{Halt, Halted};
use crate::eval::Env;
use crate::interp::Interp;
use crate::value::Value;
use spec_lang_ir::*;

impl Interp<'_> {
    /// `Ok(None)` when `v` is a `ty`; `Ok(Some(reason))` when it is not.
    pub fn conform(&mut self, v: &Value, ty: &Ty) -> Halted<Option<String>> {
        let no = |what: &str| Ok(Some(format!("{} is not {what}", show(self.prog, v))));
        match (ty, v) {
            (Ty::Unit, Value::Unit)
            | (Ty::Bool, Value::Bool(_))
            | (Ty::Int, Value::Int(_))
            | (Ty::Text, Value::Text(_)) => Ok(None),
            (Ty::Unit, _) => no("()"),
            (Ty::Bool, _) => no("a Bool"),
            (Ty::Int, _) => no("an Int"),
            (Ty::Text, _) => no("Text"),
            (Ty::Range { lo, hi, inclusive }, Value::Int(n)) => {
                let ok = *n >= *lo && if *inclusive { *n <= *hi } else { *n < *hi };
                if ok { Ok(None) } else { no(&format!("in {lo}{}{hi}", if *inclusive { "..=" } else { "..<" })) }
            }
            (Ty::Range { .. }, _) => no("an Int"),
            (Ty::Option(inner), Value::Variant(t, tag, fields)) if *t == self.prog.option_type => {
                if *tag == SOME_TAG {
                    match fields.first() {
                        Some(x) => self.conform(x, inner),
                        None => Ok(Some("Some without a value".into())),
                    }
                } else {
                    Ok(None)
                }
            }
            (Ty::Option(_), _) => no("an Option"),
            (Ty::List(t), Value::List(xs)) => self.all_conform(xs.iter(), t),
            (Ty::Set(t), Value::Set(xs)) => self.all_conform(xs.iter(), t),
            (Ty::Map(k, t), Value::Map(m)) => {
                if let Some(r) = self.all_conform(m.keys(), k)? {
                    return Ok(Some(r));
                }
                self.all_conform(m.values(), t)
            }
            (Ty::List(_), _) => no("a List"),
            (Ty::Set(_), _) => no("a Set"),
            (Ty::Map(..), _) => no("a Map"),
            (Ty::Tuple(ts), Value::Tuple(xs)) if ts.len() == xs.len() => {
                for (t, x) in ts.iter().zip(xs.iter()) {
                    if let Some(r) = self.conform(x, t)? {
                        return Ok(Some(r));
                    }
                }
                Ok(None)
            }
            (Ty::Tuple(ts), _) => no(&format!("a tuple of {}", ts.len())),
            (Ty::Where(base, claim), _) => {
                if let Some(r) = self.conform(v, base)? {
                    return Ok(Some(r));
                }
                let mut env = Env::default();
                env.push("self", v.clone());
                match self.eval(&mut env, &claim.expr) {
                    Ok(Value::Bool(true)) => Ok(None),
                    Ok(Value::Bool(false)) => Ok(Some(format!(
                        "`{}` is false for {}",
                        self.prog.snippet(claim.expr.span),
                        show(self.prog, v)
                    ))),
                    Ok(other) => Ok(Some(format!("the claim gave {} instead of a Bool", show(self.prog, &other)))),
                    Err(Halt::Fail(f)) => Ok(Some(format!("the claim failed with {}", f.name))),
                    Err(h) => Err(h),
                }
            }
            (Ty::Named(t), _) => match &self.prog.ty(*t).def {
                TypeDef::Alias(a) => self.conform(v, a),
                TypeDef::Record(fields) => match v {
                    Value::Record(u, vals) if u == t => self.fields_conform(fields, vals),
                    _ => no(&format!("a {}", self.prog.ty(*t).info.name)),
                },
                TypeDef::Sum(variants) => match v {
                    Value::Variant(u, tag, vals) if u == t => match variants.get(*tag as usize) {
                        // Option and Result carry the type of their payload in the use site, not here.
                        Some(var) if *t != self.prog.option_type && *t != self.prog.result_type => {
                            self.fields_conform(&var.fields, vals)
                        }
                        Some(_) => Ok(None),
                        None => no("a declared variant"),
                    },
                    _ => no(&format!("a {}", self.prog.ty(*t).info.name)),
                },
                TypeDef::Rules(rs) => match v {
                    Value::Event(e) if rs.contains(&e.rule) => Ok(None),
                    _ => no("a call of one of its rules"),
                },
            },
            (Ty::Trace(_), Value::Trace(_)) => Ok(None),
            (Ty::Trace(_), _) => no("a Trace"),
        }
    }

    /// Every field of a record or variant, claims included.
    fn fields_conform(&mut self, fields: &[Field], vals: &[Value]) -> Halted<Option<String>> {
        for (f, x) in fields.iter().zip(vals) {
            if let Some(r) = self.conform(x, &f.ty)? {
                return Ok(Some(format!("field `{}`: {r}", f.name)));
            }
        }
        Ok(None)
    }

    fn all_conform<'a>(&mut self, xs: impl Iterator<Item = &'a Value>, t: &Ty) -> Halted<Option<String>> {
        for x in xs {
            if let Some(r) = self.conform(x, t)? {
                return Ok(Some(r));
            }
        }
        Ok(None)
    }
}
