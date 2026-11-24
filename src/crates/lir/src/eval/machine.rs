//! One evaluator over one world: calls, atomic rules, effects.

use super::value::{Failure, Value};
use crate::program::*;
use crate::ty::LTy;
use spec_lang_diag::{Diag, Severity};
use spec_lang_ir::{ConstId, FnId, HandlerId, RuleId};
use spec_lang_syntax::Span;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// Evaluation steps a machine may take before `E-RUNTIME`.
pub const BUDGET: u64 = 20_000_000;

/// Why evaluation did not give a value.
#[derive(Clone, Debug)]
pub enum Halt<'l> {
    /// A language failure: handled by `else`, a rule's refusal.
    Fail(Rc<Failure<'l>>),
    /// Evaluation stopped: never handled.
    Fatal(Box<Diag>),
    /// A test check did not hold.
    Check(Box<Diag>),
    /// A `?hole` (`F-BLOCKED`) or a foreign body (`B-FOREIGN`).
    Blocked(Box<Diag>),
}

pub type Halted<'l, T> = Result<T, Halt<'l>>;

impl<'l> Halt<'l> {
    pub fn fail(name: &str) -> Self {
        Halt::Fail(Rc::new(Failure { name: name.to_string(), data: Vec::new() }))
    }
    pub fn missing() -> Self {
        Halt::fail("Missing")
    }
}

impl From<Diag> for Halt<'_> {
    fn from(d: Diag) -> Self {
        Halt::Fatal(Box::new(d))
    }
}

pub(crate) fn runtime<'l>(span: Span, msg: impl Into<String>) -> Halt<'l> {
    Diag::error("E-RUNTIME", span, msg).into()
}

pub(crate) fn mismatch<'l>(span: Span, msg: impl Into<String>) -> Halt<'l> {
    Diag::error("E-TYPE-MISMATCH", span, msg).into()
}

/// Locals, innermost last.
#[derive(Clone, Debug, Default)]
pub struct Env<'l> {
    pub(crate) vars: Vec<(Rc<str>, Value<'l>)>,
}

impl<'l> Env<'l> {
    pub fn push(&mut self, name: &str, v: Value<'l>) {
        self.vars.push((Rc::from(name), v));
    }
    pub fn get(&self, name: &str) -> Option<&Value<'l>> {
        self.vars.iter().rev().find(|(n, _)| &**n == name).map(|(_, v)| v)
    }
    pub(crate) fn set(&mut self, name: &str, v: Value<'l>) -> bool {
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
    pub(crate) fn truncate(&mut self, m: usize) {
        self.vars.truncate(m);
    }
}

/// How a statement ended.
#[derive(Debug)]
pub enum Flow<'l> {
    Normal,
    Return(Value<'l>),
}

/// The evaluator: a program, a world, the handlers' pretend state.
#[derive(Debug)]
pub struct Machine<'l> {
    pub lir: &'l Lir,
    /// Every state, by `StateId`.
    pub world: Vec<Value<'l>>,
    /// Each handler's pretend state, by `HandlerId`.
    pub handlers: Vec<Vec<Value<'l>>>,
    consts: Vec<Option<Value<'l>>>,
    steps_left: u64,
}

impl<'l> Machine<'l> {
    /// A machine at the initial world.
    pub fn new(lir: &'l Lir) -> Halted<'l, Self> {
        let mut m = Machine {
            lir,
            world: Vec::new(),
            handlers: Vec::new(),
            consts: vec![None; lir.consts.len()],
            steps_left: BUDGET,
        };
        let mut states = Vec::new();
        for s in &lir.states {
            let v = match &s.init {
                Some(e) => m.eval(&mut Env::default(), e)?,
                None => m.empty_of(&s.ty).ok_or_else(|| {
                    Diag::error(
                        "E-TYPE-MISMATCH",
                        s.info.span,
                        "a state that is not a collection or an Option needs an initial value",
                    )
                    .with_decl(s.info.qualified.clone())
                })?,
            };
            states.push(v);
        }
        for (i, s) in lir.states.iter().enumerate() {
            if let Some(reason) = m.conform(&states[i], &s.check)? {
                return Err(Diag::error(
                    "E-CLAIM",
                    s.info.span,
                    format!("the initial value breaks its type: {reason}"),
                )
                .with_decl(s.info.qualified.clone())
                .into());
            }
        }
        m.world = states;
        for h in &lir.handlers {
            let mut state = Vec::new();
            for p in h.pretend.iter().flat_map(|p| &p.state) {
                let v = match &p.default {
                    Some(e) => m.eval(&mut Env::default(), e)?,
                    None => m.empty_of(&p.ty).unwrap_or(Value::Unit),
                };
                state.push(v);
            }
            m.handlers.push(state);
        }
        Ok(m)
    }

    /// The value a state "starts empty" with.
    pub fn empty_of(&self, ty: &LTy) -> Option<Value<'l>> {
        match ty {
            LTy::Map(..) => Some(Value::Map(Rc::new(BTreeMap::new()))),
            LTy::Set(_) => Some(Value::Set(Rc::new(BTreeSet::new()))),
            LTy::List(_) => Some(Value::list(Vec::new())),
            LTy::Option(_) => Some(self.none()),
            _ => None,
        }
    }

