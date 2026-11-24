//! One module's file: its types, claims, constants, fns, rules, tests and laws, each with its origin.

use crate::code::Fx;
use crate::cx::{Cx, Mode, RECORD};
use crate::names::{camel, ident, local, screaming};
use spec_lang_ir::ModuleId;
use spec_lang_lir::node::{Expr, ExprKind};
use spec_lang_lir::{Body, DTy, Field, Info, LTy, Param, TypeDef};
use std::collections::BTreeSet;

/// Whether constructing needs a checked constructor: a field with a default or a check that can fail.
pub fn needs_new(cx: &Cx<'_>, fields: &[Field]) -> bool {
    fields.iter().any(|f| f.default.is_some() || !cx.trivial(&f.check))
}

pub fn new_m(cx: &Cx<'_>, fields: &[Field]) -> bool {
    fields.iter().any(|f| cx.dty_m(&f.check) || f.default.as_ref().is_some_and(|d| cx.expr_m(d)))
}

/// The constructor of a variant: `new_circle`.
pub fn ctor_name(variant: &str) -> String {
    ident(&format!("new_{}", crate::dty::snake(variant.trim_start_matches("r#"))))
}

fn doc(cx: &Cx<'_>, info: &Info, what: &str) -> String {
    let mut lines = Vec::new();
    if let Some(w) = &info.why {
        lines.push(format!("/// {w}"));
        lines.push("///".into());
    }
    lines.push(format!("/// {what} `{}`, `{}`.", info.qualified, cx.origin(info)));
    lines.join("\n")
}

/// Every local name a body may declare or read, as Rust names.
fn locals(params: &[Param], exprs: &[&Expr]) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = params.iter().map(|p| local(&p.name)).collect();
    for e in exprs {
        let mut names = BTreeSet::new();
        crate::stmt::locals_in_expr(e, &mut names);
        out.extend(names.iter().map(|n| local(n)));
    }
    out
}

fn sig(cx: &Cx<'_>, params: &[Param], module: Option<ModuleId>) -> Vec<String> {
    params
        .iter()
        .map(|p| {
            let t = cx.ty(&p.ty, module);
            if p.default.is_some() { format!("{}: Option<{t}>", local(&p.name)) } else { format!("{}: {t}", local(&p.name)) }
        })
        .collect()
}

/// `let n = n.unwrap_or_default();` for a default that is the type's zero, else the default evaluated when absent.
fn default_line(fx: &mut Fx<'_, '_>, n: &str, d: &Expr) -> String {
    let zero = match &d.kind {
        ExprKind::Bool(false) | ExprKind::Int(0) | ExprKind::None => true,
        ExprKind::Text(t) => t.is_empty(),
        ExprKind::List(xs) | ExprKind::SetOf(xs) => xs.is_empty(),
        ExprKind::MapOf(xs) => xs.is_empty(),
        _ => false,
    };
    if zero {
        return format!("let {n} = {n}.unwrap_or_default();");
    }
    let v = fx.value(d);
    format!("let {n} = match {n} {{ Some(v) => v, None => {} }};", v.seal())
}

/// Defaults filled in order (earlier parameters in scope), then every parameter checked.
fn prologue(fx: &mut Fx<'_, '_>, params: &[Param], decl: &str) -> Vec<String> {
    let mut out = Vec::new();
    for p in params {
        let n = local(&p.name);
        if let Some(d) = &p.default {
            out.push(default_line(fx, &n, d));
        }
        if fx.assigned.contains(&p.name) {
            out.push(format!("let mut {n} = {n};"));
        }
    }
    for p in params {
        if let Some(line) = fx.check(&p.check, &local(&p.name), &format!("argument `{}` of {decl}", p.name)) {
            out.push(line);
        }
    }
    out
}

