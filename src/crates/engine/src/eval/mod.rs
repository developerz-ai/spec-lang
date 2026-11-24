//! Expression evaluation.

mod call;
mod ops;
mod pattern;
mod stmt;

use crate::display::{interpolate, show};
use crate::error::{Halt, Halted};
use crate::interp::Interp;
use crate::value::{Closure, Failure, Value};
use spec_lang_diag::Diag;
use spec_lang_ir::*;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

pub use stmt::Flow;

/// Locals: parameters, `let`, `var`, loop and pattern bindings.
#[derive(Clone, Debug, Default)]
pub struct Env {
    vars: Vec<(Rc<str>, Value)>,
}

impl Env {
    pub fn push(&mut self, name: &str, v: Value) {
        self.vars.push((Rc::from(name), v));
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.vars.iter().rev().find(|(n, _)| &**n == name).map(|(_, v)| v)
    }

    pub(crate) fn set(&mut self, name: &str, v: Value) -> bool {
        match self.vars.iter_mut().rev().find(|(n, _)| &**n == name) {
            Some(slot) => {
                slot.1 = v;
                true
            }
            None => false,
        }
    }

    pub(crate) fn mark(&self) -> usize {
        self.vars.len()
    }

    pub(crate) fn truncate(&mut self, mark: usize) {
        self.vars.truncate(mark);
    }
}

pub(crate) fn runtime(span: spec_lang_syntax::Span, msg: impl Into<String>) -> Halt {
    Diag::error("E-RUNTIME", span, msg).into()
}

