//! Calls with a known callee, record construction and `.with`.

use super::{Env, runtime};
use crate::display::{show, show_ty};
use crate::error::Halted;
use crate::interp::Interp;
use crate::value::Value;
use spec_lang_diag::Diag;
use spec_lang_ir::*;
use std::rc::Rc;

impl Interp<'_> {
    pub(crate) fn eval_call(
        &mut self,
        env: &mut Env,
        callee: Callee,
        args: &[Option<Expr>],
        span: spec_lang_syntax::Span,
    ) -> Halted<Value> {
        let mut vals = Vec::with_capacity(args.len());
        for a in args {
            vals.push(match a {
                Some(x) => Some(self.eval(env, x)?),
                None => None,
            });
        }
        match callee {
            Callee::Fn(f) => self.call_fn(f, vals),
            Callee::Rule(r) => self.call_rule(r, vals),
            Callee::Record(t) => {
                let TypeDef::Record(fields) = &self.prog.ty(t).def else {
                    return Err(runtime(span, "not a record type"));
                };
                let vals = self.fields_of(fields, vals, span)?;
                Ok(Value::Record(t, Rc::new(vals)))
            }
            Callee::Variant(t, tag) => {
                let TypeDef::Sum(vs) = &self.prog.ty(t).def else {
                    return Err(runtime(span, "not a sum type"));
                };
                let Some(v) = vs.get(tag as usize) else {
                    return Err(runtime(span, "no such variant"));
                };
                let vals = if t == self.prog.option_type {
                    vals.into_iter().flatten().collect()
                } else {
                    self.fields_of(&v.fields, vals, span)?
                };
                Ok(Value::Variant(t, tag, Rc::new(vals)))
            }
            Callee::Effect(e) => {
                let mut args = Vec::with_capacity(vals.len());
                for v in vals {
                    args.push(v.ok_or_else(|| runtime(span, "an effect takes every argument"))?);
                }
                self.run_effect(e, args, span)
            }
        }
    }

    /// Field values with defaults filled in and every claim checked.
    fn fields_of(
        &mut self,
        fields: &[Field],
        vals: Vec<Option<Value>>,
        span: spec_lang_syntax::Span,
    ) -> Halted<Vec<Value>> {
        let mut out = Vec::with_capacity(fields.len());
        for (f, v) in fields.iter().zip(vals) {
            let v = match v {
                Some(v) => v,
                None => match &f.default {
                    Some(d) => self.eval(&mut Env::default(), d)?,
                    None => return Err(runtime(span, format!("missing field `{}`", f.name))),
                },
            };
            self.check_field(f, &v, span)?;
            out.push(v);
        }
        Ok(out)
    }

    fn check_field(&mut self, f: &Field, v: &Value, span: spec_lang_syntax::Span) -> Halted<()> {
        if let Some(reason) = self.conform(v, &f.ty)? {
            return Err(Diag::error(
                "E-CLAIM",
                span,
                format!("field `{}` is not a {}: {reason}", f.name, show_ty(self.prog, &f.ty)),
            )
            .with_expected(show_ty(self.prog, &f.ty), show(self.prog, v))
            .into());
        }
        Ok(())
    }

    /// `r.with(f: v)`: a copy of a record with fields changed, claims checked.
    pub(crate) fn with(
        &mut self,
        r: Value,
        changes: Vec<(String, Value)>,
        span: spec_lang_syntax::Span,
    ) -> Halted<Value> {
        let Value::Record(t, vals) = r else {
            return Err(runtime(span, format!("`.with` needs a record, found {}", show(self.prog, &r))));
        };
        let TypeDef::Record(fields) = &self.prog.ty(t).def else {
            return Err(runtime(span, "not a record type"));
        };
        let mut vals = (*vals).clone();
        for (name, v) in changes {
            let Some(i) = fields.iter().position(|f| f.name == name) else {
                return Err(Diag::error(
                    "E-TYPE-ARITY",
                    span,
                    format!("{} has no field `{name}`", self.prog.ty(t).info.name),
                )
                .into());
            };
            self.check_field(&fields[i], &v, span)?;
            vals[i] = v;
        }
        Ok(Value::Record(t, Rc::new(vals)))
    }
}
