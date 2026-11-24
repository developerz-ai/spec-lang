//! The backend-neutral front of code generation: the IR typed
//! ([`typed`]), the facts that license strategies ([`facts`]), and the
//! `cli` vocabulary's derived parameter table ([`cli`]).
//!
//! Both emitters read a [`Front`]. It belongs in the IR crate as SIR-F
//! (docs/idea/10-compiler-and-ir.md); it lives here until it moves
//! (`IR-NEEDS.md`).

pub mod cli;
pub mod facts;
pub mod typed;
pub mod typer;

use spec_lang_diag::Diag;
use spec_lang_ir::{AppDecl, AppId, FnId, Given, LawId, Program, RuleId, StateId, TestId, TypeDef};
use spec_lang_syntax::Span;

use typed::{T, TExpr, TStmt};
use typer::{R, Typer};

/// A parameter with its type and, typed, its default.
#[derive(Clone, Debug)]
pub struct TParam {
    pub name: String,
    pub ty: T,
    pub default: Option<TExpr>,
    pub span: Span,
}

#[derive(Debug)]
pub struct TFn {
    pub id: FnId,
    pub params: Vec<TParam>,
    pub result: T,
    pub body: Vec<TStmt>,
    /// `@cases` rows: arguments (defaults filled) and the value expected.
    pub cases: Vec<TCase>,
}

#[derive(Debug)]
pub struct TCase {
    pub args: Vec<TExpr>,
    pub expect: TExpr,
    pub span: Span,
}

#[derive(Debug)]
pub struct TGuard {
    pub cond: TExpr,
    pub error: String,
    pub span: Span,
}

#[derive(Debug)]
pub struct TRule {
    pub id: RuleId,
    pub params: Vec<TParam>,
    pub result: T,
    pub guards: Vec<TGuard>,
    pub body: Vec<TStmt>,
}

#[derive(Debug)]
pub enum TGiven {
    State { state: StateId, value: TExpr, span: Span },
    Test { test: TestId, span: Span },
}

#[derive(Debug)]
pub struct TTest {
    pub id: TestId,
    /// Generated from their types: the test is a property.
    pub params: Vec<TParam>,
    pub given: Vec<TGiven>,
    pub body: Vec<TStmt>,
}

#[derive(Debug)]
pub struct TLaw {
    pub id: LawId,
    pub trace: String,
    pub rules: Vec<RuleId>,
    /// Every parameter but the trace: quantified, generated.
    pub others: Vec<TParam>,
    pub body: TExpr,
}

#[derive(Debug)]
pub struct TState {
    pub id: StateId,
    pub ty: T,
    /// `None`: starts empty.
    pub init: Option<TExpr>,
}

/// One app's program, typed, with its facts.
#[derive(Debug)]
pub struct Front<'p> {
    pub prog: &'p Program,
    pub app: AppId,
    pub states: Vec<TState>,
    pub fns: Vec<TFn>,
    /// The app's rules, in the app's order.
    pub rules: Vec<TRule>,
    pub tests: Vec<TTest>,
    pub laws: Vec<TLaw>,
    /// Every error a guard names, in order of first appearance.
    pub errors: Vec<String>,
    pub facts: facts::Facts,
    pub cli: Vec<cli::Command>,
}

impl<'p> Front<'p> {
    pub fn app(&self) -> &'p AppDecl {
        self.prog.app(self.app)
    }

    pub fn rule(&self, id: RuleId) -> Option<&TRule> {
        self.rules.iter().find(|r| r.id == id)
    }

    pub fn func(&self, id: FnId) -> Option<&TFn> {
        self.fns.iter().find(|f| f.id == id)
    }

    pub fn test(&self, id: TestId) -> Option<&TTest> {
        self.tests.iter().find(|t| t.id == id)
    }

    /// `todo.board.done`.
    pub fn decl_path(&self, info: &spec_lang_ir::DeclInfo) -> String {
        self.prog.qualified(info)
    }

    /// `src/todo/board.slg:44:1` as the loader was given the path.
    pub fn at(&self, span: Span) -> String {
        let l = self.prog.location(span);
        format!("{}:{}:{}", l.path, l.line, l.col)
    }
}

/// Types every declaration the app reaches and derives its facts.
pub fn build(prog: &Program, app: AppId) -> Result<Front<'_>, Vec<Diag>> {
    let mut diags = Vec::new();
    let mut keep = |r: R<()>| {
        if let Err(d) = r {
            diags.push(d);
        }
    };
    let mut front = Front {
        prog,
        app,
        states: Vec::new(),
        fns: Vec::new(),
        rules: Vec::new(),
        tests: Vec::new(),
        laws: Vec::new(),
        errors: Vec::new(),
        facts: facts::Facts::default(),
        cli: Vec::new(),
    };
    for (i, s) in prog.states.iter().enumerate() {
        keep(state(&mut front, StateId(i as u32), s));
    }
    for (i, f) in prog.fns.iter().enumerate() {
        keep(func(&mut front, FnId(i as u32), f));
    }
    for rule in &prog.app(app).rules {
        keep(rule_decl(&mut front, *rule));
    }
    for (i, _) in prog.tests.iter().enumerate() {
        keep(test(&mut front, TestId(i as u32)));
    }
    for (i, _) in prog.laws.iter().enumerate() {
        keep(law(&mut front, LawId(i as u32)));
    }
    if !diags.is_empty() {
        return Err(diags);
    }
    for r in &front.rules {
        for g in &r.guards {
            if !front.errors.contains(&g.error) {
                front.errors.push(g.error.clone());
            }
        }
    }
    front.facts = facts::derive(&front).map_err(|d| vec![d])?;
    front.cli = cli::derive(&front).map_err(|d| vec![d])?;
    Ok(front)
}

