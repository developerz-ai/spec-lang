//! The interpreter: calls, atomic rules, the world.

use crate::error::{Halt, Halted};
use crate::eval::Env;
use crate::value::{Failure, Value};
use crate::world::World;
use spec_lang_diag::Diag;
use spec_lang_ir::*;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// The default evaluation budget per run: steps of the evaluator.
pub const DEFAULT_BUDGET: u64 = 20_000_000;

/// How a rule call ended.
#[derive(Clone, Debug)]
pub enum Outcome {
    Ok(Value),
    /// Refused with a failure; the world is unchanged.
    Refused(Rc<Failure>),
}

/// One interpreter over one program and one world.
#[derive(Debug)]
pub struct Interp<'p> {
    pub prog: &'p Program,
    pub world: World,
    /// The state of each library handler's pretend (`pretend.slg`), by
    /// handler id; empty for a handler without one. Committed and rolled
    /// back with the world; a new interpreter starts it fresh.
    pub handlers: Vec<Vec<Value>>,
    /// Depth of rule calls: 0 means a call from outside.
    pub(crate) depth: u32,
    consts: Vec<Option<Value>>,
    /// Evaluation steps left before `E-RUNTIME`.
    pub steps_left: u64,
}

impl<'p> Interp<'p> {
    /// An interpreter at the initial world: every state at its initial value.
    pub fn new(prog: &'p Program) -> Halted<Self> {
        let mut it = Interp {
            prog,
            world: World { states: Vec::new() },
            handlers: Vec::new(),
            depth: 0,
            consts: vec![None; prog.consts.len()],
            steps_left: DEFAULT_BUDGET,
        };
        let mut states = Vec::new();
        for s in &prog.states {
            let v = match &s.init {
                Some(e) => it.eval(&mut Env::default(), e)?,
                None => it.empty_of(&s.ty).ok_or_else(|| {
                    Diag::error(
                        "E-TYPE-MISMATCH",
                        s.info.span,
                        "a state that is not a collection or an Option needs an initial value",
                    )
                    .in_decl(&prog.qualified(&s.info))
                })?,
            };
            states.push(v);
        }
        for (i, s) in prog.states.iter().enumerate() {
            if let Some(reason) = it.conform(&states[i], &s.ty)? {
                return Err(Diag::error(
                    "E-CLAIM",
                    s.info.span,
                    format!("the initial value breaks its type: {reason}"),
                )
                .in_decl(&prog.qualified(&s.info))
                .into());
            }
        }
        it.world.states = states;
        for h in &prog.handlers {
            let mut state = Vec::new();
            for p in h.pretend.iter().flat_map(|p| &p.state) {
                let v = match &p.default {
                    Some(e) => it.eval(&mut Env::default(), e)?,
                    None => it.empty_of(&p.ty).unwrap_or(Value::Unit),
                };
                state.push(v);
            }
            it.handlers.push(state);
        }
        Ok(it)
    }

