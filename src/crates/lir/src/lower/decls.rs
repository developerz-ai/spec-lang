//! Types, claims, constants, states, functions and rules.

use super::{Lowerer, R, Typer};
use crate::node::Expr;
use crate::program::*;
use crate::ty::{ClaimId, LTy};
use spec_lang_ir as ir;
use spec_lang_ir::{ConstId, FnId, RuleId, StateId, TypeId};

/// Fields of a record or variant: types, checks and defaults (closed expressions).
fn fields<'p>(low: &mut Lowerer<'p>, decl: &str, fs: &'p [ir::Field]) -> R<Vec<Field>> {
    let mut out = Vec::new();
    for f in fs {
        let ty = low.erase(&f.ty);
        let check = low.dty(&f.ty);
        let default = match &f.default {
            Some(d) => Some(closed(low, decl, d, &ty)?),
            None => None,
        };
        out.push(Field { name: f.name.clone(), ty, check, default });
    }
    Ok(out)
}

/// A closed expression of a known type.
fn closed<'p>(low: &mut Lowerer<'p>, decl: &str, e: &'p ir::Expr, ty: &LTy) -> R<Expr> {
    let mut t = Typer::new(low, decl.to_string());
    let mut x = t.expr_as(e, ty)?;
    t.finish_expr(&mut x);
    Ok(x)
}

pub(super) fn lower_type(low: &mut Lowerer<'_>, id: TypeId) -> R<TypeDecl> {
    let prog = low.prog;
    let d = prog.ty(id);
    let info = low.info(&d.info);
    let builtin = id == prog.option_type || id == prog.result_type;
    let def = match &d.def {
        _ if builtin => {
            let ir::TypeDef::Sum(vs) = &d.def else { return Err(spec_lang_diag::Diag::load("bad built-in type")) };
            TypeDef::Sum { variants: vs.iter().map(|v| Variant { name: v.name.clone(), fields: Vec::new() }).collect() }
        }
        ir::TypeDef::Alias(t) => TypeDef::Alias { of: low.dty(t) },
        ir::TypeDef::Record(fs) => TypeDef::Record { fields: fields(low, &info.qualified, fs)? },
        ir::TypeDef::Sum(vs) => {
            let mut variants = Vec::new();
            for v in vs {
                variants.push(Variant { name: v.name.clone(), fields: fields(low, &info.qualified, &v.fields)? });
            }
            TypeDef::Sum { variants }
        }
        ir::TypeDef::Rules(rs) => TypeDef::RuleSet { rules: rs.clone() },
    };
    Ok(TypeDecl { info, builtin, def })
}

pub(super) fn lower_claim(low: &mut Lowerer<'_>, id: ClaimId) -> R<Claim> {
    let (src, self_ty) = {
        let s = &low.claims[id.index()];
        (s.src, s.self_ty.clone())
    };
    let prog = low.prog;
    let text = prog.snippet(src.expr.span).to_string();
    let decl = format!("{}: claim `{text}`", prog.module(src.module).name);
    let mut t = Typer::new(low, decl);
    t.bind("self", self_ty.clone());
    let mut body = t.expr_as(&src.expr, &LTy::Bool)?;
    t.finish_expr(&mut body);
    Ok(Claim { module: src.module, self_ty, body, text })
}

pub(super) fn lower_const(low: &mut Lowerer<'_>, id: ConstId) -> R<Const> {
    let prog = low.prog;
    let d = prog.constant(id);
    let info = low.info(&d.info);
    let declared = d.ty.as_ref().map(|t| low.dty(t));
    let mut t = Typer::new(low, info.qualified.clone());
    let mut value = t.expr(&d.value)?;
    if let Some(ty) = &d.ty {
        let want = t.low.erase(ty);
        t.unify(&value.ty, &want, d.value.span)?;
    }
    t.finish_expr(&mut value);
    Ok(Const { info, ty: value.ty.clone(), declared, value })
}