fn params(t: &mut Typer<'_>, ps: &[spec_lang_ir::Param]) -> R<Vec<TParam>> {
    let mut out = Vec::new();
    for p in ps {
        let ty = t.ty(&p.ty, p.span)?;
        let default = match &p.default {
            Some(d) => Some(t.expr(d, Some(&ty))?),
            None => None,
        };
        t.bind(&p.name, ty.clone());
        out.push(TParam { name: p.name.clone(), ty, default, span: p.span });
    }
    Ok(out)
}

fn state(front: &mut Front<'_>, id: StateId, s: &spec_lang_ir::StateDecl) -> R<()> {
    let mut t = Typer::new(front.prog, front.prog.qualified(&s.info));
    let ty = t.ty(&s.ty, s.info.span)?;
    let init = match &s.init {
        Some(e) => Some(t.expr(e, Some(&ty))?),
        None => None,
    };
    front.states.push(TState { id, ty, init });
    Ok(())
}

fn func(front: &mut Front<'_>, id: FnId, f: &spec_lang_ir::FnDecl) -> R<()> {
    let mut t = Typer::new(front.prog, front.prog.qualified(&f.info));
    let ps = params(&mut t, &f.params)?;
    t.result = match &f.result {
        Some(r) => t.ty(r, f.info.span)?,
        None => typed::T::Unit,
    };
    let body = t.body(&f.body)?;
    let mut cases = Vec::new();
    for c in &f.cases {
        let mut args = Vec::new();
        for (p, a) in ps.iter().zip(&c.args) {
            args.push(match a {
                Some(a) => t.expr(a, Some(&p.ty))?,
                None => {
                    p.default.clone().ok_or_else(|| t.unsupported(c.span, "a case without a value for a parameter"))?
                }
            });
        }
        let expect = match &c.expect {
            spec_lang_ir::Expect::Value(v) => t.expr(v, Some(&t.result.clone()))?,
            spec_lang_ir::Expect::Fail(_) => return Err(t.unsupported(c.span, "a case that expects a failure of a fn")),
        };
        cases.push(TCase { args, expect, span: c.span });
    }
    front.fns.push(TFn { id, params: ps, result: t.result, body, cases });
    Ok(())
}

fn rule_decl(front: &mut Front<'_>, id: RuleId) -> R<()> {
    let r = front.prog.rule(id);
    let mut t = Typer::new(front.prog, front.prog.qualified(&r.info));
    let ps = params(&mut t, &r.params)?;
    t.result = match &r.result {
        Some(x) => t.ty(x, r.info.span)?,
        None => typed::T::Unit,
    };
    let mut guards = Vec::new();
    for g in &r.guards {
        guards.push(TGuard { cond: t.expr(&g.cond, Some(&typed::T::Bool))?, error: g.error.clone(), span: g.span });
    }
    let body = t.body(&r.body)?;
    front.rules.push(TRule { id, params: ps, result: t.result, guards, body });
    Ok(())
}

fn test(front: &mut Front<'_>, id: TestId) -> R<()> {
    let d = front.prog.test(id);
    let mut t = Typer::new(front.prog, front.prog.qualified(&d.info));
    if !d.cases.is_empty() {
        return Err(t.unsupported(d.info.span, "@cases on a test"));
    }
    let ps = params(&mut t, &d.params)?;
    let mut given = Vec::new();
    for g in &d.given {
        given.push(match g {
            Given::State { state, value, span } => {
                let ty = t.ty(&front.prog.state(*state).ty, *span)?;
                TGiven::State { state: *state, value: t.expr(value, Some(&ty))?, span: *span }
            }
            Given::Test { test, span } => TGiven::Test { test: *test, span: *span },
        });
    }
    let body = t.block(&d.body)?;
    front.tests.push(TTest { id, params: ps, given, body });
    Ok(())
}

fn law(front: &mut Front<'_>, id: LawId) -> R<()> {
    let d = front.prog.law(id);
    let mut t = Typer::new(front.prog, front.prog.qualified(&d.info));
    let Some((index, rules)) = d.trace() else {
        return Err(t.unsupported(d.info.span, "a law without a Trace parameter"));
    };
    let mut others = Vec::new();
    let mut trace = String::new();
    for (i, p) in d.params.iter().enumerate() {
        let ty = t.ty(&p.ty, p.span)?;
        t.bind(&p.name, ty.clone());
        if i == index {
            trace = p.name.clone();
        } else {
            others.push(TParam { name: p.name.clone(), ty, default: None, span: p.span });
        }
    }
    let body = t.expr(&d.body, Some(&typed::T::Bool))?;
    front.laws.push(TLaw { id, trace, rules: rules.to_vec(), others, body });
    Ok(())
}

/// The fields of a record type, or `None`.
pub fn record_fields(prog: &Program, id: spec_lang_ir::TypeId) -> Option<&[spec_lang_ir::Field]> {
    match &prog.ty(id).def {
        TypeDef::Record(fields) => Some(fields),
        _ => None,
    }
}
