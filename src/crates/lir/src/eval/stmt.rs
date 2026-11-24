//! Statements, places and test steps.

use super::machine::{Env, Flow, Halt, Halted, Machine, mismatch, runtime};
use super::show::show;
use super::value::{Failure, Value};
use crate::node::*;
use spec_lang_diag::{Diag, Severity};
use spec_lang_syntax::Span;
use std::rc::Rc;

/// One step into a place, its key evaluated.
enum Key<'l> {
    Field(u32),
    Map(Value<'l>),
    List(Value<'l>),
}

impl<'l> Machine<'l> {
    pub(crate) fn exec_block(&mut self, env: &mut Env<'l>, b: &'l Block) -> Halted<'l, Flow<'l>> {
        let mark = env.mark();
        let r = self.exec_stmts(env, &b.stmts);
        env.truncate(mark);
        r
    }

    fn exec_stmts(&mut self, env: &mut Env<'l>, stmts: &'l [Stmt]) -> Halted<'l, Flow<'l>> {
        for s in stmts {
            if let Flow::Return(v) = self.exec(env, s)? {
                return Ok(Flow::Return(v));
            }
        }
        Ok(Flow::Normal)
    }

    pub fn exec(&mut self, env: &mut Env<'l>, s: &'l Stmt) -> Halted<'l, Flow<'l>> {
        self.tick(s.span)?;
        match &s.kind {
            StmtKind::Let { name, check, value, .. } => {
                let v = self.eval(env, value)?;
                if let Some(ty) = check
                    && let Some(reason) = self.conform(&v, ty)?
                {
                    return Err(Diag::error("E-CLAIM", s.span, format!("`{name}`: {reason}")).into());
                }
                env.push(name, v);
            }
            StmtKind::Assign { place, value } => {
                let v = self.eval(env, value)?;
                let keys = self.keys(env, place)?;
                self.write(env, place, &keys, v, s.span)?;
            }
            StmtKind::Mutate { place, op, args } => {
                let mut vals = self.eval_all(env, args)?;
                let keys = self.keys(env, place)?;
                let cur = self.read(env, place, &keys, s.span)?;
                vals.insert(0, cur);
                let new = self.apply_prim(*op, vals, s.span)?;
                self.write(env, place, &keys, new, s.span)?;
            }
            StmtKind::Expr(e) => {
                self.eval(env, e)?;
            }
            StmtKind::Return(v) => {
                let v = match v {
                    Some(e) => self.eval(env, e)?,
                    None => Value::Unit,
                };
                return Ok(Flow::Return(v));
            }
            StmtKind::Assert(e) => {
                if !self.truth(env, e)? {
                    return Err(Halt::Check(Box::new(
                        Diag::new(Severity::Failure, "F-TEST", "an assert is false").with_span(e.span),
                    )));
                }
            }
            StmtKind::For { var, iter, body, .. } => {
                let Value::List(items) = self.eval(env, iter)? else {
                    return Err(runtime(iter.span, "`for` over a value that is not a List"));
                };
                for x in items.iter() {
                    let mark = env.mark();
                    env.push(var, x.clone());
                    let r = self.exec_block(env, body);
                    env.truncate(mark);
                    if let Flow::Return(v) = r? {
                        return Ok(Flow::Return(v));
                    }
                }
            }
            StmtKind::While { cond, body } => {
                while self.truth(env, cond)? {
                    if let Flow::Return(v) = self.exec_block(env, body)? {
                        return Ok(Flow::Return(v));
                    }
                }
            }
            StmtKind::If { cond, then, els } => {
                let flow = if self.truth(env, cond)? {
                    self.exec_block(env, then)?
                } else {
                    match els {
                        Some(b) => self.exec_block(env, b)?,
                        None => Flow::Normal,
                    }
                };
                if let Flow::Return(v) = flow {
                    return Ok(Flow::Return(v));
                }
            }
            StmtKind::Expect { subject, expect } => self.expect(env, subject, expect, s.span)?,
        }
        Ok(Flow::Normal)
    }

    /// `subject -> value;` or `subject -> fail E;`.
    fn expect(&mut self, env: &mut Env<'l>, subject: &'l Expr, expect: &'l Expected, span: Span) -> Halted<'l, ()> {
        let got = match self.eval(env, subject) {
            Ok(v) => Ok(v),
            Err(Halt::Fail(f)) => Err(f),
            Err(h) => return Err(h),
        };
        let wanted = match expect {
            Expected::Value(e) => Ok(self.eval(env, e)?),
            Expected::Fail { name, args } => {
                Err(Rc::new(Failure { name: name.clone(), data: self.eval_all(env, args)? }))
            }
        };
        let ok = match (&got, &wanted) {
            (Ok(a), Ok(b)) => a == b,
            (Err(a), Err(b)) => a.name == b.name && (b.data.is_empty() || a.data == b.data),
            _ => false,
        };
        if ok {
            return Ok(());
        }
        let text = |m: &Machine<'l>, r: &Result<Value<'l>, Rc<Failure<'l>>>| match r {
            Ok(v) => show(m.lir, v),
            Err(f) => show(m.lir, &Value::Failure(f.clone())),
        };
        let (w, g) = (text(self, &wanted), text(self, &got));
        Err(Halt::Check(Box::new(
            Diag::new(Severity::Failure, "F-TEST", format!("the step gave {g}")).with_span(span).with_expected(w, g),
        )))
    }

    fn keys(&mut self, env: &mut Env<'l>, p: &'l Place) -> Halted<'l, Vec<Key<'l>>> {
        let mut out = Vec::with_capacity(p.path.len());
        for step in &p.path {
            out.push(match step {
                PlaceStep::Field { index, .. } => Key::Field(*index),
                PlaceStep::MapKey(k) => Key::Map(self.eval(env, k)?),
                PlaceStep::ListIndex(k) => Key::List(self.eval(env, k)?),
            });
        }
        Ok(out)
    }

    fn root(&self, env: &Env<'l>, p: &Place, span: Span) -> Halted<'l, Value<'l>> {
        match &p.root {
            PlaceRoot::State(s) => Ok(self.world[s.index()].clone()),
            PlaceRoot::Local(n) => env.get(n).cloned().ok_or_else(|| runtime(span, format!("`{n}` has no value"))),
        }
    }

    fn read(&mut self, env: &Env<'l>, p: &Place, keys: &[Key<'l>], span: Span) -> Halted<'l, Value<'l>> {
        let mut v = self.root(env, p, span)?;
        for k in keys {
            v = match (k, &v) {
                (Key::Field(i), Value::Record(_, fs)) => {
                    fs.get(*i as usize).cloned().ok_or_else(|| runtime(span, "no such field"))?
                }
                (Key::Map(k), Value::Map(m)) => m.get(k).cloned().ok_or_else(Halt::missing)?,
                (Key::List(Value::Int(i)), Value::List(xs)) => {
                    usize::try_from(*i).ok().and_then(|i| xs.get(i)).cloned().ok_or_else(Halt::missing)?
                }
                _ => return Err(runtime(span, "cannot read inside this value")),
            };
        }
        Ok(v)
    }

    /// Write `v` at a place: `conform` to its check first (a state place).
    fn write(&mut self, env: &mut Env<'l>, p: &'l Place, keys: &[Key<'l>], v: Value<'l>, span: Span) -> Halted<'l, ()> {
        if let Some(ty) = &p.check
            && let Some(reason) = self.conform(&v, ty)?
        {
            let name = match &p.root {
                PlaceRoot::State(s) => self.lir.state(*s).info.qualified.clone(),
                PlaceRoot::Local(n) => n.clone(),
            };
            return Err(Diag::error("E-CLAIM", span, format!("a write to {name} breaks its type: {reason}")).into());
        }
        let root = self.root(env, p, span)?;
        let new = update(root, keys, v, span)?;
        match &p.root {
            PlaceRoot::State(s) => self.world[s.index()] = new,
            PlaceRoot::Local(n) => {
                env.set(n, new);
            }
        }
        Ok(())
    }
}

