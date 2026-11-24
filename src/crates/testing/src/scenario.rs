//! Scenarios and `@cases`: build the `@given` world, run the steps.

use crate::config::Config;
use spec_lang_diag::{Diag, Severity};
use spec_lang_engine::display::show;
use spec_lang_engine::{Env, Flow, Halt, Interp, Value, World};
use spec_lang_ir::*;
use std::collections::HashMap;
use std::rc::Rc;

/// How one run of a test ended.
#[derive(Clone, Debug)]
pub(crate) enum Run {
    Pass(World),
    Fail(Diag),
    Blocked(Diag),
}

/// Shared state of a test run: end worlds of tests named by `@given`.
pub(crate) struct Ctx<'p> {
    pub(crate) prog: &'p Program,
    pub(crate) cfg: Config,
    end_worlds: HashMap<TestId, Run>,
    running: Vec<TestId>,
}

impl<'p> Ctx<'p> {
    pub(crate) fn new(prog: &'p Program, cfg: Config) -> Self {
        Ctx { prog, cfg, end_worlds: HashMap::new(), running: Vec::new() }
    }

    pub(crate) fn interp(&self) -> Result<Interp<'p>, Diag> {
        Interp::new(self.prog).map_err(|h| halt_diag(self.prog, h, None))
    }

    /// The end of a parameterless test, run once and remembered.
    pub(crate) fn end_of(&mut self, t: TestId) -> Run {
        if let Some(r) = self.end_worlds.get(&t) {
            return r.clone();
        }
        let r = self.run_test(t, Vec::new());
        self.end_worlds.insert(t, r.clone());
        r
    }

    /// A fresh interpreter at a test's `@given` world.
    pub(crate) fn given(&mut self, t: TestId) -> Result<Interp<'p>, Run> {
        let decl = self.prog.qualified(&self.prog.test(t).info);
        let mut it = self.interp().map_err(Run::Fail)?;
        for (i, g) in self.prog.test(t).given.iter().enumerate() {
            let at = format!("given[{i}]");
            match g {
                Given::State { state, value, span } => {
                    let v = it.eval_closed(value).map_err(|h| {
                        Run::Fail(halt_diag(self.prog, h, Some(*span)).in_decl(&decl).with_at(at.clone()))
                    })?;
                    let ty = &self.prog.state(*state).ty;
                    match it.conform(&v, ty) {
                        Ok(None) => it.world.states[state.index()] = v,
                        Ok(Some(reason)) => {
                            return Err(Run::Fail(
                                Diag::error(
                                    "E-CLAIM",
                                    *span,
                                    format!("this `@given` value breaks the state's type: {reason}"),
                                )
                                .with_decl(decl)
                                .with_at(at),
                            ));
                        }
                        Err(h) => return Err(Run::Fail(halt_diag(self.prog, h, Some(*span)).in_decl(&decl))),
                    }
                }
                Given::Test { test, span } => {
                    if self.running.contains(test) || *test == t {
                        return Err(Run::Fail(
                            Diag::error("E-TYPE-MISMATCH", *span, "`@given` tests form a cycle").with_decl(decl),
                        ));
                    }
                    if !self.prog.test(*test).params.is_empty() {
                        return Err(Run::Fail(
                            Diag::error(
                                "E-TYPE-MISMATCH",
                                *span,
                                "`@given` names a test with parameters; it has no one end world",
                            )
                            .with_decl(decl),
                        ));
                    }
                    self.running.push(t);
                    let parent = self.end_of(*test);
                    self.running.pop();
                    let name = self.prog.qualified(&self.prog.test(*test).info);
                    match parent {
                        Run::Pass(w) => it.world = w,
                        Run::Fail(_) | Run::Blocked(_) => {
                            return Err(Run::Blocked(
                                Diag::new(Severity::Failure, "F-BLOCKED", format!("blocked by {name}"))
                                    .with_span(*span)
                                    .with_decl(decl)
                                    .with_at(at),
                            ));
                        }
                    }
                }
            }
        }
        Ok(it)
    }

    /// Run a test with its parameters bound to `args` (in parameter order).
    pub(crate) fn run_test(&mut self, t: TestId, args: Vec<Value>) -> Run {
        let mut it = match self.given(t) {
            Ok(it) => it,
            Err(r) => return r,
        };
        let test = self.prog.test(t);
        let decl = self.prog.qualified(&test.info);
        let mut env = Env::default();
        for (p, v) in test.params.iter().zip(args) {
            env.push(&p.name, v);
        }
        for (i, s) in test.body.stmts.iter().enumerate() {
            match it.exec(&mut env, s) {
                Ok(Flow::Normal) => {}
                Ok(Flow::Return(_)) => break,
                Err(h) => {
                    let at = format!("body.stmt[{i}]");
                    let d = match h {
                        Halt::Fail(f) => Diag::new(
                            Severity::Failure,
                            "F-TEST",
                            format!("this step was refused: {}", show(self.prog, &Value::Failure(f))),
                        )
                        .with_span(s.span)
                        .with_note("write `-> fail Name;` if the refusal is expected"),
                        Halt::Blocked(h, span) => {
                            return Run::Blocked(
                                Diag::new(Severity::Failure, "F-BLOCKED", format!("reached ?{h}"))
                                    .with_span(span)
                                    .with_decl(decl)
                                    .with_at(at),
                            );
                        }
                        other => halt_diag(self.prog, other, Some(s.span)),
                    };
                    return Run::Fail(d.in_decl(&decl).with_at(at));
                }
            }
        }
        Run::Pass(it.world)
    }
}

