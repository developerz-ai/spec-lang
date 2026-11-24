//! Types and fns of one module: sums as `enum class` with name tables, refined
//! types as classes whose only way in is `make` (the claim is checked there and
//! nowhere else), records as aggregates, fns `constexpr` where their body allows.

use spec_lang_emit_rust::front::typed::*;
use spec_lang_emit_rust::front::typer::Typer;
use spec_lang_emit_rust::front::{TFn, record_fields};
use spec_lang_ir::{FnId, ModuleId, TypeDef, TypeId};

use super::expr::{Env, Local, R};
use super::{Gen, ident, snake};
use crate::ast::*;
use crate::format::Unit;

/// Where a fn's definition lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// `constexpr` in the module header.
    Header,
    /// Declared in the header, defined in `src/<module>.cpp`.
    Source,
    /// Not `pub` and used by exactly one rule of its module: inside that rule's source.
    Rule(spec_lang_ir::RuleId),
}

/// The types of `module`, in an order C++ can compile: sums, the fns claims call, refined types,
/// records, then the remaining fns.
pub fn types_and_fns(g: &Gen<'_, '_>, unit: &mut Unit, module: ModuleId) -> R<Vec<Decl>> {
    let prog = g.f.prog;
    let m = prog.module(module);
    let mut out = Vec::new();
    let types: Vec<TypeId> = m
        .decls
        .iter()
        .filter_map(|d| match d {
            spec_lang_ir::DeclRef::Type(t) if !matches!(prog.ty(*t).def, TypeDef::Rules(_)) => Some(*t),
            _ => None,
        })
        .collect();
    let fns: Vec<&TFn> = g.f.fns.iter().filter(|f| prog.func(f.id).info.module == module).collect();
    let (early, late): (Vec<&TFn>, Vec<&TFn>) = fns.iter().partition(|f| {
        f.params.iter().all(|p| !matches!(&p.ty, T::Named(id) if !matches!(prog.ty(*id).def, TypeDef::Sum(_))))
    });
    for t in types.iter().filter(|t| matches!(prog.ty(**t).def, TypeDef::Sum(_))) {
        out.push(unit.mark(g.origin(&prog.ty(*t).info, "type")));
        out.extend(sum(g, *t)?);
    }
    for f in &early {
        if let Some(d) = fn_decl(g, unit, f)? {
            out.push(unit.mark(g.origin(&prog.func(f.id).info, "fn")));
            out.push(d);
        }
    }
    for t in types.iter().filter(|t| matches!(prog.ty(**t).def, TypeDef::Alias(_))) {
        out.push(unit.mark(g.origin(&prog.ty(*t).info, "type with its claim as the validated constructor")));
        out.push(alias(g, *t)?);
    }
    for t in types.iter().filter(|t| matches!(prog.ty(**t).def, TypeDef::Record(_))) {
        out.push(unit.mark(g.origin(&prog.ty(*t).info, "type")));
        out.push(record(g, *t)?);
    }
    for f in &late {
        if let Some(d) = fn_decl(g, unit, f)? {
            out.push(unit.mark(g.origin(&prog.func(f.id).info, "fn")));
            out.push(d);
        }
    }
    Ok(out)
}