fn update<'l>(cur: Value<'l>, keys: &[Key<'l>], v: Value<'l>, span: Span) -> Halted<'l, Value<'l>> {
    let Some((k, rest)) = keys.split_first() else { return Ok(v) };
    match (cur, k) {
        (Value::Map(mut m), Key::Map(key)) => {
            let inner = if rest.is_empty() {
                v
            } else {
                let old = m.get(key).cloned().ok_or_else(Halt::missing)?;
                update(old, rest, v, span)?
            };
            Rc::make_mut(&mut m).insert(key.clone(), inner);
            Ok(Value::Map(m))
        }
        (Value::List(mut xs), Key::List(Value::Int(i))) => {
            let i = usize::try_from(*i).ok().filter(|i| *i < xs.len()).ok_or_else(Halt::missing)?;
            let inner = update(xs[i].clone(), rest, v, span)?;
            Rc::make_mut(&mut xs)[i] = inner;
            Ok(Value::List(xs))
        }
        (Value::Record(t, mut fs), Key::Field(i)) => {
            let i = *i as usize;
            if i >= fs.len() {
                return Err(runtime(span, "no such field"));
            }
            let inner = update(fs[i].clone(), rest, v, span)?;
            Rc::make_mut(&mut fs)[i] = inner;
            Ok(Value::Record(t, fs))
        }
        _ => Err(mismatch(span, "cannot write inside this value")),
    }
}
