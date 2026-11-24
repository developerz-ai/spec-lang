//! Passes 5 and 6: signatures, then bodies and notes, per declaration kind.

use super::Lowerer;
use super::declare::flatten_bar;
use super::expr::{Body, Cx, R};
use super::notes::note_args;
use super::scope::Binding;
use crate::error::IrError;
use crate::expr::*;
use crate::ids::*;
use crate::program::*;
use crate::ty::Ty;
use spec_lang_syntax::ast::{self, ExprKind as AK, Tail};

impl Lowerer {
    fn qualified_name(&self, d: DeclRef) -> String {
        self.prog.qualified(self.decl_info(d))
    }

    /// Pass 5: types of fields, parameters, states and results, and defaults.
    pub(super) fn lower_signature(&mut self, d: DeclRef, decl: &ast::Decl) {
        let r = self.signature(d, decl);
        if let Err(e) = r {
            let name = self.qualified_name(d);
            self.err(e.in_decl(&name));
        }
    }

    fn signature(&mut self, d: DeclRef, decl: &ast::Decl) -> R<()> {
        let m = self.decl_module(d);
        match d {
            DeclRef::Type(t) => self.type_signature(t, m, decl),
            DeclRef::State(s) => {
                let ty = match &decl.result {
                    Some(r) if r.sep == ast::ResultSep::Colon => self.lower_ty(m, &r.ty)?,
                    _ => {
                        return Err(IrError::error(
                            "E-TYPE-MISMATCH",
                            decl.span,
                            "a state needs a type: `state x: T;`",
                        ));
                    }
                };
                self.prog.states[s.index()].ty = ty;
                Ok(())
            }
            DeclRef::Const(c) => {
                if let Some(r) = &decl.result {
                    self.prog.consts[c.index()].ty = Some(self.lower_ty(m, &r.ty)?);
                }
                Ok(())
            }
            DeclRef::Fn(f) => {
                let params = self.lower_params(m, decl)?;
                let result = self.lower_result(m, decl)?;
                let x = &mut self.prog.fns[f.index()];
                x.params = params;
                x.result = result;
                Ok(())
            }
            DeclRef::Rule(r) => {
                let params = self.lower_params(m, decl)?;
                let result = self.lower_result(m, decl)?;
                let x = &mut self.prog.rules[r.index()];
                x.params = params;
                x.result = result;
                Ok(())
            }
            DeclRef::Test(t) => {
                self.prog.tests[t.index()].params = self.lower_params(m, decl)?;
                Ok(())
            }
            DeclRef::Law(l) => {
                let params = self.lower_params(m, decl)?;
                let traces = params.iter().filter(|p| matches!(p.ty, Ty::Trace(_))).count();
                if traces != 1 {
                    return Err(IrError::error(
                        "E-TYPE-MISMATCH",
                        decl.name.span,
                        "a law takes exactly one `Trace[...]` parameter",
                    )
                    .with_expected("1 trace parameter", traces.to_string()));
                }
                self.prog.laws[l.index()].params = params;
                Ok(())
            }
            DeclRef::App(a) => self.app_signature(a, m, decl),
        }
    }