fn sum(g: &Gen<'_, '_>, id: TypeId) -> R<Vec<Decl>> {
    let d = g.f.prog.ty(id);
    let TypeDef::Sum(vs) = &d.def else { return Ok(Vec::new()) };
    if vs.iter().any(|v| !v.fields.is_empty()) {
        return Err(g.unsupported(d.info.span, &d.info, "a sum type with fields"));
    }
    let name = d.info.name.clone();
    let names: Vec<String> = vs.iter().map(|v| v.name.clone()).collect();
    let ty = Ty::name(name.clone());
    let to_name = Func {
        doc: vec!["The variant's name as the spec writes it.".into()],
        prefix: vec!["[[nodiscard]]", "constexpr"],
        ret: Ty::name("std::string_view"),
        name: "name".into(),
        params: vec![Param { ty: ty.clone(), name: "v".into(), default: None }],
        suffix: vec!["noexcept"],
        body: Some(vec![
            Stmt::Switch {
                on: Expr::id("v"),
                cases: names
                    .iter()
                    .map(|n| (Expr::id(format!("{name}::{n}")), vec![Stmt::Return(Some(Expr::str(n.clone())))]))
                    .collect(),
            },
            Stmt::Expr(Expr::call("std::unreachable", vec![])),
        ]),
    };
    let mut from = Vec::new();
    for n in &names {
        from.push(Stmt::If {
            cond: Expr::bin("==", Expr::id("text"), Expr::str(n.clone())),
            then: vec![Stmt::Return(Some(Expr::id(format!("{name}::{n}"))))],
            els: vec![],
        });
    }
    from.push(Stmt::Return(Some(Expr::id("std::nullopt"))));
    let named = Func {
        doc: vec![format!("The {name} a name denotes, as the spec writes it.")],
        prefix: vec!["[[nodiscard]]", "constexpr"],
        ret: Ty::generic("std::optional", vec![ty]),
        name: format!("{}_named", snake(&name)),
        params: vec![Param { ty: Ty::name("std::string_view"), name: "text".into(), default: None }],
        suffix: vec!["noexcept"],
        body: Some(from),
    };
    Ok(vec![
        Decl::Enum {
            doc: vec![format!("type {name} = {}.", names.join(" | "))],
            name: name.clone(),
            underlying: Ty::name("std::uint8_t"),
            variants: names,
        },
        Decl::Var {
            doc: vec![],
            prefix: vec!["inline", "constexpr"],
            ty: Ty::name("std::size_t"),
            name: format!("{}_count", snake(&name)),
            init: Expr::Int(vs.len() as i128),
        },
        Decl::Func(to_name),
        Decl::Func(named),
    ])
}