    /// The empty value of a collection or `Option` type: where a state "starts empty".
    pub fn empty_of(&self, ty: &Ty) -> Option<Value> {
        match ty {
            Ty::Map(..) => Some(Value::Map(Rc::new(BTreeMap::new()))),
            Ty::Set(_) => Some(Value::Set(Rc::new(BTreeSet::new()))),
            Ty::List(_) => Some(Value::list(Vec::new())),
            Ty::Option(_) => Some(Value::Variant(self.prog.option_type, NONE_TAG, Rc::new(Vec::new()))),
            Ty::Where(b, _) => self.empty_of(b),
            Ty::Named(t) => match &self.prog.ty(*t).def {
                TypeDef::Alias(a) => self.empty_of(a),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn tick(&mut self, span: spec_lang_syntax::Span) -> Halted<()> {
        if self.steps_left == 0 {
            return Err(
                Diag::error("E-RUNTIME", span, "the evaluation budget ran out (a loop that does not end?)").into()
            );
        }
        self.steps_left -= 1;
        Ok(())
    }

    pub(crate) fn constant(&mut self, c: ConstId) -> Halted<Value> {
        if let Some(v) = &self.consts[c.index()] {
            return Ok(v.clone());
        }
        let v = self.eval(&mut Env::default(), &self.prog.constant(c).value)?;
        self.consts[c.index()] = Some(v.clone());
        Ok(v)
    }

    /// Fill defaults: `None` arguments take the parameter's default,
    /// evaluated with the earlier parameters in scope.
    pub(crate) fn complete_args(&mut self, params: &[Param], args: Vec<Option<Value>>) -> Halted<(Env, Vec<Value>)> {
        let mut env = Env::default();
        let mut out = Vec::new();
        for (p, a) in params.iter().zip(args) {
            let v = match a {
                Some(v) => v,
                None => match &p.default {
                    Some(d) => self.eval(&mut env, d)?,
                    None => {
                        return Err(
                            Diag::error("E-TYPE-ARITY", p.span, format!("missing argument `{}`", p.name)).into()
                        );
                    }
                },
            };
            env.push(&p.name, v.clone());
            out.push(v);
        }
        Ok((env, out))
    }

    fn check_params(&mut self, decl: &DeclInfo, params: &[Param], args: &[Value]) -> Halted<()> {
        for (p, a) in params.iter().zip(args) {
            if let Some(reason) = self.conform(a, &p.ty)? {
                return Err(Diag::error(
                    "E-CLAIM",
                    p.span,
                    format!("argument `{}` is not a {}: {reason}", p.name, crate::display::show_ty(self.prog, &p.ty)),
                )
                .with_decl(self.prog.qualified(decl))
                .with_expected(crate::display::show_ty(self.prog, &p.ty), crate::display::show(self.prog, a))
                .into());
            }
        }
        Ok(())
    }

    fn check_result(&mut self, decl: &DeclInfo, result: &Option<Ty>, v: &Value) -> Halted<()> {
        if let Some(ty) = result
            && let Some(reason) = self.conform(v, ty)?
        {
            return Err(Diag::error(
                "E-CLAIM",
                decl.name_span,
                format!("the result is not a {}: {reason}", crate::display::show_ty(self.prog, ty)),
            )
            .with_decl(self.prog.qualified(decl))
            .with_expected(crate::display::show_ty(self.prog, ty), crate::display::show(self.prog, v))
            .into());
        }
        Ok(())
    }

    /// Call a `fn`. Arguments and result are checked against their types,
    /// claims included.
    pub fn call_fn(&mut self, f: FnId, args: Vec<Option<Value>>) -> Halted<Value> {
        let d = self.prog.func(f);
        let (mut env, vals) = self.complete_args(&d.params, args)?;
        self.check_params(&d.info, &d.params, &vals)?;
        if let Some(f) = &d.foreign {
            return Err(Halt::foreign(&self.prog.qualified(&d.info), f));
        }
        let v = self.eval(&mut env, &d.body)?;
        self.check_result(&d.info, &d.result, &v)?;
        Ok(v)
    }

    /// Call a rule. From outside (depth 0) it is one atomic step: a failure
    /// restores the world and every handler's pretend state (the lines it
    /// sent); a success commits them. Inside a rule it joins the caller.
    pub fn call_rule(&mut self, r: RuleId, args: Vec<Option<Value>>) -> Halted<Value> {
        let outside = self.depth == 0;
        let saved = if outside { Some((self.world.clone(), self.handlers.clone())) } else { None };
        self.depth += 1;
        let result = self.rule_body(r, args);
        self.depth -= 1;
        if let Some((world, handlers)) = saved
            && result.is_err()
        {
            self.world = world;
            self.handlers = handlers;
        }
        result
    }

    /// [`Self::call_rule`] from outside, with every argument given.
    pub fn step(&mut self, r: RuleId, args: Vec<Value>) -> Halted<Outcome> {
        match self.call_rule(r, args.into_iter().map(Some).collect()) {
            Ok(v) => Ok(Outcome::Ok(v)),
            Err(Halt::Fail(f)) => Ok(Outcome::Refused(f)),
            Err(h) => Err(h),
        }
    }

    fn rule_body(&mut self, r: RuleId, args: Vec<Option<Value>>) -> Halted<Value> {
        let d = self.prog.rule(r);
        let (mut env, vals) = self.complete_args(&d.params, args)?;
        self.check_params(&d.info, &d.params, &vals)?;
        for g in &d.guards {
            match self.eval(&mut env, &g.cond) {
                Ok(Value::Bool(true)) => {}
                Ok(Value::Bool(false)) | Err(Halt::Fail(_)) => return Err(Halt::fail(&g.error)),
                Ok(other) => {
                    return Err(Diag::error("E-TYPE-MISMATCH", g.cond.span, "a guard condition is a Bool")
                        .with_decl(self.prog.qualified(&d.info))
                        .with_expected("Bool", crate::display::show(self.prog, &other))
                        .into());
                }
                Err(h) => return Err(h),
            }
        }
        if let Some(f) = &d.foreign {
            return Err(Halt::foreign(&self.prog.qualified(&d.info), f));
        }
        let v = self.eval(&mut env, &d.body)?;
        self.check_result(&d.info, &d.result, &v)?;
        Ok(v)
    }

    /// Evaluate a closed expression (no locals): a `@given` value, a case input.
    pub fn eval_closed(&mut self, e: &Expr) -> Halted<Value> {
        self.eval(&mut Env::default(), e)
    }

    /// Evaluate an expression with locals: a law body with its parameters.
    pub fn eval_in(&mut self, env: &mut Env, e: &Expr) -> Halted<Value> {
        self.eval(env, e)
    }

    /// Run an operation of a library handler with its pretend: bind the
    /// pretend's state and the arguments, run the arm, keep the new state.
    pub(crate) fn run_effect(&mut self, e: Effect, args: Vec<Value>, span: spec_lang_syntax::Span) -> Halted<Value> {
        let prog = self.prog;
        let h = prog.handler(e.handler);
        let name = prog.effect_name(e);
        let (Some(pretend), Some(op)) = (&h.pretend, prog.effect_op(e)) else {
            return Err(Diag::error(
                "E-UNSUPPORTED",
                span,
                format!("`{name}`: library `{}` has no pretend for handler `{}`", h.library, h.name),
            )
            .into());
        };
        let Some(Some(arm)) = pretend.arms.get(e.op as usize) else {
            return Err(Diag::error("E-UNSUPPORTED", span, format!("`{name}`: its pretend has no arm")).into());
        };
        for (p, a) in op.params.iter().flatten().zip(&args) {
            if let Some(reason) = self.conform(a, &p.ty)? {
                return Err(Diag::error("E-CLAIM", span, format!("argument `{}` of `{name}`: {reason}", p.name)).into());
            }
        }
        let mut env = Env::default();
        let state = self.handlers.get(e.handler.index()).cloned().unwrap_or_default();
        for (p, v) in pretend.state.iter().zip(state) {
            env.push(&p.name, v);
        }
        for (n, v) in arm.params.iter().zip(args) {
            env.push(n, v);
        }
        let v = self.eval(&mut env, &arm.body)?;
        let state: Vec<Value> = pretend.state.iter().filter_map(|p| env.get(&p.name).cloned()).collect();
        if let Some(slot) = self.handlers.get_mut(e.handler.index()) {
            *slot = state;
        }
        Ok(v)
    }

    /// A [`Value::World`] of the current states.
    pub fn world_value(&self) -> Value {
        Value::World(Rc::new(self.world.clone()))
    }
}