    fn type_signature(&mut self, t: TypeId, m: ModuleId, decl: &ast::Decl) -> R<()> {
        let def = std::mem::replace(&mut self.prog.types[t.index()].def, TypeDef::Alias(Ty::Unit));
        let def = match def {
            TypeDef::Record(_) => TypeDef::Record(self.lower_fields(m, decl.params.as_deref().unwrap_or(&[]))?),
            TypeDef::Sum(mut variants) => {
                let body = body_expr(decl);
                let mut alts = Vec::new();
                if let Some(b) = body {
                    flatten_bar(b, &mut alts);
                }
                for (v, alt) in variants.iter_mut().zip(alts) {
                    if let AK::Call(_, args) = &alt.kind {
                        let mut fields = Vec::new();
                        for a in args {
                            let Some(n) = &a.name else {
                                return Err(IrError::error(
                                    "E-TYPE-MISMATCH",
                                    a.span,
                                    "a variant field is `name: Type`",
                                ));
                            };
                            fields.push(Field {
                                name: n.name.clone(),
                                ty: self.lower_ty(m, &a.value)?,
                                default: None,
                                span: a.span,
                            });
                        }
                        v.fields = fields;
                    }
                }
                TypeDef::Sum(variants)
            }
            TypeDef::Rules(_) => match body_expr(decl) {
                Some(b) => TypeDef::Rules(self.rule_set(m, b)?),
                None => TypeDef::Rules(Vec::new()),
            },
            TypeDef::Alias(_) => match body_expr(decl) {
                Some(b) => TypeDef::Alias(self.lower_ty(m, b)?),
                None => return Err(IrError::unsupported(decl.span, "this form of `type`")),
            },
        };
        self.prog.types[t.index()].def = def;
        Ok(())
    }

    fn lower_fields(&mut self, m: ModuleId, params: &[ast::Param]) -> R<Vec<Field>> {
        let mut out = Vec::new();
        for p in params {
            let Some(ty) = &p.ty else {
                return Err(IrError::error("E-TYPE-MISMATCH", p.span, format!("field `{}` needs a type", p.name.name)));
            };
            let ty = self.lower_ty(m, ty)?;
            let default = match &p.default {
                Some(d) => Some(self.lower_expr(&mut Cx::new(m, Body::Value), d)?),
                None => None,
            };
            out.push(Field { name: p.name.name.clone(), ty, default, span: p.span });
        }
        Ok(out)
    }

    fn lower_params(&mut self, m: ModuleId, decl: &ast::Decl) -> R<Vec<Param>> {
        let mut out = Vec::new();
        let mut cx = Cx::new(m, Body::Value);
        for p in decl.params.iter().flatten() {
            if let Some(n) = p.notes.first() {
                return Err(IrError::unsupported(n.span, "notes on parameters"));
            }
            let Some(ty) = &p.ty else {
                return Err(IrError::error(
                    "E-TYPE-MISMATCH",
                    p.span,
                    format!("parameter `{}` needs a type", p.name.name),
                ));
            };
            let ty = self.lower_ty(m, ty)?;
            let default = match &p.default {
                Some(d) => Some(self.lower_expr(&mut cx, d)?),
                None => None,
            };
            self.bind_local(&mut cx, &p.name.name, false, p.name.span)?;
            out.push(Param { name: p.name.name.clone(), ty, default, span: p.span });
        }
        Ok(out)
    }

    fn lower_result(&mut self, m: ModuleId, decl: &ast::Decl) -> R<Option<Ty>> {
        match &decl.result {
            None => Ok(None),
            Some(r) if r.sep == ast::ResultSep::Arrow => Ok(Some(self.lower_ty(m, &r.ty)?)),
            Some(r) => {
                Err(IrError::error("E-TYPE-MISMATCH", r.ty.span, "a result is written `-> T`").with_expected("->", ":"))
            }
        }
    }

    fn app_signature(&mut self, a: AppId, m: ModuleId, decl: &ast::Decl) -> R<()> {
        let cx = Cx::new(m, Body::Value);
        let mut rules = Vec::new();
        for p in decl.params.iter().flatten() {
            if p.name.name != "rules" {
                return Err(IrError::unsupported(p.span, format!("the app parameter `{}`", p.name.name)));
            }
            let Some(AK::List(items)) = p.ty.as_ref().map(|t| &t.kind) else {
                return Err(IrError::error(
                    "E-TYPE-MISMATCH",
                    p.span,
                    "`rules:` takes a list of rules: `rules: [a.b, c]`",
                ));
            };
            for it in items {
                match self.binding_of(&cx, &it.value) {
                    Some(Ok(Binding::Decl(DeclRef::Rule(r)))) => rules.push(r),
                    Some(Err(d)) => return Err(d),
                    _ => return Err(IrError::error("E-NAME-KIND", it.span, "not a rule")),
                }
            }
        }
        self.prog.apps[a.index()].rules = rules;
        Ok(())
    }