fn alias(g: &Gen<'_, '_>, id: TypeId) -> R<Decl> {
    let prog = g.f.prog;
    let d = prog.ty(id);
    let name = d.info.name.clone();
    let Some((base, claim)) = spec_lang_emit_rust::front::typer::alias_base(prog, id) else {
        return Err(g.unsupported(d.info.span, &d.info, "an alias of this type"));
    };
    let claim_text = claim.map(|c| prog.snippet(c.expr.span).to_string());
    let (field_ty, param_ty, getter, getter_ty, constexpr) = match base {
        T::Int => (Ty::name("std::int64_t"), Ty::name("std::int64_t"), "value", Ty::name("std::int64_t"), true),
        T::Text => (Ty::name("std::string"), Ty::name("std::string"), "text", Ty::name("std::string").cref(), false),
        _ => return Err(g.unsupported(d.info.span, &d.info, "a refined type over this base")),
    };
    // The claim, typed with `self` as the base value.
    let check = match claim {
        Some(c) => {
            let mut t = Typer::new(prog, g.path(&d.info));
            t.bind("self", base.clone());
            let typed = t.expr(&c.expr, Some(&T::Bool))?;
            let mut env = Env::new(g, &d.info);
            env.bind("self", Local::Value(Expr::id("value")));
            Some(env.expr(&typed)?)
        }
        None => None,
    };
    // Text is valid UTF-8 everywhere in the program: the cli and the store check it where bytes come in.
    let valid = check;
    let mut make_body = Vec::new();
    if let Some(v) = valid {
        make_body.push(Stmt::If {
            cond: v.negate(),
            then: vec![Stmt::Return(Some(Expr::id("std::nullopt")))],
            els: vec![],
        });
    }
    let build = if base == T::Text { Expr::id("value").moved() } else { Expr::id("value") };
    make_body.push(Stmt::Return(Some(Expr::call(name.clone(), vec![build.clone()]))));
    let pre = |c: bool| if c { vec!["[[nodiscard]]", "static", "constexpr"] } else { vec!["[[nodiscard]]", "static"] };
    let mut members = vec![Member::Func(Func {
        doc: vec!["The claim is checked here and nowhere else; nullopt is a value outside it.".into()],
        prefix: pre(constexpr),
        ret: Ty::generic("std::optional", vec![Ty::name(name.clone())]),
        name: "make".into(),
        params: vec![Param { ty: param_ty.clone(), name: "value".into(), default: None }],
        suffix: if constexpr { vec!["noexcept"] } else { vec![] },
        body: Some(make_body),
    })];
    if base == T::Int {
        members.push(Member::Func(Func {
            doc: vec!["For a value whose claim the build discharged; the caller's comment names the fact.".into()],
            prefix: pre(true),
            ret: Ty::name(name.clone()),
            name: "proven".into(),
            params: vec![Param { ty: param_ty.clone(), name: "value".into(), default: None }],
            suffix: vec!["noexcept"],
            body: Some(vec![Stmt::Return(Some(Expr::call(name.clone(), vec![Expr::id("value")])))]),
        }));
    }
    members.push(Member::Func(Func {
        doc: vec![],
        prefix: if constexpr { vec!["[[nodiscard]]", "constexpr"] } else { vec!["[[nodiscard]]"] },
        ret: getter_ty,
        name: getter.into(),
        params: vec![],
        suffix: vec!["const", "noexcept"],
        body: Some(vec![Stmt::Return(Some(Expr::id("value_")))]),
    }));
    members.push(Member::Func(Func {
        doc: vec![],
        prefix: vec![],
        ret: Ty::Auto,
        name: "operator<=>".into(),
        params: vec![Param { ty: Ty::name(name.clone()).cref(), name: String::new(), default: None }],
        suffix: vec!["const", "= default"],
        body: None,
    }));
    members.push(Member::Access("private"));
    members.push(Member::Ctor {
        explicit: true,
        constexpr,
        name: name.clone(),
        params: vec![Param { ty: param_ty, name: "value".into(), default: None }],
        inits: vec![("value_".into(), build)],
    });
    members.push(Member::Field { ty: field_ty, name: "value_".into(), init: None });
    let mut doc = vec![format!(
        "type {name} = {}{}. Only make() builds one, so every {name} meets the claim.",
        if base == T::Int { "Int" } else { "Text" },
        claim_text.map(|c| format!(" where {c}")).unwrap_or_default()
    )];
    if base == T::Int {
        doc.push("Int is exact in the spec; 64 bits hold every value the store carries (at most 2^53).".into());
    }
    Ok(Decl::Struct {
        doc,
        keyword: "class",
        name,
        members: std::iter::once(Member::Access("public")).chain(members).collect(),
    })
}

fn record(g: &Gen<'_, '_>, id: TypeId) -> R<Decl> {
    let prog = g.f.prog;
    let d = prog.ty(id);
    let Some(fields) = record_fields(prog, id) else { return Err(g.unsupported(d.info.span, &d.info, "a record")) };
    let mut members = Vec::new();
    let mut t = Typer::new(prog, g.path(&d.info));
    for f in fields {
        let fty = t.ty(&f.ty, f.span)?;
        let init = match &f.default {
            Some(x) => {
                let typed = t.expr(x, Some(&fty))?;
                let mut env = Env::new(g, &d.info);
                Some(match env.expr(&typed)? {
                    Expr::Brace(_, xs) if xs.is_empty() => Expr::Brace(None, vec![]),
                    v => v,
                })
            }
            None => None,
        };
        members.push(Member::Field { ty: g.cty(&fty), name: ident(&f.name), init });
    }
    members.push(Member::Func(Func {
        doc: vec![],
        prefix: vec![],
        ret: Ty::name("bool"),
        name: "operator==".into(),
        params: vec![Param { ty: Ty::name(d.info.name.clone()).cref(), name: String::new(), default: None }],
        suffix: vec!["const", "= default"],
        body: None,
    }));
    let shape = fields.iter().map(|f| prog.snippet(f.span).to_string()).collect::<Vec<_>>().join(", ");
    Ok(Decl::Struct {
        doc: vec![format!("type {}({shape}).", d.info.name)],
        keyword: "struct",
        name: d.info.name.clone(),
        members,
    })
}