/// The body, then the result checked.
fn body(fx: &mut Fx<'_, '_>, info: &Info, params: &[Param], body: &Body, result: &LTy, check: Option<&DTy>) -> Vec<String> {
    let mut out = Vec::new();
    let e = match body {
        Body::Extern(x) => {
            let sep = if x.library.is_empty() { "" } else { "#" };
            let written = format!("foreign {} \"{}{sep}{}\"", x.lang, x.library, x.symbol);
            if fx.cx.mode == Mode::App {
                let names: Vec<String> = params.iter().map(|p| local(&p.name)).collect();
                out.extend(crate::app::extern_call(fx, x, result, &names));
            } else {
                out.push(format!("Err(rt::foreign({:?}, {written:?}))", info.qualified));
            }
            return out;
        }
        Body::Expr(e) => e,
    };
    let checked = check.filter(|d| !fx.cx.trivial(d));
    match (checked, &e.kind) {
        (Some(d), _) => {
            let v = match &e.kind {
                ExprKind::Block(b) => fx.block_value(b),
                _ => fx.value(e),
            };
            out.extend(v.pre.iter().cloned());
            out.push(format!("let result: {} = {};", fx.ty(result), v.text));
            if let Some(line) = fx.check(d, "result", &format!("the result of {}", info.qualified)) {
                out.push(line);
            }
            out.push("Ok(result)".into());
        }
        (None, ExprKind::Block(b)) => out.extend(fx.fn_body(b, result)),
        (None, _) => {
            let v = fx.value(e);
            out.extend(v.pre.iter().cloned());
            out.push(crate::code::ok(&v.text));
        }
    }
    out
}

fn fx_for<'c, 'a>(cx: &'c mut Cx<'a>, module: ModuleId, has_m: bool, params: &[Param], exprs: &[&Expr]) -> Fx<'c, 'a> {
    let mut fx = Fx::new(cx, Some(module), has_m);
    fx.locals = locals(params, exprs);
    let mut assigned = BTreeSet::new();
    for e in exprs {
        crate::stmt::assigned_in_expr(e, &mut assigned);
        if let ExprKind::Block(b) = &e.kind {
            crate::stmt::assigned(&b.stmts, &mut assigned);
        }
    }
    fx.assigned = assigned;
    fx
}

fn derive() -> &'static str {
    "#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]"
}

fn type_items(cx: &mut Cx<'_>, module: ModuleId) -> Vec<String> {
    let lir = cx.lir;
    let mut out = Vec::new();
    let from = Some(module);
    for (i, decl) in lir.types.iter().enumerate() {
        if decl.builtin || decl.info.module != module {
            continue;
        }
        let id = spec_lang_ir::TypeId(i as u32);
        let name = ident(&decl.info.name);
        match &decl.def {
            TypeDef::Alias { of } => {
                out.push(format!(
                    "{}\n/// Checked against `{}` wherever a value of it is given.\npub type {name} = {};",
                    doc(cx, &decl.info, "Type"),
                    crate::dty::describe(cx, of),
                    cx.ty(&cx.lty(of), from)
                ));
            }
            TypeDef::RuleSet { rules } => {
                let names: Vec<String> = rules.iter().map(|r| format!("`{}`", lir.rule(*r).info.qualified)).collect();
                out.push(format!(
                    "{}\n/// The calls of {}: a value of it is a `crate::machine::Event` of one of them.\npub type {name} = crate::machine::Event;",
                    doc(cx, &decl.info, "Rule set"),
                    names.join(", ")
                ));
            }
            TypeDef::Record { fields } => {
                let fs: Vec<String> = fields
                    .iter()
                    .enumerate()
                    .map(|(fi, f)| {
                        let t = cx.ty(&f.ty, from);
                        let t = if cx.is_boxed(id, RECORD, fi as u32) { format!("Box<{t}>") } else { t };
                        format!("pub {}: {t},", ident(&f.name))
                    })
                    .collect();
                out.push(format!("{}\n{}\npub struct {name} {{ {} }}", doc(cx, &decl.info, "Record"), derive(), fs.join(" ")));
                let shown: Vec<String> = fields.iter().map(|f| format!("({:?}, self.{}.show())", f.name, ident(&f.name))).collect();
                out.push(format!(
                    "impl Show for {name} {{ fn show(&self) -> String {{ rt::show::record({:?}, &[{}]) }} }}",
                    decl.info.name,
                    shown.join(", ")
                ));
                if needs_new(cx, fields) {
                    let ctor = constructor(cx, module, id, RECORD, "new", &name, fields);
                    out.push(format!("impl {name} {{ {ctor} }}"));
                }
            }
            TypeDef::Sum { variants } => {
                let mut vs = Vec::new();
                let mut shows = Vec::new();
                let mut ctors = Vec::new();
                for (vi, v) in variants.iter().enumerate() {
                    let vn = ident(&camel(&v.name));
                    if v.fields.is_empty() {
                        vs.push(format!("{vn},"));
                        shows.push(format!("{name}::{vn} => {:?}.to_string(),", v.name));
                        continue;
                    }
                    let fs: Vec<String> = v
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(fi, f)| {
                            let t = cx.ty(&f.ty, from);
                            let t = if cx.is_boxed(id, vi as u32, fi as u32) { format!("Box<{t}>") } else { t };
                            format!("{}: {t},", ident(&f.name))
                        })
                        .collect();
                    vs.push(format!("{vn} {{ {} }},", fs.join(" ")));
                    let names: Vec<String> = v.fields.iter().map(|f| ident(&f.name)).collect();
                    let shown: Vec<String> = v.fields.iter().map(|f| format!("({:?}, {}.show())", f.name, ident(&f.name))).collect();
                    shows.push(format!(
                        "{name}::{vn} {{ {} }} => rt::show::record({:?}, &[{}]),",
                        names.join(", "),
                        v.name,
                        shown.join(", ")
                    ));
                    if needs_new(cx, &v.fields) {
                        let path = format!("{name}::{vn}");
                        ctors.push(constructor(cx, module, id, vi as u32, &ctor_name(&vn), &path, &v.fields));
                    }
                }
                out.push(format!("{}\n{}\npub enum {name} {{ {} }}", doc(cx, &decl.info, "Sum"), derive(), vs.join(" ")));
                out.push(format!("impl Show for {name} {{ fn show(&self) -> String {{ match self {{ {} }} }} }}", shows.join(" ")));
                if !ctors.is_empty() {
                    out.push(format!("impl {name} {{ {} }}", ctors.join("\n\n")));
                }
            }
        }
    }
    out
}