/// A halt as a diagnostic, with `span` when the halt has none.
pub(crate) fn halt_diag(prog: &Program, h: Halt, span: Option<spec_lang_syntax::Span>) -> Diag {
    let d = match h {
        Halt::Fatal(d) | Halt::Check(d) => *d,
        Halt::Fail(f) => {
            Diag::new(Severity::Failure, "F-TEST", format!("failed with {}", show(prog, &Value::Failure(f))))
        }
        Halt::Blocked(h, s) => Diag::new(Severity::Failure, "F-BLOCKED", format!("reached ?{h}")).with_span(s),
    };
    match (d.span, span) {
        (None, Some(s)) => d.with_span(s),
        _ => d,
    }
}

/// The result of one `@cases` row.
pub(crate) fn run_case(ctx: &mut Ctx<'_>, target: CaseTarget, case: &Case, decl: &str, at: String) -> Result<(), Diag> {
    let prog = ctx.prog;
    let mut it = match target {
        CaseTarget::Test(t) => match ctx.given(t) {
            Ok(it) => it,
            Err(Run::Fail(d) | Run::Blocked(d)) => return Err(d),
            Err(Run::Pass(_)) => return Ok(()),
        },
        CaseTarget::Fn(_) => ctx.interp()?,
    };
    let mut args = Vec::new();
    for a in &case.args {
        args.push(match a {
            Some(e) => Some(
                it.eval_closed(e).map_err(|h| halt_diag(prog, h, Some(case.span)).in_decl(decl).with_at(at.clone()))?,
            ),
            None => None,
        });
    }
    let got = match target {
        CaseTarget::Fn(f) => it.call_fn(f, args),
        CaseTarget::Test(t) => {
            let vals: Vec<Value> = args.into_iter().flatten().collect();
            return match ctx.run_test(t, vals) {
                Run::Pass(_) => Ok(()),
                Run::Fail(d) | Run::Blocked(d) => Err(d.with_at(at)),
            };
        }
    };
    let got = match got {
        Ok(v) => Ok(v),
        Err(Halt::Fail(f)) => Err(f),
        Err(h) => return Err(halt_diag(prog, h, Some(case.span)).in_decl(decl).with_at(at)),
    };
    let want = match &case.expect {
        Expect::Value(e) => Ok(it.eval_closed(e).map_err(|h| halt_diag(prog, h, Some(case.span)))?),
        Expect::Fail(f) => Err(Rc::new(spec_lang_engine::Failure { name: f.name.clone(), data: Vec::new() })),
    };
    let same = match (&got, &want) {
        (Ok(a), Ok(b)) => a == b,
        (Err(a), Err(b)) => a.name == b.name,
        _ => false,
    };
    if same {
        return Ok(());
    }
    let text = |r: &Result<Value, Rc<spec_lang_engine::Failure>>| match r {
        Ok(v) => show(prog, v),
        Err(f) => show(prog, &Value::Failure(f.clone())),
    };
    Err(Diag::new(Severity::Failure, "F-CASE", format!("the case gave {}", text(&got)))
        .with_span(case.span)
        .with_decl(decl)
        .with_at(at)
        .with_expected(text(&want), text(&got)))
}

#[derive(Clone, Copy)]
pub(crate) enum CaseTarget {
    Fn(FnId),
    Test(TestId),
}