    /// Pass 6: notes and bodies.
    pub(super) fn lower_decl_body(&mut self, d: DeclRef, decl: &ast::Decl) {
        let r = self.decl_body(d, decl);
        if let Err(e) = r {
            let name = self.qualified_name(d);
            self.err(e.in_decl(&name));
        }
    }

    fn decl_body(&mut self, d: DeclRef, decl: &ast::Decl) -> R<()> {
        let m = self.decl_module(d);
        self.check_notes(m, &decl.kind.dotted(), &decl.notes)?;
        match d {
            DeclRef::Type(_) => Ok(()),
            DeclRef::State(s) => {
                if let Some(v) = body_value(decl) {
                    self.prog.states[s.index()].init = Some(self.lower_value(&mut Cx::new(m, Body::Value), v)?);
                }
                Ok(())
            }
            DeclRef::Const(c) => {
                let v = body_value(decl)
                    .ok_or_else(|| IrError::error("E-TYPE-MISMATCH", decl.span, "a const needs a value"))?;
                self.prog.consts[c.index()].value = self.lower_value(&mut Cx::new(m, Body::Value), v)?;
                Ok(())
            }
            DeclRef::Fn(f) => {
                let mut cx = self.params_cx(m, Body::Fn, &decl.params);
                let v = body_value(decl).ok_or_else(|| {
                    IrError::unsupported(decl.span, "a `fn` without a define body (declared, machine or foreign)")
                })?;
                let body = self.lower_value(&mut cx, v)?;
                let sig = self.sigs.get(&Callee::Fn(f)).cloned().unwrap_or_default();
                let cases = self.lower_cases(m, &sig, decl)?;
                let x = &mut self.prog.fns[f.index()];
                x.body = body;
                x.cases = cases;
                Ok(())
            }
            DeclRef::Rule(r) => self.rule_body(r, m, decl),
            DeclRef::Test(t) => self.test_body(t, m, decl),
            DeclRef::Law(l) => {
                let mut cx = self.params_cx(m, Body::Law, &decl.params);
                let v = body_value(decl)
                    .ok_or_else(|| IrError::error("E-TYPE-MISMATCH", decl.span, "a law needs a body"))?;
                self.prog.laws[l.index()].body = self.lower_value(&mut cx, v)?;
                Ok(())
            }
            DeclRef::App(a) => self.app_notes(a, decl),
        }
    }

    fn params_cx(&self, m: ModuleId, body: Body, params: &Option<Vec<ast::Param>>) -> Cx {
        let mut cx = Cx::new(m, body);
        for p in params.iter().flatten() {
            self.push_local(&mut cx, &p.name.name, false);
        }
        cx
    }