/// A checked constructor: given fields in order, a left-out one (`None`) takes its default; each checked.
fn constructor(cx: &mut Cx<'_>, module: ModuleId, id: spec_lang_ir::TypeId, variant: u32, fname: &str, path: &str, fields: &[Field]) -> String {
    let m = new_m(cx, fields);
    let mut params: Vec<String> = fields
        .iter()
        .map(|f| {
            let t = cx.ty(&f.ty, Some(module));
            if f.default.is_some() { format!("{}: Option<{t}>", ident(&f.name)) } else { format!("{}: {t}", ident(&f.name)) }
        })
        .collect();
    if m {
        params.insert(0, "m: &mut Machine".into());
    }
    let mut fx = Fx::new(cx, Some(module), m);
    let mut lines = Vec::new();
    for f in fields {
        let n = ident(&f.name);
        if let Some(d) = &f.default {
            lines.push(default_line(&mut fx, &n, d));
        }
        if let Some(line) = fx.check(&f.check, &n, &format!("field `{}`", f.name)) {
            lines.push(line);
        }
    }
    let inits: Vec<String> = fields
        .iter()
        .enumerate()
        .map(|(fi, f)| {
            let n = ident(&f.name);
            if fx.cx.is_boxed(id, variant, fi as u32) { format!("{n}: Box::new({n})") } else { n }
        })
        .collect();
    format!(
        "/// Builds one, checking every field in order; a field left out (`None`) takes its default.\npub fn {fname}({}) -> R<Self> {{ {} Ok({path} {{ {} }}) }}",
        params.join(", "),
        lines.join(" "),
        inits.join(", ")
    )
}