pub(super) fn lower_state(low: &mut Lowerer<'_>, id: StateId) -> R<State> {
    let prog = low.prog;
    let d = prog.state(id);
    let info = low.info(&d.info);
    let ty = low.erase(&d.ty);
    let check = low.dty(&d.ty);
    let init = match &d.init {
        Some(e) => Some(closed(low, &info.qualified, e, &ty)?),
        None => None,
    };
    Ok(State { info, ty, check, init })
}

/// Parameters: typed, checked, defaults with the earlier parameters in scope.
pub(super) fn params<'p>(t: &mut Typer<'_, 'p>, ps: &'p [ir::Param]) -> R<Vec<Param>> {
    let mut out = Vec::new();
    for p in ps {
        let ty = t.low.erase(&p.ty);
        let check = t.low.dty(&p.ty);
        let default = match &p.default {
            Some(d) => {
                let mut x = t.expr_as(d, &ty)?;
                t.finish_expr(&mut x);
                Some(x)
            }
            None => None,
        };
        t.bind(&p.name, ty.clone());
        out.push(Param { name: p.name.clone(), ty, check, default });
    }
    Ok(out)
}

fn extern_of(f: &ir::Foreign, uses: &[String]) -> Extern {
    let (library, symbol) = match f.symbol.split_once('#') {
        Some((l, s)) => (l.to_string(), s.to_string()),
        None => (String::new(), f.symbol.clone()),
    };
    Extern { lang: f.lang.clone(), library, symbol, effects: uses.to_vec() }
}

/// A body and its result: declared, or found from the body.
fn body<'p>(
    t: &mut Typer<'_, 'p>,
    body: &'p ir::Expr,
    result: &'p Option<ir::Ty>,
    foreign: &Option<ir::Foreign>,
    uses: &[String],
) -> R<(Body, LTy, Option<crate::ty::DTy>)> {
    let declared = result.as_ref().map(|r| (t.low.erase(r), t.low.dty(r)));
    if let Some(f) = foreign {
        let ty = declared.as_ref().map_or(LTy::Unit, |(l, _)| l.clone());
        return Ok((Body::Extern(extern_of(f, uses)), ty, declared.map(|(_, d)| d)));
    }
    let mut x = t.expr(body)?;
    if let Some((l, _)) = &declared {
        t.unify(&x.ty, l, body.span)?;
    }
    t.finish_expr(&mut x);
    let ty = match &declared {
        Some((l, _)) => l.clone(),
        None if x.ty == LTy::Never => LTy::Unit,
        None => x.ty.clone(),
    };
    Ok((Body::Expr(x), ty, declared.map(|(_, d)| d)))
}

pub(super) fn lower_fn(low: &mut Lowerer<'_>, id: FnId) -> R<Func> {
    let prog = low.prog;
    let d = prog.func(id);
    let info = low.info(&d.info);
    let mut t = Typer::new(low, info.qualified.clone());
    let params = params(&mut t, &d.params)?;
    let (body, result, result_check) = body(&mut t, &d.body, &d.result, &d.foreign, &d.uses)?;
    Ok(Func { info, params, result, result_check, body, uses: d.uses.clone() })
}

pub(super) fn lower_rule(low: &mut Lowerer<'_>, id: RuleId) -> R<Rule> {
    let prog = low.prog;
    let d = prog.rule(id);
    let info = low.info(&d.info);
    let mut t = Typer::new(low, info.qualified.clone());
    let params = params(&mut t, &d.params)?;
    let mut guards = Vec::new();
    for g in &d.guards {
        let mut cond = t.expr_as(&g.cond, &LTy::Bool)?;
        t.finish_expr(&mut cond);
        guards.push(Guard { cond, error: g.error.clone() });
    }
    let (body, result, result_check) = body(&mut t, &d.body, &d.result, &d.foreign, &d.uses)?;
    let triggers = d.triggers.iter().map(crate::manifest_of::note).collect();
    Ok(Rule { info, params, result, result_check, guards, body, uses: d.uses.clone(), triggers })
}
