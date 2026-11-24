//! Pattern matching. Bindings are pushed on the environment.

use super::Env;
use crate::error::Halted;
use crate::interp::Interp;
use crate::value::Value;
use spec_lang_ir::*;

impl Interp<'_> {
    pub(crate) fn matches(&mut self, env: &mut Env, v: &Value, p: &Pattern) -> Halted<bool> {
        match p {
            Pattern::Wild => Ok(true),
            Pattern::Bind(n) => {
                env.push(n, v.clone());
                Ok(true)
            }
            Pattern::Value(e) => {
                let x = self.eval(env, e)?;
                Ok(&x == v)
            }
            Pattern::Variant { ty, tag, args } => match v {
                Value::Variant(t, g, fields) if t == ty && g == tag => {
                    let names: Vec<String> = match &self.prog.ty(*ty).def {
                        TypeDef::Sum(vs) => vs
                            .get(*tag as usize)
                            .map(|x| x.fields.iter().map(|f| f.name.clone()).collect())
                            .unwrap_or_default(),
                        _ => Vec::new(),
                    };
                    self.match_args(env, fields, &names, args)
                }
                _ => Ok(false),
            },
            Pattern::Tuple(ps) => match v {
                Value::Tuple(xs) if xs.len() == ps.len() => {
                    for (x, p) in xs.iter().zip(ps) {
                        if !self.matches(env, x, p)? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                }
                _ => Ok(false),
            },
            Pattern::Call { rule, args } => match v {
                Value::Event(e) if e.rule == *rule => {
                    let names: Vec<String> = self.prog.rule(*rule).params.iter().map(|p| p.name.clone()).collect();
                    self.match_args(env, &e.args, &names, args)
                }
                _ => Ok(false),
            },
        }
    }

    /// Positional patterns match fields in order; named ones by name. Fields
    /// a pattern does not mention match anything.
    fn match_args(&mut self, env: &mut Env, fields: &[Value], names: &[String], args: &[PatArg]) -> Halted<bool> {
        for (i, a) in args.iter().enumerate() {
            let idx = match &a.name {
                Some(n) => names.iter().position(|x| x == n),
                None => Some(i),
            };
            let Some(x) = idx.and_then(|i| fields.get(i)) else {
                return Ok(false);
            };
            if !self.matches(env, x, &a.pattern)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