impl Interp<'_> {
    pub(crate) fn eval(&mut self, env: &mut Env, e: &Expr) -> Halted<Value> {
        self.tick(e.span)?;
        match &e.kind {
            ExprKind::Unit => Ok(Value::Unit),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Int(n) => Ok(Value::Int(*n)),
            ExprKind::Text(t) => Ok(Value::text(t)),
            ExprKind::Interp(parts) => {
                let mut s = String::new();
                for p in parts {
                    match p {
                        InterpPart::Lit(l) => s.push_str(l),
                        InterpPart::Expr(x) => {
                            let v = self.eval(env, x)?;
                            s.push_str(&interpolate(self.prog, &v));
                        }
                    }
                }
                Ok(Value::text(&s))
            }
            ExprKind::Local(n) => {
                env.get(n).cloned().ok_or_else(|| runtime(e.span, format!("`{n}` has no value here")))
            }
            ExprKind::Const(c) => self.constant(*c),
            ExprKind::State(s) => Ok(self.world.states[s.index()].clone()),
            ExprKind::Variant { ty, tag } => Ok(Value::Variant(*ty, *tag, Rc::new(Vec::new()))),
            ExprKind::Call { callee, args } => self.eval_call(env, *callee, args, e.span),
            ExprKind::CallValue { callee, args } => {
                let f = self.eval(env, callee)?;
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.eval(env, a)?);
                }
                self.apply(&f, vals, e.span)
            }
            ExprKind::Method { recv, name, args } => {
                let r = self.eval(env, recv)?;
                let mut vals = Vec::new();
                for a in args {
                    vals.push((a.name.clone(), self.eval(env, &a.value)?));
                }
                self.method(r, name, vals, e.span)
            }
            ExprKind::Prop { recv, name } => {
                let r = self.eval(env, recv)?;
                self.prop(r, name, e.span)
            }
            ExprKind::Index { recv, key } => {
                let r = self.eval(env, recv)?;
                let k = self.eval(env, key)?;
                self.index(&r, &k, e.span)
            }
            ExprKind::With { recv, fields } => {
                let r = self.eval(env, recv)?;
                let mut vals = Vec::new();
                for (n, x) in fields {
                    vals.push((n.clone(), self.eval(env, x)?));
                }
                self.with(r, vals, e.span)
            }
            ExprKind::Is { value, pattern } => {
                let v = self.eval(env, value)?;
                let mark = env.mark();
                let m = self.matches(env, &v, pattern);
                env.truncate(mark);
                Ok(Value::Bool(m?))
            }
            ExprKind::Unary(op, x) => {
                let v = self.eval(env, x)?;
                self.unary(*op, v, e.span)
            }
            ExprKind::Binary(op, l, r) => self.binary(env, *op, l, r, e.span),
            ExprKind::Compare(first, rest) => self.compare(env, first, rest),
            ExprKind::Range { lo, hi, inclusive } => {
                let lo = self.eval(env, lo)?;
                let hi = self.eval(env, hi)?;
                match (lo, hi) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Range(a, b, *inclusive)),
                    (a, b) => Err(runtime(
                        e.span,
                        format!("a range needs Int bounds, found {} and {}", show(self.prog, &a), show(self.prog, &b)),
                    )),
                }
            }
            ExprKind::Tuple(xs) => Ok(Value::Tuple(Rc::new(self.eval_all(env, xs)?))),
            ExprKind::List(xs) => Ok(Value::list(self.eval_all(env, xs)?)),
            ExprKind::SetOf(xs) => {
                let vals = self.eval_all(env, xs)?;
                for v in &vals {
                    self.need_comparable(v, e.span)?;
                }
                Ok(Value::Set(Rc::new(vals.into_iter().collect::<BTreeSet<_>>())))
            }
            ExprKind::MapOf(pairs) => {
                let mut m = BTreeMap::new();
                for (k, v) in pairs {
                    let k = self.eval(env, k)?;
                    self.need_comparable(&k, e.span)?;
                    let v = self.eval(env, v)?;
                    m.insert(k, v);
                }
                Ok(Value::Map(Rc::new(m)))
            }
            ExprKind::Lambda(id) => {
                let captured = env.vars.clone();
                Ok(Value::Func(Rc::new(Closure { lambda: *id, captured })))
            }
            ExprKind::If { cond, then, els } => match self.eval(env, cond)? {
                Value::Bool(true) => self.eval(env, then),
                Value::Bool(false) => self.eval(env, els),
                v => Err(self.not_bool(&v, cond.span)),
            },
            ExprKind::Else { value, fallback, unwrap_option } => match self.eval(env, value) {
                Ok(Value::Variant(t, tag, fields)) if *unwrap_option && t == self.prog.option_type => {
                    if tag == SOME_TAG {
                        Ok(fields.first().cloned().unwrap_or(Value::Unit))
                    } else {
                        self.eval(env, fallback)
                    }
                }
                Ok(v) => Ok(v),
                Err(Halt::Fail(_)) => self.eval(env, fallback),
                Err(h) => Err(h),
            },
            ExprKind::Fail(f) => {
                let data = self.eval_all(env, &f.args)?;
                Err(Halt::Fail(Rc::new(Failure { name: f.name.clone(), data })))
            }
            ExprKind::Match { scrutinee, arms } => {
                let v = self.eval(env, scrutinee)?;
                for arm in arms {
                    let mark = env.mark();
                    if self.matches(env, &v, &arm.pattern)? {
                        let r = self.eval(env, &arm.value);
                        env.truncate(mark);
                        return r;
                    }
                    env.truncate(mark);
                }
                Err(runtime(e.span, format!("no arm matches {}", show(self.prog, &v))))
            }
            ExprKind::Block(b) => match self.exec_block(env, b)? {
                Flow::Return(v) => Ok(v),
                Flow::Normal => Ok(Value::Unit),
            },
            ExprKind::OutText => Ok(Value::text(&self.out_text())),
            ExprKind::Hole(h) => Err(Halt::Blocked(h.clone(), e.span)),
        }
    }

    pub(crate) fn eval_all(&mut self, env: &mut Env, xs: &[Expr]) -> Halted<Vec<Value>> {
        let mut out = Vec::with_capacity(xs.len());
        for x in xs {
            out.push(self.eval(env, x)?);
        }
        Ok(out)
    }

    pub(crate) fn need_comparable(&self, v: &Value, span: spec_lang_syntax::Span) -> Halted<()> {
        if v.comparable() {
            Ok(())
        } else {
            Err(runtime(span, "a function is not a value that can be compared or kept in a Set or Map key"))
        }
    }

    pub(crate) fn not_bool(&self, v: &Value, span: spec_lang_syntax::Span) -> Halt {
        Diag::error("E-TYPE-MISMATCH", span, "a condition is a Bool").with_expected("Bool", show(self.prog, v)).into()
    }

    /// Call a function value with positional arguments.
    pub(crate) fn apply(&mut self, f: &Value, args: Vec<Value>, span: spec_lang_syntax::Span) -> Halted<Value> {
        let Value::Func(c) = f else {
            return Err(runtime(span, format!("{} is not a function", show(self.prog, f))));
        };
        let lambda = self.prog.lambda(c.lambda);
        if lambda.params.len() != args.len() {
            return Err(runtime(
                span,
                format!("the function takes {} argument(s), given {}", lambda.params.len(), args.len()),
            ));
        }
        let mut env = Env { vars: c.captured.clone() };
        for (p, a) in lambda.params.iter().zip(args) {
            env.push(p, a);
        }
        match &lambda.body {
            LambdaBody::Expr(x) => self.eval(&mut env, x),
            LambdaBody::Block(b) => match self.exec_block(&mut env, b)? {
                Flow::Return(v) => Ok(v),
                Flow::Normal => Ok(Value::Unit),
            },
        }
    }
}