/// Where `f` is defined: see [`Place`].
pub fn place(g: &Gen<'_, '_>, f: &TFn) -> Place {
    let prog = g.f.prog;
    let info = &prog.func(f.id).info;
    if !info.is_pub {
        let users: Vec<_> = g.f.rules.iter().filter(|r| rule_calls(r, f.id)).collect();
        let others = g.f.fns.iter().any(|o| o.id != f.id && body_calls(&o.body, f.id));
        if let [r] = users.as_slice()
            && !others
            && prog.rule(r.id).info.module == info.module
        {
            return Place::Rule(r.id);
        }
    }
    if constexpr_ok(g, f) { Place::Header } else { Place::Source }
}

fn rule_calls(r: &spec_lang_emit_rust::front::TRule, f: FnId) -> bool {
    body_calls(&r.body, f) || r.guards.iter().any(|gd| expr_calls(&gd.cond, f))
}

fn body_calls(body: &[TStmt], f: FnId) -> bool {
    let mut found = false;
    for s in body {
        spec_lang_emit_rust::front::facts::stmt_exprs(s, &mut |e| found |= expr_calls(e, f));
    }
    found
}

fn expr_calls(e: &TExpr, f: FnId) -> bool {
    if matches!(e.kind, K::CallFn { f: g, .. } if g == f) {
        return true;
    }
    let mut found = false;
    spec_lang_emit_rust::front::facts::children(e, &mut |c| found |= expr_calls(c, f));
    found
}

/// A fn whose body is literals, matches, fields, trivial values and calls of such fns: `constexpr`.
fn constexpr_ok(g: &Gen<'_, '_>, f: &TFn) -> bool {
    fn ok(g: &Gen<'_, '_>, e: &TExpr, depth: u32) -> bool {
        let here = match &e.kind {
            K::Bool(_) | K::Int(_) | K::Text(_) | K::Variant { .. } | K::Local(_) | K::Tuple(_) | K::Compare(..) => {
                true
            }
            K::Field(..) | K::Match { .. } | K::Not(_) | K::Bin(..) => true,
            K::Widen(x) => matches!(&x.ty, T::Named(id) if g.alias_base(*id) == Some(T::Int)),
            K::CallFn { f, .. } => {
                depth < 8 && g.f.func(*f).is_some_and(|c| c.body.iter().all(|s| stmt_ok(g, s, depth + 1)))
            }
            _ => false,
        };
        let mut kids = true;
        spec_lang_emit_rust::front::facts::children(e, &mut |c| kids &= ok(g, c, depth));
        here && kids
    }
    fn stmt_ok(g: &Gen<'_, '_>, s: &TStmt, depth: u32) -> bool {
        matches!(&s.kind, S::Return(Some(e)) if ok(g, e, depth))
    }
    let result_ok = g.trivial(&f.result) || (f.result == T::Text && literal_text(f));
    result_ok && f.body.iter().all(|s| stmt_ok(g, s, 0))
}

/// A fn returning Text whose every result is a literal: it returns `std::string_view`.
pub fn literal_text(f: &TFn) -> bool {
    fn leaves(e: &TExpr) -> bool {
        match &e.kind {
            K::Text(_) => true,
            K::Match { arms, .. } => arms.iter().all(|(_, v)| leaves(v)),
            K::If { then, els, .. } => leaves(then) && leaves(els),
            _ => false,
        }
    }
    f.result == T::Text && f.body.iter().all(|s| matches!(&s.kind, S::Return(Some(e)) if leaves(e)))
}