pub fn module_file(cx: &mut Cx<'_>, module: ModuleId) -> String {
    let lir = cx.lir;
    let mut out = type_items(cx, module);

    for (i, c) in lir.claims.iter().enumerate() {
        if c.module != module {
            continue;
        }
        let id = spec_lang_lir::ClaimId(i as u32);
        let (_, name) = crate::dty::claim_name(cx, id);
        let m = cx.claim_m[i];
        let t = cx.ty(&c.self_ty, Some(module));
        let mut fx = fx_for(cx, module, m, &[], &[&c.body]);
        let v = fx.value(&c.body);
        let mp = if m { "m: &mut Machine, " } else { "" };
        out.push(format!(
            "/// The claim `{}`.\npub fn {name}({mp}self_: {t}) -> R<bool> {{ {} {} }}",
            c.text,
            v.pre.join(" "),
            crate::code::ok(&v.text)
        ));
    }

    for (i, c) in lir.consts.iter().enumerate() {
        if c.info.module != module {
            continue;
        }
        let id = spec_lang_ir::ConstId(i as u32);
        let d = doc(cx, &c.info, "Constant");
        if cx.const_literal(id) {
            let (t, v) = match &c.value.kind {
                ExprKind::Text(s) => ("&str".to_string(), format!("{s:?}")),
                ExprKind::Int(n) => ("Int".to_string(), crate::expr::int_lit(*n).text),
                ExprKind::Bool(b) => ("bool".to_string(), b.to_string()),
                _ => continue,
            };
            out.push(format!("{d}\npub const {}: {t} = {v};", screaming(&c.info.name)));
            continue;
        }
        let m = cx.const_m[i];
        let t = cx.ty(&c.ty, Some(module));
        let mut fx = fx_for(cx, module, m, &[], &[&c.value]);
        let v = fx.value(&c.value);
        let mp = if m { "m: &mut Machine" } else { "" };
        out.push(format!("{d}\npub fn {}({mp}) -> R<{t}> {{ {} {} }}", ident(&c.info.name), v.pre.join(" "), crate::code::ok(&v.text)));
    }

    for (i, f) in lir.fns.iter().enumerate() {
        if f.info.module != module {
            continue;
        }
        let m = cx.fn_m[i];
        let mut exprs: Vec<&Expr> = f.params.iter().filter_map(|p| p.default.as_ref()).collect();
        if let Body::Expr(e) = &f.body {
            exprs.push(e);
        }
        let d = doc(cx, &f.info, "Fn");
        let mut params = sig(cx, &f.params, Some(module));
        if m {
            params.insert(0, "m: &mut Machine".into());
        }
        let ret = cx.ty(&f.result, Some(module));
        let mut fx = fx_for(cx, module, m, &f.params, &exprs);
        let mut lines = prologue(&mut fx, &f.params, &f.info.qualified);
        lines.extend(body(&mut fx, &f.info, &f.params, &f.body, &f.result, f.result_check.as_ref()));
        out.push(format!("{d}\npub fn {}({}) -> R<{ret}> {{ {} }}", ident(&f.info.name), params.join(", "), lines.join(" ")));
    }

    for r in &lir.rules {
        if r.info.module != module {
            continue;
        }
        let mut exprs: Vec<&Expr> = r.params.iter().filter_map(|p| p.default.as_ref()).collect();
        exprs.extend(r.guards.iter().map(|g| &g.cond));
        if let Body::Expr(e) = &r.body {
            exprs.push(e);
        }
        let d = doc(cx, &r.info, "Rule");
        let mut params = sig(cx, &r.params, Some(module));
        params.insert(0, "m: &mut Machine".into());
        let ret = cx.ty(&r.result, Some(module));
        let mut fx = fx_for(cx, module, true, &r.params, &exprs);
        let mut lines = prologue(&mut fx, &r.params, &r.info.qualified);
        for g in &r.guards {
            let c = fx.value(&g.cond);
            if !c.fail && c.pre.is_empty() {
                lines.push(format!("if !{} {{ return Err(rt::fail({:?}, Vec::new())); }}", c.operand(), g.error));
            } else {
                lines.push(format!("rt::guard(rt::catch(|| {}), {:?})?;", crate::code::ok(&c.seal()), g.error));
            }
        }
        lines.extend(body(&mut fx, &r.info, &r.params, &r.body, &r.result, r.result_check.as_ref()));
        out.push(format!(
            "{d}\n///\n/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.\npub fn {}({}) -> R<{ret}> {{ m.atomic(|m| {{ {} }}) }}",
            ident(&r.info.name),
            params.join(", "),
            lines.join(" ")
        ));
    }

    for t in &lir.tests {
        if t.info.module != module {
            continue;
        }
        let d = doc(cx, &t.info, "Test");
        let mut params = sig(cx, &t.params, Some(module));
        params.insert(0, "m: &mut Machine".into());
        let wrap = Expr::new(ExprKind::Block(t.body.clone()), LTy::Unit, t.body.span);
        let mut fx = fx_for(cx, module, true, &t.params, &[&wrap]);
        fx.discard = true;
        let lines = fx.fn_body(&t.body, &LTy::Unit);
        out.push(format!("{d}\npub fn {}({}) -> R<()> {{ {} }}", ident(&t.info.name), params.join(", "), lines.join(" ")));
    }

    for l in &lir.laws {
        if l.info.module != module {
            continue;
        }
        let d = doc(cx, &l.info, "Law");
        let mut params = sig(cx, &l.params, Some(module));
        params.insert(0, "m: &mut Machine".into());
        let mut fx = fx_for(cx, module, true, &l.params, &[&l.body]);
        let v = fx.value(&l.body);
        out.push(format!(
            "{d}\n#[allow(unused_variables)]\npub fn {}({}) -> R<bool> {{ {} {} }}",
            ident(&l.info.name),
            params.join(", "),
            v.pre.join(" "),
            crate::code::ok(&v.text)
        ));
    }
    out.join("\n\n")
}
