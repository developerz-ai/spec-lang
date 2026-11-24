//! Expressions.

use super::machine::{Env, Flow, Halt, Halted, Machine, mismatch, runtime};
use super::show::{interpolate, show};
use super::value::{Closure, Failure, Value};
use crate::node::*;
use crate::program::TypeDef;
use spec_lang_diag::Diag;
use spec_lang_syntax::Span;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

impl<'l> Machine<'l> {
    pub(crate) fn eval_all(&mut self, env: &mut Env<'l>, xs: &'l [Expr]) -> Halted<'l, Vec<Value<'l>>> {
        xs.iter().map(|x| self.eval(env, x)).collect()
    }

    fn eval_opt(&mut self, env: &mut Env<'l>, xs: &'l [Option<Expr>]) -> Halted<'l, Vec<Option<Value<'l>>>> {
        let mut out = Vec::with_capacity(xs.len());
        for x in xs {
            out.push(match x {
                Some(e) => Some(self.eval(env, e)?),
                None => None,
            });
        }
        Ok(out)
    }

    pub(crate) fn truth(&mut self, env: &mut Env<'l>, e: &'l Expr) -> Halted<'l, bool> {
        match self.eval(env, e)? {
            Value::Bool(b) => Ok(b),
            _ => Err(mismatch(e.span, "a condition is a Bool")),
        }
    }

    pub fn eval(&mut self, env: &mut Env<'l>, e: &'l Expr) -> Halted<'l, Value<'l>> {
        self.tick(e.span)?;
        let span = e.span;
        match &e.kind {
            ExprKind::Unit => Ok(Value::Unit),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Int(n) => Ok(Value::Int(*n)),
            ExprKind::Text(t) => Ok(Value::text(t)),
            ExprKind::Interp(parts) => {
                let mut s = String::new();
                for p in parts {
                    match p {
                        Part::Lit(l) => s.push_str(l),
                        Part::Show(x) => {
                            let v = self.eval(env, x)?;
                            s.push_str(&interpolate(self.lir, &v));
                        }
                    }
                }
                Ok(Value::text(&s))
            }
            ExprKind::Local(n) => env.get(n).cloned().ok_or_else(|| runtime(span, format!("`{n}` has no value here"))),
            ExprKind::Const(c) => self.constant(*c),
            ExprKind::State(s) => Ok(self.world[s.index()].clone()),
            ExprKind::Variant { ty, tag } => Ok(Value::Variant(*ty, *tag, Rc::new(Vec::new()))),
            ExprKind::Some(x) => {
                let v = self.eval(env, x)?;
                Ok(self.some(v))
            }
            ExprKind::None => Ok(self.none()),
            ExprKind::Construct { ty, tag, args } => {
                let vals = self.eval_opt(env, args)?;
                let fields = match (&self.lir.ty(*ty).def, tag) {
                    (TypeDef::Record { fields }, None) => fields,
                    (TypeDef::Sum { variants }, Some(t)) => match variants.get(*t as usize) {
                        Some(v) => &v.fields,
                        None => return Err(runtime(span, "no such variant")),
                    },
                    _ => return Err(runtime(span, "not a constructor")),
                };
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
                Ok(match tag {
                    Some(t) => Value::Variant(*ty, *t, Rc::new(out)),
                    None => Value::Record(*ty, Rc::new(out)),
                })
            }
            ExprKind::CallFn { func, args } => {
                let vals = self.eval_opt(env, args)?;
                self.call_fn(*func, vals)
            }
            ExprKind::CallRule { rule, args } => {
                let vals = self.eval_opt(env, args)?;
                self.call_rule(*rule, vals)
            }
            ExprKind::Effect { handler, op, name, args } => {
                let vals = self.eval_all(env, args)?;
                self.effect(*handler, *op, name, vals, span)
            }
            ExprKind::EffectRead { handler, op, name } => self.effect(*handler, *op, name, Vec::new(), span),
            ExprKind::Prim { op, args } => self.prim(env, *op, args, span),
            ExprKind::Lambda(l) => {
                let mut captured = Vec::with_capacity(l.captures.len());
                for (n, _) in &l.captures {
                    let v = env.get(n).cloned().ok_or_else(|| runtime(span, format!("`{n}` has no value here")))?;
                    captured.push((Rc::from(n.as_str()), v));
                }
                Ok(Value::Func(Rc::new(Closure { lambda: l, captured })))
            }
            ExprKind::CallLambda { callee, args } => {
                let f = self.eval(env, callee)?;
                let vals = self.eval_all(env, args)?;
                self.apply(&f, vals, span)
            }
            ExprKind::Field { recv, index, .. } => match self.eval(env, recv)? {
                Value::Record(_, fs) => fs.get(*index as usize).cloned().ok_or_else(|| runtime(span, "no such field")),
                _ => Err(runtime(span, "a field of a value that is not a record")),
            },
            ExprKind::VariantField { recv, name, by_tag, .. } => match self.eval(env, recv)? {
                Value::Variant(_, tag, fs) => by_tag
                    .get(tag as usize)
                    .copied()
                    .flatten()
                    .and_then(|i| fs.get(i as usize).cloned())
                    .ok_or_else(|| mismatch(span, format!("this variant has no `{name}`"))),
                _ => Err(runtime(span, "a field of a value that is not a sum")),
            },
            ExprKind::WorldState { recv, state } => match self.eval(env, recv)? {
                Value::World(w) => w.get(state.index()).cloned().ok_or_else(|| runtime(span, "no such state")),
                _ => Err(runtime(span, "a state of a value that is not a world")),
            },
            ExprKind::EventArg { recv, name, by_rule } => match self.eval(env, recv)? {
                Value::Event(ev) => by_rule
                    .iter()
                    .find(|(r, _)| *r == ev.rule)
                    .and_then(|(_, i)| *i)
                    .and_then(|i| ev.args.get(i as usize).cloned())
                    .ok_or_else(|| mismatch(span, format!("the event has no argument `{name}`"))),
                _ => Err(runtime(span, "an argument of a value that is not an event")),
            },
            ExprKind::With { recv, ty, changes } => {
                let r = self.eval(env, recv)?;
                let mut vals = Vec::new();
                for (i, x) in changes {
                    vals.push((*i, self.eval(env, x)?));
                }
                let Value::Record(t, fs) = r else { return Err(runtime(span, "`.with` needs a record")) };
                let TypeDef::Record { fields } = &self.lir.ty(*ty).def else {
                    return Err(runtime(span, "not a record type"));
                };
                let mut fs = (*fs).clone();
                for (i, v) in vals {
                    self.check_field(&fields[i as usize], &v, span)?;
                    fs[i as usize] = v;
                }
                Ok(Value::Record(t, Rc::new(fs)))
            }
            ExprKind::Is { value, pat } => {
                let v = self.eval(env, value)?;
                let mark = env.mark();
                let m = self.matches(env, &v, pat);
                env.truncate(mark);
                Ok(Value::Bool(m?))
            }
            ExprKind::If { cond, then, els } => {
                if self.truth(env, cond)? {
                    self.eval(env, then)
                } else {
                    self.eval(env, els)
                }
            }
            ExprKind::Logic { op, left, right } => {
                let l = self.truth(env, left)?;
                Ok(Value::Bool(match op {
                    Logic::And => l && self.truth(env, right)?,
                    Logic::Or => l || self.truth(env, right)?,
                    Logic::Implies => !l || self.truth(env, right)?,
                }))
            }
            ExprKind::Compare { first, links } => {
                let mut left = self.eval(env, first)?;
                for (op, x) in links {
                    let right = self.eval(env, x)?;
                    // `x in c`: a containment primitive takes the collection first.
                    let args = if op.is_containment() { vec![right.clone(), left] } else { vec![left, right.clone()] };
                    let held = match self.apply_prim(*op, args, x.span)? {
                        Value::Bool(b) => b,
                        _ => return Err(runtime(x.span, "a comparison gave no Bool")),
                    };
                    if !held {
                        return Ok(Value::Bool(false));
                    }
                    left = right;
                }
                Ok(Value::Bool(true))
            }
            ExprKind::Else { value, fallback, unwrap } => match self.eval(env, value) {
                Ok(Value::Variant(t, tag, fields)) if *unwrap && t == self.lir.option_type => {
                    if tag == spec_lang_ir::SOME_TAG {
                        Ok(fields.first().cloned().unwrap_or(Value::Unit))
                    } else {
                        self.eval(env, fallback)
                    }
                }
                Ok(v) => Ok(v),
                Err(Halt::Fail(_)) => self.eval(env, fallback),
                Err(h) => Err(h),
            },
            ExprKind::Fail { name, args } => {
                let data = self.eval_all(env, args)?;
                Err(Halt::Fail(Rc::new(Failure { name: name.clone(), data })))
            }
            ExprKind::Match { scrutinee, arms } => {
                let v = self.eval(env, scrutinee)?;
                for arm in arms {
                    let mark = env.mark();
                    let hit = self.matches(env, &v, &arm.pat);
                    if hit? {
                        let r = self.eval(env, &arm.value);
                        env.truncate(mark);
                        return r;
                    }
                    env.truncate(mark);
                }
                Err(runtime(span, format!("no arm matches {}", show(self.lir, &v))))
            }
            ExprKind::Block(b) => match self.exec_block(env, b)? {
                Flow::Return(v) => Ok(v),
                Flow::Normal => Ok(Value::Unit),
            },
            ExprKind::Hole(h) => Err(Halt::Blocked(Box::new(
                Diag::new(spec_lang_diag::Severity::Failure, "F-BLOCKED", format!("reached ?{h}")).with_span(span),
            ))),
            ExprKind::Tuple(xs) => Ok(Value::Tuple(Rc::new(self.eval_all(env, xs)?))),
            ExprKind::List(xs) => Ok(Value::list(self.eval_all(env, xs)?)),
            ExprKind::SetOf(xs) => {
                Ok(Value::Set(Rc::new(self.eval_all(env, xs)?.into_iter().collect::<BTreeSet<_>>())))
            }
            ExprKind::MapOf(pairs) => {
                let mut m = BTreeMap::new();
                for (k, v) in pairs {
                    let k = self.eval(env, k)?;
                    let v = self.eval(env, v)?;
                    m.insert(k, v);
                }
                Ok(Value::Map(Rc::new(m)))
            }
            ExprKind::Range { lo, hi, inclusive } => {
                let lo = self.eval(env, lo)?;
                let hi = self.eval(env, hi)?;
                match (lo, hi) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Range(a, b, *inclusive)),
                    _ => Err(runtime(span, "a range needs Int bounds")),
                }
            }
        }
    }

    pub(crate) fn check_field(&mut self, f: &crate::program::Field, v: &Value<'l>, span: Span) -> Halted<'l, ()> {
        if let Some(reason) = self.conform(v, &f.check)? {
            return Err(Diag::error("E-CLAIM", span, format!("field `{}`: {reason}", f.name)).into());
        }
        Ok(())
    }

    /// Call a function value.
    pub(crate) fn apply(&mut self, f: &Value<'l>, args: Vec<Value<'l>>, span: Span) -> Halted<'l, Value<'l>> {
        let Value::Func(c) = f else { return Err(runtime(span, "not a function")) };
        let lam = c.lambda;
        if lam.params.len() != args.len() {
            return Err(runtime(span, "a function called with the wrong number of arguments"));
        }
        let mut env = Env { vars: c.captured.clone() };
        for ((n, _), a) in lam.params.iter().zip(args) {
            env.push(n, a);
        }
        match &lam.body {
            LambdaBody::Expr(x) => self.eval(&mut env, x),
            LambdaBody::Block(b) => match self.exec_block(&mut env, b)? {
                Flow::Return(v) => Ok(v),
                Flow::Normal => Ok(Value::Unit),
            },
        }
    }

    /// Run a compiled pattern; bindings are pushed on `env`.
    pub(crate) fn matches(&mut self, env: &mut Env<'l>, v: &Value<'l>, p: &'l Pat) -> Halted<'l, bool> {
        for step in &p.steps {
            match step {
                PatStep::Tag { path, ty, tag } => match at(v, path) {
                    Some(Value::Variant(t, g, _)) if t == *ty && g == *tag => {}
                    _ => return Ok(false),
                },
                PatStep::Event { path, rule } => match at(v, path) {
                    Some(Value::Event(e)) if e.rule == *rule => {}
                    _ => return Ok(false),
                },
                PatStep::Equals { path, value } => {
                    let x = self.eval(env, value)?;
                    if at(v, path).as_ref() != Some(&x) {
                        return Ok(false);
                    }
                }
                PatStep::Never => return Ok(false),
                PatStep::Bind { name, path, .. } => match at(v, path) {
                    Some(x) => env.push(name, x),
                    None => return Ok(false),
                },
            }
        }
        Ok(true)
    }
}

/// The part of `v` a path names, if `v` has it.
fn at<'l>(v: &Value<'l>, path: &[PathStep]) -> Option<Value<'l>> {
    let mut cur = v.clone();
    for s in path {
        cur = match (s, &cur) {
            (PathStep::VariantField { ty, tag, index }, Value::Variant(t, g, fs)) if t == ty && g == tag => {
                fs.get(*index as usize)?.clone()
            }
            (PathStep::TupleItem(i), Value::Tuple(xs)) => xs.get(*i as usize)?.clone(),
            (PathStep::EventArg { rule, index }, Value::Event(e)) if e.rule == *rule => {
                e.args.get(*index as usize)?.clone()
            }
            _ => return None,
        };
    }
    Some(cur)
}