pub fn ret_ty(g: &Gen<'_, '_>, f: &TFn) -> Ty {
    if literal_text(f) { Ty::name("std::string_view") } else { g.cty(&f.result) }
}

/// The header declaration of a fn (or its constexpr definition); `None` when it lives in a rule's source.
fn fn_decl(g: &Gen<'_, '_>, unit: &mut Unit, f: &TFn) -> R<Option<Decl>> {
    match place(g, f) {
        Place::Rule(_) => Ok(None),
        Place::Header => Ok(Some(Decl::Func(fn_def(g, unit, f, true)?))),
        Place::Source => {
            let mut d = fn_def(g, unit, f, false)?;
            d.body = None;
            Ok(Some(Decl::Func(d)))
        }
    }
}

/// A fn's definition.
pub fn fn_def(g: &Gen<'_, '_>, unit: &mut Unit, f: &TFn, constexpr: bool) -> R<Func> {
    let prog = g.f.prog;
    let info = &prog.func(f.id).info;
    let mut env = Env::new(g, info);
    let mut params = Vec::new();
    for p in &f.params {
        env.value(&p.name);
        params.push(Param { ty: g.param_ty(&p.ty), name: ident(&p.name), default: None });
    }
    let mut body = Vec::new();
    for s in &f.body {
        match &s.kind {
            S::Return(Some(e @ TExpr { kind: K::Match { scrutinee, arms }, .. })) => {
                body.extend(env.match_stmts(e, scrutinee, arms, &mut |_, v| Ok(Stmt::Return(Some(v))))?);
            }
            S::Return(Some(e)) => body.push(Stmt::Return(Some(env.expr(e)?))),
            _ => return Err(g.unsupported(s.span, info, "a fn body with statements")),
        }
    }
    let _ = unit;
    let mut doc = Vec::new();
    if let Some(why) = &info.why {
        doc.push(format!("{why}."));
    }
    let line = prog.snippet(info.span).lines().find(|l| l.contains("fn ")).unwrap_or("").trim();
    doc.push(line.split(" = ").next().unwrap_or(line).trim_start_matches("pub ").to_string());
    let mut prefix = vec!["[[nodiscard]]"];
    if constexpr {
        prefix.push("constexpr");
    }
    Ok(Func {
        doc,
        prefix,
        ret: ret_ty(g, f),
        name: ident(&info.name),
        params,
        suffix: if constexpr { vec!["noexcept"] } else { vec![] },
        body: Some(body),
    })
}

/// The source with the module's out-of-line fn definitions; `None` when there are none.
pub fn source(g: &Gen<'_, '_>, module: ModuleId) -> R<Option<Unit>> {
    let prog = g.f.prog;
    let m = prog.module(module);
    let short = g.mods.get(&module).cloned().unwrap_or_default();
    let fns: Vec<&TFn> =
        g.f.fns.iter().filter(|f| prog.func(f.id).info.module == module && place(g, f) == Place::Source).collect();
    if fns.is_empty() {
        return Ok(None);
    }
    let names: Vec<(String, spec_lang_syntax::Span)> =
        fns.iter().map(|f| (format!("fn {}", prog.func(f.id).info.name), prog.func(f.id).info.span)).collect();
    let mut unit = Unit::new(
        format!("src/{short}.cpp"),
        g.header(&names),
        spec_lang_emit_rust::provenance::Origin::new(m.name.clone(), m.path.clone(), "file header"),
    );
    unit.include(format!("{}/{short}.hpp", g.ns));
    unit.include("rt/text.hpp");
    unit.include("rt/core.hpp");
    let mut items = Vec::new();
    for f in fns {
        items.push(unit.mark(g.origin(&prog.func(f.id).info, "fn")));
        items.push(Decl::Func(fn_def(g, &mut unit, f, false)?));
    }
    unit.decls.push(Decl::Namespace(format!("{}::{short}", g.ns), items));
    Ok(Some(unit))
}
