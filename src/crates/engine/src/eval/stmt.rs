//! Statements, places, and test steps.

use super::{Env, runtime};
use crate::display::{show, show_ty};
use crate::error::{Halt, Halted};
use crate::interp::Interp;
use crate::value::{Failure, Value};
use spec_lang_diag::Diag;
use spec_lang_ir::*;
use std::rc::Rc;

/// How a statement or block ended.
#[derive(Debug)]
pub enum Flow {
    Normal,
    Return(Value),
}

impl Interp<'_> {
    pub(crate) fn exec_block(&mut self, env: &mut Env, b: &Block) -> Halted<Flow> {
        let mark = env.mark();
        let r = self.exec_stmts(env, &b.stmts);
        env.truncate(mark);
        r
    }

    fn exec_stmts(&mut self, env: &mut Env, stmts: &[Stmt]) -> Halted<Flow> {
        for s in stmts {
            if let Flow::Return(v) = self.exec(env, s)? {
                return Ok(Flow::Return(v));
            }
        }
        Ok(Flow::Normal)
    }

    /// Run one statement: tests run their bodies a statement at a time, so a
    /// failure names the step.
    pub fn exec(&mut self, env: &mut Env, s: &Stmt) -> Halted<Flow> {
        self.tick(s.span)?;
        match &s.kind {
            StmtKind::Let { name, ty, value, .. } => {
                let v = self.eval(env, value)?;
                if let Some(ty) = ty
                    && let Some(reason) = self.conform(&v, ty)?
                {
                    return Err(Diag::error(
                        "E-CLAIM",
                        s.span,
                        format!("`{name}` is not a {}: {reason}", show_ty(self.prog, ty)),
                    )
                    .into());
                }
                env.push(name, v);
            }
            StmtKind::Assign { place, value } => {
                let v = self.eval(env, value)?;
                self.assign(env, place, v)?;
            }
            StmtKind::Mutate { place, op, args } => {
                let args = self.eval_all(env, args)?;
                let cur = self.read_place(env, place)?;
                let new = self.mutated(cur, *op, args, s.span)?;
                self.assign(env, place, new)?;
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
            StmtKind::Assert(e) => match self.eval(env, e)? {
                Value::Bool(true) => {}
                Value::Bool(false) => return Err(self.assert_failed(env, e)),
                v => return Err(self.not_bool(&v, e.span)),
            },
            StmtKind::For { var, iter, body } => {
                let it = self.eval(env, iter)?;
                let items = self.items(it, iter.span)?;
                for x in items {
                    let mark = env.mark();
                    env.push(var, x);
                    let r = self.exec_block(env, body);
                    env.truncate(mark);
                    if let Flow::Return(v) = r? {
                        return Ok(Flow::Return(v));
                    }
                }
            }
            StmtKind::While { cond, body } => loop {
                match self.eval(env, cond)? {
                    Value::Bool(true) => {}
                    Value::Bool(false) => break,
                    v => return Err(self.not_bool(&v, cond.span)),
                }
                if let Flow::Return(v) = self.exec_block(env, body)? {
                    return Ok(Flow::Return(v));
                }
            },
            StmtKind::If { cond, then, els } => {
                let flow = match self.eval(env, cond)? {
                    Value::Bool(true) => self.exec_block(env, then)?,
                    Value::Bool(false) => match els {
                        Some(b) => self.exec_block(env, b)?,
                        None => Flow::Normal,
                    },
                    v => return Err(self.not_bool(&v, cond.span)),
                };
                if let Flow::Return(v) = flow {
                    return Ok(Flow::Return(v));
                }
            }
            StmtKind::Expect { subject, expect } => self.expect(env, subject, expect, s.span)?,
        }
        Ok(Flow::Normal)
    }

    /// The elements a `for` visits, computed once before the loop.
    pub(crate) fn items(&self, v: Value, span: spec_lang_syntax::Span) -> Halted<Vec<Value>> {
        match v {
            Value::List(xs) => Ok((*xs).clone()),
            Value::Set(xs) => Ok(xs.iter().cloned().collect()),
            Value::Range(lo, hi, incl) => {
                let end = if incl { hi } else { hi - 1 };
                if end.saturating_sub(lo) > 10_000_000 {
                    return Err(runtime(span, "a range of more than ten million numbers"));
                }
                Ok((lo..=end).map(Value::Int).collect())
            }
            other => Err(Diag::error(
                "E-TYPE-MISMATCH",
                span,
                format!("`for` needs a List, Set or range, found {}", show(self.prog, &other)),
            )
            .into()),
        }
    }

    fn assert_failed(&mut self, env: &mut Env, e: &Expr) -> Halt {
        let mut d = Diag::new(
            spec_lang_diag::Severity::Failure,
            "F-TEST",
            format!("assert {} is false", self.prog.snippet(e.span)),
        )
        .with_span(e.span);
        if let ExprKind::Compare(l, rest) = &e.kind
            && let [(CmpOp::Eq, r)] = rest.as_slice()
            && let (Ok(a), Ok(b)) = (self.eval(env, l), self.eval(env, r))
        {
            d = d.with_expected(show(self.prog, &b), show(self.prog, &a));
        }
        Halt::Check(Box::new(d))
    }

    /// A test step `subject -> value;` or `subject -> fail E;`.
    fn expect(&mut self, env: &mut Env, subject: &Expr, expect: &Expect, span: spec_lang_syntax::Span) -> Halted<()> {
        let got = match self.eval(env, subject) {
            Ok(v) => Ok(v),
            Err(Halt::Fail(f)) => Err(f),
            Err(h) => return Err(h),
        };
        let wanted = match expect {
            Expect::Value(e) => Ok(self.eval(env, e)?),
            Expect::Fail(f) => Err(Rc::new(Failure { name: f.name.clone(), data: self.eval_all(env, &f.args)? })),
        };
        let ok = match (&got, &wanted) {
            (Ok(a), Ok(b)) => a == b,
            (Err(a), Err(b)) => a.name == b.name && (b.data.is_empty() || a.data == b.data),
            _ => false,
        };
        if ok {
            return Ok(());
        }
        let text = |r: &Result<Value, Rc<Failure>>| match r {
            Ok(v) => show(self.prog, v),
            Err(f) => show(self.prog, &Value::Failure(f.clone())),
        };
        let d = Diag::new(
            spec_lang_diag::Severity::Failure,
            "F-TEST",
            format!("{} gave {}", self.prog.snippet(subject.span), text(&got)),
        )
        .with_span(span)
        .with_expected(text(&wanted), text(&got));
        Err(Halt::Check(Box::new(d)))
    }

    fn read_place(&mut self, env: &mut Env, p: &Place) -> Halted<Value> {
        let mut v = match &p.root {
            PlaceRoot::State(s) => self.world.states[s.index()].clone(),
            PlaceRoot::Local(n) => env.get(n).cloned().ok_or_else(|| runtime(p.span, format!("`{n}` has no value")))?,
        };
        for step in &p.path {
            v = match step {
                PlaceStep::Field(f) => self.prop(v, f, p.span)?,
                PlaceStep::Index(k) => {
                    let k = self.eval(env, k)?;
                    self.index(&v, &k, p.span)?
                }
            };
        }
        Ok(v)
    }

    /// Write `v` at a place. The value is checked against the place's type:
    /// a state write is where a claim is an obligation (O-Sub).
    fn assign(&mut self, env: &mut Env, p: &Place, v: Value) -> Halted<()> {
        let mut keys = Vec::new();
        for step in &p.path {
            keys.push(match step {
                PlaceStep::Field(f) => Key::Field(f.clone()),
                PlaceStep::Index(k) => Key::Index(self.eval(env, k)?),
            });
        }
        if let PlaceRoot::State(s) = &p.root {
            let ty = self.prog.state(*s).ty.clone();
            if let Some(leaf) = self.place_ty(&ty, &keys)
                && let Some(reason) = self.conform(&v, &leaf)?
            {
                let name = self.prog.qualified(&self.prog.state(*s).info);
                return Err(Diag::error(
                    "E-CLAIM",
                    p.span,
                    format!("a write to {name} breaks its type {}: {reason}", show_ty(self.prog, &leaf)),
                )
                .with_expected(show_ty(self.prog, &leaf), show(self.prog, &v))
                .into());
            }
        }
        let root = match &p.root {
            PlaceRoot::State(s) => self.world.states[s.index()].clone(),
            PlaceRoot::Local(n) => env.get(n).cloned().ok_or_else(|| runtime(p.span, format!("`{n}` has no value")))?,
        };
        let new = self.update(root, &keys, v, p.span)?;
        match &p.root {
            PlaceRoot::State(s) => self.world.states[s.index()] = new,
            PlaceRoot::Local(n) => {
                env.set(n, new);
            }
        }
        Ok(())
    }

    fn update(&self, cur: Value, keys: &[Key], v: Value, span: spec_lang_syntax::Span) -> Halted<Value> {
        let Some((k, rest)) = keys.split_first() else {
            return Ok(v);
        };
        match (cur, k) {
            (Value::Map(m), Key::Index(key)) => {
                let mut m = m;
                let inner = if rest.is_empty() {
                    v
                } else {
                    let old = m.get(key).cloned().ok_or_else(Halt::missing)?;
                    self.update(old, rest, v, span)?
                };
                Rc::make_mut(&mut m).insert(key.clone(), inner);
                Ok(Value::Map(m))
            }
            (Value::List(xs), Key::Index(Value::Int(i))) => {
                let mut xs = xs;
                let i = usize::try_from(*i).ok().filter(|i| *i < xs.len()).ok_or_else(Halt::missing)?;
                let inner = self.update(xs[i].clone(), rest, v, span)?;
                Rc::make_mut(&mut xs)[i] = inner;
                Ok(Value::List(xs))
            }
            (Value::Record(t, fs), Key::Field(f)) => {
                let TypeDef::Record(fields) = &self.prog.ty(t).def else {
                    return Err(runtime(span, "not a record"));
                };
                let i =
                    fields.iter().position(|x| &x.name == f).ok_or_else(|| runtime(span, format!("no field `{f}`")))?;
                let mut fs = fs;
                let inner = self.update(fs[i].clone(), rest, v, span)?;
                Rc::make_mut(&mut fs)[i] = inner;
                Ok(Value::Record(t, fs))
            }
            (cur, _) => Err(runtime(span, format!("cannot write inside {}", show(self.prog, &cur)))),
        }
    }

    fn place_ty(&self, ty: &Ty, keys: &[Key]) -> Option<Ty> {
        let Some((k, rest)) = keys.split_first() else {
            return Some(ty.clone());
        };
        match (ty, k) {
            (Ty::Map(_, v), Key::Index(_)) | (Ty::List(v), Key::Index(_)) => self.place_ty(v, rest),
            (Ty::Named(t), _) => match &self.prog.ty(*t).def {
                TypeDef::Alias(a) => self.place_ty(a, keys),
                TypeDef::Record(fs) => match k {
                    Key::Field(f) => fs.iter().find(|x| &x.name == f).and_then(|x| self.place_ty(&x.ty, rest)),
                    Key::Index(_) => None,
                },
                _ => None,
            },
            (Ty::Where(b, _), _) => self.place_ty(b, keys),
            _ => None,
        }
    }

    fn mutated(&mut self, cur: Value, op: MutOp, args: Vec<Value>, span: spec_lang_syntax::Span) -> Halted<Value> {
        match (cur, op, args.as_slice()) {
            (Value::List(xs), MutOp::Push, [x]) => {
                let mut xs = xs;
                Rc::make_mut(&mut xs).push(x.clone());
                Ok(Value::List(xs))
            }
            (Value::List(xs), MutOp::Insert, [Value::Int(at), x]) => {
                let mut xs = xs;
                let at = usize::try_from(*at).ok().filter(|i| *i <= xs.len()).ok_or_else(Halt::missing)?;
                Rc::make_mut(&mut xs).insert(at, x.clone());
                Ok(Value::List(xs))
            }
            (Value::List(xs), MutOp::Remove, [Value::Int(at)]) => {
                let mut xs = xs;
                let at = usize::try_from(*at).ok().filter(|i| *i < xs.len()).ok_or_else(Halt::missing)?;
                Rc::make_mut(&mut xs).remove(at);
                Ok(Value::List(xs))
            }
            (Value::Set(xs), MutOp::Insert, [x]) => {
                self.need_comparable(x, span)?;
                let mut xs = xs;
                Rc::make_mut(&mut xs).insert(x.clone());
                Ok(Value::Set(xs))
            }
            (Value::Set(xs), MutOp::Remove, [x]) => {
                let mut xs = xs;
                Rc::make_mut(&mut xs).remove(x);
                Ok(Value::Set(xs))
            }
            (Value::Map(m), MutOp::Remove, [k]) => {
                let mut m = m;
                Rc::make_mut(&mut m).remove(k);
                Ok(Value::Map(m))
            }
            (cur, op, _) => Err(Diag::error(
                "E-TYPE-MISMATCH",
                span,
                format!("`{op:?}` with these arguments does not apply to {}", show(self.prog, &cur)),
            )
            .into()),
        }
    }
}

/// One step into a place.
enum Key {
    Field(String),
    Index(Value),
}