    fn rule_body(&mut self, r: RuleId, m: ModuleId, decl: &ast::Decl) -> R<()> {
        let mut cx = self.params_cx(m, Body::Rule, &decl.params);
        let mut guards = Vec::new();
        for (_, args) in note_args(&decl.notes, "guard") {
            for a in args {
                let at = format!("guard[{}]", guards.len());
                let AK::Else(cond, err) = &a.value.kind else {
                    return Err(IrError::error("E-GUARD-FORM", a.span, "a guard clause is `condition else Error`")
                        .with_at(at)
                        .with_edit(a.span, format!("{} else Refused", self.prog.snippet(a.span))));
                };
                let Some(error) = err.as_name() else {
                    return Err(
                        IrError::error("E-GUARD-FORM", err.span, "the error of a guard clause is a name").with_at(at)
                    );
                };
                let cond = self.lower_expr(&mut cx, cond).map_err(|e| e.with_at(at))?;
                guards.push(Guard { cond, error: error.to_string(), span: a.span });
            }
        }
        let mut uses = Vec::new();
        for (_, args) in note_args(&decl.notes, "uses") {
            uses.extend(args.iter().map(|a| self.prog.snippet(a.value.span).to_string()));
        }
        let mut triggers = Vec::new();
        for (n, args) in note_args(&decl.notes, "on") {
            match args {
                [a] => match &a.value.kind {
                    AK::Call(c, cargs)
                        if c.as_name() == Some(crate::vocab::CLI)
                            && self.prog.module(m).vocabularies.iter().any(|v| v == crate::vocab::CLI) =>
                    {
                        match cargs.as_slice() {
                            [ast::Arg { name: None, value: ast::Expr { kind: AK::Text(t), .. }, .. }]
                                if t.prefix.is_none() =>
                            {
                                triggers.push(Trigger::Cli { command: t.raw.clone() });
                            }
                            _ => {
                                return Err(IrError::error(
                                    "E-NOTE-ARG",
                                    a.span,
                                    "`cli(...)` takes the command name as text",
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(IrError::unsupported(
                            a.span,
                            format!("the trigger `{}`", self.prog.snippet(a.span)),
                        ));
                    }
                },
                _ => return Err(IrError::error("E-NOTE-ARG", n.span, "`@on` takes one trigger")),
            }
        }
        let v = body_value(decl).ok_or_else(|| IrError::unsupported(decl.span, "a `rule` without a define body"))?;
        let body = self.lower_value(&mut cx, v).map_err(|e| if e.at.is_none() { e } else { prefix_at(e, "body") })?;
        let x = &mut self.prog.rules[r.index()];
        x.guards = guards;
        x.uses = uses;
        x.triggers = triggers;
        x.body = body;
        Ok(())
    }

    fn test_body(&mut self, t: TestId, m: ModuleId, decl: &ast::Decl) -> R<()> {
        let mut given = Vec::new();
        for (_, args) in note_args(&decl.notes, "given") {
            for (i, a) in args.iter().enumerate() {
                given.push(self.lower_given(m, a).map_err(|e| e.with_at(format!("given[{i}]")))?);
            }
        }
        let sig: Vec<(String, bool)> =
            decl.params.iter().flatten().map(|p| (p.name.name.clone(), p.default.is_some())).collect();
        let cases = self.lower_cases(m, &sig, decl)?;
        let mut cx = self.params_cx(m, Body::Test, &decl.params);
        let body = match &decl.tail {
            Tail::Body(ast::Body::Value(ast::Value::Block(b))) => {
                self.lower_block(&mut cx, b).map_err(|e| prefix_at(e, "body"))?
            }
            _ => {
                return Err(IrError::error("E-TYPE-MISMATCH", decl.span, "a test body is a block: `test t = { ... }`"));
            }
        };
        let x = &mut self.prog.tests[t.index()];
        x.given = given;
        x.cases = cases;
        x.body = body;
        Ok(())
    }

    fn lower_given(&mut self, m: ModuleId, a: &ast::Arg) -> R<Given> {
        let cx = Cx::new(m, Body::Value);
        match &a.value.kind {
            AK::Arrow(place, value) => match self.binding_of(&cx, place) {
                Some(Ok(Binding::Decl(DeclRef::State(state)))) => {
                    let owner = self.prog.state(state).info.module;
                    if owner != m {
                        let owner = self.prog.module(owner).name.clone();
                        return Err(IrError::error(
                            "E-STATE-FOREIGN",
                            place.span,
                            format!("`@given` sets only its own module's state; this one belongs to {owner}"),
                        )
                        .with_note(format!(
                            "name a test of {owner} that builds this world: `@given({}.<test>)`",
                            owner.rsplit('.').next().unwrap_or("")
                        )));
                    }
                    let value = self.lower_expr(&mut Cx::new(m, Body::Value), value)?;
                    Ok(Given::State { state, value, span: a.span })
                }
                Some(Err(d)) => Err(d),
                _ => Err(IrError::unsupported(a.span, format!("the `@given` item `{}`", self.prog.snippet(a.span)))),
            },
            _ => match self.binding_of(&cx, &a.value) {
                Some(Ok(Binding::Decl(DeclRef::Test(test)))) => Ok(Given::Test { test, span: a.span }),
                Some(Err(d)) => Err(d),
                _ => Err(IrError::unsupported(
                    a.span,
                    format!("the `@given` item `{}` (handlers are not implemented)", self.prog.snippet(a.span)),
                )),
            },
        }
    }

    /// `@cases(input -> expected, ...)`: inputs bound like a call's arguments.
    fn lower_cases(&mut self, m: ModuleId, sig: &[(String, bool)], decl: &ast::Decl) -> R<Vec<Case>> {
        let n_params = sig.len();
        let mut out = Vec::new();
        for (_, args) in note_args(&decl.notes, "cases") {
            for a in args {
                let at = format!("case[{}]", out.len());
                let AK::Arrow(input, expected) = &a.value.kind else {
                    return Err(IrError::error("E-NOTE-ARG", a.span, "a case is `arguments -> expected`").with_at(at));
                };
                let mut cx = Cx::new(m, Body::Value);
                let one = ast::Arg { name: None, value: (**input).clone(), span: input.span };
                let inputs: Vec<ast::Arg> = match &input.kind {
                    AK::Group { args, .. } if n_params != 1 || args.iter().any(|x| x.name.is_some()) => args.clone(),
                    _ => vec![one],
                };
                let bound = self.bind_with(&mut cx, sig, &inputs, input.span).map_err(|e| e.with_at(at.clone()))?;
                let expect = self.lower_expect(&mut cx, expected).map_err(|e| e.with_at(at))?;
                out.push(Case { args: bound, expect, span: a.span });
            }
        }
        Ok(out)
    }

    fn app_notes(&mut self, a: AppId, decl: &ast::Decl) -> R<()> {
        let text_arg = |args: &[ast::Arg]| match args {
            [ast::Arg { name: None, value: ast::Expr { kind: AK::Text(t), .. }, .. }] if t.prefix.is_none() => {
                Some(t.raw.clone())
            }
            _ => None,
        };
        for n in &decl.notes {
            let args = n.args.as_deref().unwrap_or(&[]);
            match n.path.dotted().as_str() {
                "target" => match args {
                    [x] if x.value.as_name().is_some() => {
                        self.prog.apps[a.index()].target = x.value.as_name().map(str::to_string);
                    }
                    _ => return Err(IrError::error("E-NOTE-ARG", n.span, "`@target(name)` takes one backend name")),
                },
                "bin" => {
                    let v = text_arg(args)
                        .ok_or_else(|| IrError::error("E-NOTE-ARG", n.span, "`@bin(\"name\")` takes text"))?;
                    self.prog.apps[a.index()].bin = Some(v);
                }
                "store" => {
                    let file = match args {
                        [x] => match &x.value.kind {
                            AK::Call(c, cargs) if c.as_name() == Some("file") => text_arg(cargs),
                            _ => None,
                        },
                        _ => None,
                    };
                    let f = file
                        .ok_or_else(|| IrError::error("E-NOTE-ARG", n.span, "`@store(file(\"path\"))` takes a file"))?;
                    self.prog.apps[a.index()].store = Some(Store::File(f));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn body_value(decl: &ast::Decl) -> Option<&ast::Value> {
    match &decl.tail {
        Tail::Body(ast::Body::Value(v)) => Some(v),
        _ => None,
    }
}

fn body_expr(decl: &ast::Decl) -> Option<&ast::Expr> {
    match body_value(decl) {
        Some(ast::Value::Expr(e)) => Some(e),
        _ => None,
    }
}

fn prefix_at(mut e: IrError, prefix: &str) -> IrError {
    e.at = Some(match e.at.take() {
        Some(a) => format!("{prefix}.{a}"),
        None => prefix.to_string(),
    });
    e
}