    pub(crate) fn none(&self) -> Value<'l> {
        Value::Variant(self.lir.option_type, spec_lang_ir::NONE_TAG, Rc::new(Vec::new()))
    }

    pub(crate) fn some(&self, v: Value<'l>) -> Value<'l> {
        Value::Variant(self.lir.option_type, spec_lang_ir::SOME_TAG, Rc::new(vec![v]))
    }

    pub(crate) fn option(&self, v: Option<Value<'l>>) -> Value<'l> {
        match v {
            Some(x) => self.some(x),
            None => self.none(),
        }
    }

    pub(crate) fn tick(&mut self, span: Span) -> Halted<'l, ()> {
        if self.steps_left == 0 {
            return Err(runtime(span, "the evaluation budget ran out (a loop that does not end?)"));
        }
        self.steps_left -= 1;
        Ok(())
    }

    pub(crate) fn constant(&mut self, c: ConstId) -> Halted<'l, Value<'l>> {
        if let Some(v) = &self.consts[c.index()] {
            return Ok(v.clone());
        }
        let v = self.eval(&mut Env::default(), &self.lir.constant(c).value)?;
        self.consts[c.index()] = Some(v.clone());
        Ok(v)
    }

    /// The prologue: defaults filled in order, then every parameter conformed.
    fn prologue(&mut self, decl: &Info, params: &'l [Param], args: Vec<Option<Value<'l>>>) -> Halted<'l, Env<'l>> {
        let mut env = Env::default();
        let mut vals = Vec::new();
        for (p, a) in params.iter().zip(args) {
            let v = match a {
                Some(v) => v,
                None => match &p.default {
                    Some(d) => self.eval(&mut env, d)?,
                    None => {
                        return Err(
                            Diag::error("E-TYPE-ARITY", decl.span, format!("missing argument `{}`", p.name)).into()
                        );
                    }
                },
            };
            env.push(&p.name, v.clone());
            vals.push(v);
        }
        for (p, v) in params.iter().zip(&vals) {
            if let Some(reason) = self.conform(v, &p.check)? {
                return Err(Diag::error("E-CLAIM", decl.span, format!("argument `{}`: {reason}", p.name))
                    .with_decl(decl.qualified.clone())
                    .into());
            }
        }
        Ok(env)
    }

    fn epilogue(&mut self, decl: &Info, check: &Option<crate::ty::DTy>, v: &Value<'l>) -> Halted<'l, ()> {
        if let Some(ty) = check
            && let Some(reason) = self.conform(v, ty)?
        {
            return Err(Diag::error("E-CLAIM", decl.span, format!("the result: {reason}"))
                .with_decl(decl.qualified.clone())
                .into());
        }
        Ok(())
    }

    fn foreign(decl: &Info, x: &Extern) -> Halt<'l> {
        let written =
            format!("foreign {} \"{}{}{}\"", x.lang, x.library, if x.library.is_empty() { "" } else { "#" }, x.symbol);
        Halt::Blocked(Box::new(
            Diag::new(
                Severity::Blocked,
                "B-FOREIGN",
                format!("reached {}, which is {written}: only a backend runs it", decl.qualified),
            )
            .with_span(decl.span)
            .with_expected("a body the interpreter runs", written),
        ))
    }

    pub fn call_fn(&mut self, f: FnId, args: Vec<Option<Value<'l>>>) -> Halted<'l, Value<'l>> {
        let d = self.lir.func(f);
        let mut env = self.prologue(&d.info, &d.params, args)?;
        let body = match &d.body {
            Body::Extern(x) => return Err(Self::foreign(&d.info, x)),
            Body::Expr(e) => e,
        };
        let v = self.eval(&mut env, body)?;
        self.epilogue(&d.info, &d.result_check, &v)?;
        Ok(v)
    }

    /// A rule call: one atomic step. On any halt the world and the
    /// handlers' state are restored to what they were before the call, also
    /// for a rule called inside a rule whose failure the caller handles
    /// (`inner(x) else v`: card, How it runs 1). Inside a rule it otherwise
    /// joins the caller's step: its writes commit with the caller's.
    pub fn call_rule(&mut self, r: RuleId, args: Vec<Option<Value<'l>>>) -> Halted<'l, Value<'l>> {
        let saved = (self.world.clone(), self.handlers.clone());
        let result = self.rule_body(r, args);
        if result.is_err() {
            (self.world, self.handlers) = saved;
        }
        result
    }

    fn rule_body(&mut self, r: RuleId, args: Vec<Option<Value<'l>>>) -> Halted<'l, Value<'l>> {
        let d = self.lir.rule(r);
        let mut env = self.prologue(&d.info, &d.params, args)?;
        for g in &d.guards {
            match self.eval(&mut env, &g.cond) {
                Ok(Value::Bool(true)) => {}
                Ok(Value::Bool(false)) | Err(Halt::Fail(_)) => return Err(Halt::fail(&g.error)),
                Ok(_) => return Err(mismatch(g.cond.span, "a guard condition is a Bool")),
                Err(h) => return Err(h),
            }
        }
        let body = match &d.body {
            Body::Extern(x) => return Err(Self::foreign(&d.info, x)),
            Body::Expr(e) => e,
        };
        let v = self.eval(&mut env, body)?;
        self.epilogue(&d.info, &d.result_check, &v)?;
        Ok(v)
    }

    /// A rule call from outside, every argument given: its value or refusal.
    pub fn step(&mut self, r: RuleId, args: Vec<Value<'l>>) -> Halted<'l, Result<Value<'l>, Rc<Failure<'l>>>> {
        match self.call_rule(r, args.into_iter().map(Some).collect()) {
            Ok(v) => Ok(Ok(v)),
            Err(Halt::Fail(f)) => Ok(Err(f)),
            Err(h) => Err(h),
        }
    }

    /// An operation of a handler, run by its pretend.
    pub(crate) fn effect(
        &mut self,
        h: HandlerId,
        op: u32,
        name: &str,
        args: Vec<Value<'l>>,
        span: Span,
    ) -> Halted<'l, Value<'l>> {
        let handler = self.lir.handler(h);
        let (Some(pretend), Some(o)) = (&handler.pretend, handler.ops.get(op as usize)) else {
            return Err(Diag::error(
                "E-UNSUPPORTED",
                span,
                format!("`{name}`: library `{}` has no pretend for handler `{}`", handler.library, handler.name),
            )
            .into());
        };
        let Some(Some(arm)) = pretend.arms.get(op as usize) else {
            return Err(Diag::error("E-UNSUPPORTED", span, format!("`{name}`: its pretend has no arm")).into());
        };
        for (p, a) in o.params.iter().flatten().zip(&args) {
            if let Some(reason) = self.conform(a, &p.check)? {
                return Err(Diag::error("E-CLAIM", span, format!("argument `{}` of `{name}`: {reason}", p.name)).into());
            }
        }
        let mut env = Env::default();
        let state = self.handlers.get(h.index()).cloned().unwrap_or_default();
        for (p, v) in pretend.state.iter().zip(state) {
            env.push(&p.name, v);
        }
        for ((n, _), v) in arm.params.iter().zip(args) {
            env.push(n, v);
        }
        let v = self.eval(&mut env, &arm.body)?;
        let state: Vec<Value<'l>> = pretend.state.iter().filter_map(|p| env.get(&p.name).cloned()).collect();
        if let Some(slot) = self.handlers.get_mut(h.index()) {
            *slot = state;
        }
        Ok(v)
    }

    /// The current world as a value.
    pub fn world_value(&self) -> Value<'l> {
        Value::World(Rc::new(self.world.clone()))
    }
}
