//! A module's states and rules: the world as one value, the errors its guards
//! name, one event per rule, one source per rule (guards in written order, then
//! the body, writing the world in place), and the dispatch of an event to its rule.
//!
//! A rule is atomic: all its writes or none. Guards run first and the body
//! has no failure point after a write (a map read with no guard is refused at
//! build time), so a refused rule has written nothing.

use spec_lang_emit_rust::front::TRule;
use spec_lang_emit_rust::front::facts::RuleFacts;
use spec_lang_emit_rust::front::typed::*;
use spec_lang_emit_rust::provenance::Origin;
use spec_lang_ir::{ModuleId, RuleId};

use super::expr::{Env, Local, R, uses};
use super::model::{Place, fn_def, place};
use super::{Gen, camel, ident, snake};
use crate::ast::*;
use crate::format::Unit;

/// The module that owns the states: the board.
pub fn state_module(g: &Gen<'_, '_>) -> Option<ModuleId> {
    g.f.states.first().map(|s| g.f.prog.state(s.id).info.module)
}

fn refusal() -> Ty {
    Ty::name("Refusal")
}

/// The rule's reply as C++, `void` for none.
fn reply(g: &Gen<'_, '_>, r: &TRule) -> Ty {
    g.cty(&r.result)
}

/// The rule's return type: `std::expected<reply, Refusal>` when it has guards.
pub fn ret(g: &Gen<'_, '_>, r: &TRule) -> Ty {
    if r.guards.is_empty() { reply(g, r) } else { Ty::generic("std::expected", vec![reply(g, r), refusal()]) }
}

fn facts<'a>(g: &'a Gen<'_, '_>, id: RuleId) -> &'a RuleFacts {
    let i = g.f.rules.iter().position(|r| r.id == id).unwrap_or(0);
    &g.f.facts.rules[i]
}

fn params(g: &Gen<'_, '_>, r: &TRule) -> Vec<Param> {
    let world = if facts(g, r.id).read_only() { Ty::name("World").cref() } else { Ty::name("World").mref() };
    let mut ps = vec![
        Param { ty: world, name: "w".into(), default: None },
        Param { ty: Ty::name("Lines").mref(), name: "out".into(), default: None },
    ];
    for p in &r.params {
        ps.push(Param { ty: g.cty(&p.ty), name: ident(&p.name), default: None });
    }
    ps
}

/// World, errors, events and rule declarations, for the module header.
pub fn header_decls(g: &Gen<'_, '_>, unit: &mut Unit, module: ModuleId) -> R<Vec<Decl>> {
    let prog = g.f.prog;
    let mut out = Vec::new();
    if state_module(g) == Some(module) {
        let first = prog.state(g.f.states[0].id);
        let mut members = Vec::new();
        let mut names = Vec::new();
        for s in &g.f.states {
            let d = prog.state(s.id);
            names.push(prog.snippet(d.info.span).trim().trim_end_matches(';').to_string());
            let init = match &s.init {
                Some(x) => Some(Env::new(g, &d.info).expr(x)?),
                None => None,
            };
            members.push(Member::Field { ty: g.cty(&s.ty), name: ident(&d.info.name), init });
        }
        members.push(Member::Func(Func {
            doc: vec![],
            prefix: vec![],
            ret: Ty::name("bool"),
            name: "operator==".into(),
            params: vec![Param { ty: Ty::name("World").cref(), name: String::new(), default: None }],
            suffix: vec!["const", "= default"],
            body: None,
        }));
        out.push(unit.mark(Origin::new(g.path(&first.info), g.at(first.info.span), "state: the world")));
        out.push(Decl::Struct {
            doc: vec![
                "Every state of the module, as one value; a rule writes it in place, only after its guards held."
                    .into(),
                names.join("; "),
            ],
            keyword: "struct",
            name: "World".into(),
            members,
        });
    }
    let rules: Vec<&TRule> = g.f.rules.iter().filter(|r| prog.rule(r.id).info.module == module).collect();
    if rules.is_empty() {
        return Ok(out);
    }
    let first = &prog.rule(rules[0].id).info;
    out.push(unit.mark(g.origin(first, "the errors the guards name")));
    out.push(Decl::Enum {
        doc: vec!["The errors the guards declare. A refused rule has changed nothing and sent nothing.".into()],
        name: "Refusal".into(),
        underlying: Ty::name("std::uint8_t"),
        variants: g.f.errors.clone(),
    });
    out.push(Decl::Func(Func {
        doc: vec![],
        prefix: vec!["[[nodiscard]]", "constexpr"],
        ret: Ty::name("std::string_view"),
        name: "name".into(),
        params: vec![Param { ty: refusal(), name: "e".into(), default: None }],
        suffix: vec!["noexcept"],
        body: Some(vec![
            Stmt::Switch {
                on: Expr::id("e"),
                cases: g
                    .f
                    .errors
                    .iter()
                    .map(|n| (Expr::id(format!("Refusal::{n}")), vec![Stmt::Return(Some(Expr::str(n.clone())))]))
                    .collect(),
            },
            Stmt::Expr(Expr::call("std::unreachable", vec![])),
        ]),
    }));
    out.push(Decl::Comment("The lines out.line sends, collected; they leave only after commit.".into()));
    out.push(Decl::Using { name: "Lines".into(), ty: Ty::generic("std::vector", vec![Ty::name("std::string")]) });
    // Events.
    for r in &rules {
        let info = &prog.rule(r.id).info;
        out.push(unit.mark(g.origin(info, "event: the rule's parameters")));
        let mut members = Vec::new();
        for p in &r.params {
            let init = match &p.default {
                Some(d) => Some(match Env::new(g, info).expr(d)? {
                    Expr::Brace(_, xs) if xs.is_empty() => Expr::Brace(None, vec![]),
                    v => v,
                }),
                None => None,
            };
            members.push(Member::Field { ty: g.cty(&p.ty), name: ident(&p.name), init });
        }
        out.push(Decl::Struct {
            doc: vec![format!("A call of rule {}.", info.name)],
            keyword: "struct",
            name: camel(&info.name),
            members,
        });
    }
    out.push(unit.mark(g.origin(first, "event: any rule of the module")));
    out.push(Decl::Using {
        name: "Event".into(),
        ty: Ty::generic("std::variant", rules.iter().map(|r| Ty::name(camel(&prog.rule(r.id).info.name))).collect()),
    });
    let readers: Vec<Expr> = rules
        .iter()
        .filter(|r| facts(g, r.id).read_only())
        .map(|r| {
            Expr::CallT(
                Box::new(Expr::id("std::holds_alternative")),
                vec![Ty::name(camel(&prog.rule(r.id).info.name))],
                vec![Expr::id("e")],
            )
        })
        .collect();
    let any_reader = readers.into_iter().reduce(|a, b| Expr::bin("||", a, b)).unwrap_or(Expr::Bool(false));
    out.push(Decl::Func(Func {
        doc: vec!["Whether the event's rule writes the world: its @uses names a write.".into()],
        prefix: vec!["[[nodiscard]]", "constexpr"],
        ret: Ty::name("bool"),
        name: "writes".into(),
        params: vec![Param { ty: Ty::name("Event").cref(), name: "e".into(), default: None }],
        suffix: vec!["noexcept"],
        body: Some(vec![Stmt::Return(Some(any_reader.negate()))]),
    }));
    for r in &rules {
        let info = &prog.rule(r.id).info;
        if let Some(p) = projection(g, r) {
            out.extend(projected_world(g, unit, r, p)?);
            continue;
        }
        out.push(unit.mark(g.origin(info, "rule signature")));
        let ret = ret(g, r);
        let mut prefix = Vec::new();
        if ret != Ty::Void {
            prefix.push("[[nodiscard]]");
        }
        let mut doc = vec![signature(g, r)];
        if facts(g, r.id).read_only() {
            doc.push("It only reads (no write in its @uses): the world is const.".into());
        }
        out.push(Decl::Func(Func {
            doc,
            prefix,
            ret,
            name: ident(&info.name),
            params: params(g, r),
            suffix: vec![],
            body: None,
        }));
    }
    out.push(unit.mark(g.origin(first, "dispatch of an event to its rule")));
    out.push(Decl::Func(Func {
        doc: vec!["Runs the event's rule. Replies have no reader on a command line and stop here.".into()],
        prefix: vec!["[[nodiscard]]"],
        ret: Ty::generic("std::expected", vec![Ty::Void, refusal()]),
        name: "apply".into(),
        params: vec![
            Param { ty: Ty::name("Event"), name: "event".into(), default: None },
            Param { ty: Ty::name("World").mref(), name: "w".into(), default: None },
            Param { ty: Ty::name("Lines").mref(), name: "out".into(), default: None },
        ],
        suffix: vec![],
        body: None,
    }));
    Ok(out)
}

/// The projection fact of a read-only rule, when its body can live in the header (no fn of its own).
pub fn projection<'a>(g: &'a Gen<'_, '_>, r: &TRule) -> Option<&'a spec_lang_emit_rust::front::facts::Projection> {
    if !g.on("store.read.projection") {
        return None;
    }
    let p = facts(g, r.id).projection.as_ref()?;
    let local = g.f.fns.iter().any(|f| place(g, f) == Place::Rule(r.id));
    if local { None } else { Some(p) }
}

/// `Stats`: the prefix of a rule's projected types.
pub fn view_name(g: &Gen<'_, '_>, r: &TRule) -> String {
    camel(&g.f.prog.rule(r.id).info.name)
}

/// The world a projected read builds for `r` (only the fields it reads of the projected map's values),
/// and the rule's body as a template over the world, so the full world and the projected one share it.
fn projected_world(
    g: &Gen<'_, '_>,
    unit: &mut Unit,
    r: &TRule,
    p: &spec_lang_emit_rust::front::facts::Projection,
) -> R<Vec<Decl>> {
    let prog = g.f.prog;
    let info = &prog.rule(r.id).info;
    let view = view_name(g, r);
    let st = g.f.states.iter().find(|s| s.id == p.state);
    let Some(T::Map(k, v)) = st.map(|s| &s.ty) else {
        return Err(g.unsupported(info.span, info, "a projection of a state that is not a Map"));
    };
    let T::Named(rec) = **v else {
        return Err(g.unsupported(info.span, info, "a projection of values that are not records"));
    };
    let fields = spec_lang_emit_rust::front::record_fields(prog, rec).unwrap_or(&[]);
    let typer = spec_lang_emit_rust::front::typer::Typer::new(prog, g.path(info));
    let mut members = Vec::new();
    for i in &p.fields {
        let f = &fields[*i];
        members.push(Member::Field { ty: g.cty(&typer.ty(&f.ty, f.span)?), name: ident(&f.name), init: None });
    }
    let record = format!("{view}{}", prog.ty(rec).info.name);
    let mut world = Vec::new();
    for s in &g.f.states {
        let d = prog.state(s.id);
        let ty = if s.id == p.state {
            Ty::generic("rt::FlatMap", vec![g.cty(k), Ty::name(record.clone())])
        } else {
            g.cty(&s.ty)
        };
        let init = match &s.init {
            Some(x) => Some(Env::new(g, &d.info).expr(x)?),
            None => None,
        };
        world.push(Member::Field { ty, name: ident(&d.info.name), init });
    }
    unit.include("rt/flat_map.hpp");
    let origin =
        g.origin(info, "projected world for a read-only rule").strategy("store.read.projection", p.licence.clone());
    let mut out = vec![
        unit.mark(origin),
        Decl::Struct {
            doc: vec![format!("The fields of {} that rule {} reads.", prog.ty(rec).info.name, info.name)],
            keyword: "struct",
            name: record,
            members,
        },
        Decl::Struct {
            doc: vec![format!(
                "The world as rule {} reads it: a store read builds this, not the whole world.",
                info.name
            )],
            keyword: "struct",
            name: format!("{view}World"),
            members: world,
        },
    ];
    let mut f = rule_func(g, r, unit)?;
    f.prefix = vec!["template <class W>"];
    f.params[0].ty = Ty::name("W").cref();
    f.doc.push(format!("One body for World and {view}World.").to_string());
    out.push(unit.mark(g.origin(info, "rule body")));
    out.push(Decl::Func(f));
    Ok(out)
}

/// `pub rule done(no: TaskNo)` with its guards, as written.
fn signature(g: &Gen<'_, '_>, r: &TRule) -> String {
    let text = g.f.prog.snippet(g.f.prog.rule(r.id).info.span);
    text.lines()
        .filter(|l| l.contains("rule ") || l.trim_start().starts_with("@guard"))
        .map(|l| l.trim().trim_end_matches(" = {").to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// One rule's source file.
pub fn rule_source(g: &Gen<'_, '_>, r: &TRule) -> R<Unit> {
    let prog = g.f.prog;
    let info = &prog.rule(r.id).info;
    let module = g.module_of(info).to_string();
    let mut unit = Unit::new(
        format!("src/{module}/{}.cpp", snake(&info.name)),
        g.header(&[(format!("rule {}", info.name), info.span)]),
        g.origin(info, "file header"),
    );
    unit.include(format!("{}/{module}.hpp", g.ns));
    unit.include("rt/core.hpp");
    unit.include("rt/text.hpp");
    let mut items = Vec::new();
    let local: Vec<_> = g.f.fns.iter().filter(|f| place(g, f) == Place::Rule(r.id)).collect();
    if !local.is_empty() {
        let mut inner = Vec::new();
        for f in local {
            inner.push(unit.mark(g.origin(&prog.func(f.id).info, "fn, used by this rule only")));
            inner.push(Decl::Func(fn_def(g, &mut unit, f, false)?));
        }
        items.push(Decl::Namespace(String::new(), inner));
    }
    items.push(unit.mark(g.origin(info, "rule body")));
    items.push(Decl::Func(rule_func(g, r, &mut unit)?));
    unit.decls.push(Decl::Namespace(format!("{}::{module}", g.ns), items));
    Ok(unit)
}

/// A rule's definition: guards in written order, with the strategies the facts license, then the body.
fn rule_func(g: &Gen<'_, '_>, r: &TRule, unit: &mut Unit) -> R<Func> {
    let prog = g.f.prog;
    let info = &prog.rule(r.id).info;
    let rf = facts(g, r.id);
    let mut env = Env::new(g, info);
    env.fallible = !r.guards.is_empty();
    for p in &r.params {
        env.value(&p.name);
        if uses(&r.body, &p.name) + r.guards.iter().map(|gd| expr_uses(&gd.cond, &p.name)).sum::<usize>() == 1 {
            env.movable.push(p.name.clone());
        }
    }
    let mut body = Vec::new();
    let mut skip_remove = false;
    for (i, gd) in r.guards.iter().enumerate() {
        let mut origin = Origin::new(g.path(info), g.at(gd.span), format!("guard[{i}]"));
        let fail = Stmt::Return(Some(Expr::call("std::unexpected", vec![Expr::id(format!("Refusal::{}", gd.error))])));
        match (&rf.entry, i) {
            (Some(entry), 0) if entry.removes && g.on("entry.remove") => {
                origin = origin.strategy(
                    "entry.remove",
                    format!("the first guard is `{} in {}` and the rule's only other use of it is `remove({})` (facts.entry)", entry.key, prog.state(entry.state).info.name, entry.key),
                );
                body.push(Stmt::Comment(unit.anchor(origin)));
                let erase = env
                    .world
                    .clone()
                    .member(&ident(&prog.state(entry.state).info.name))
                    .method("erase", vec![Expr::id(ident(&entry.key))]);
                body.push(Stmt::If { cond: Expr::bin("==", erase, Expr::Int(0)), then: vec![fail], els: vec![] });
                skip_remove = true;
            }
            (Some(entry), 0) if !entry.removes && g.on("entry.lookup") => {
                let state = ident(&prog.state(entry.state).info.name);
                origin = origin.strategy(
                    "entry.lookup",
                    format!(
                        "the first guard is `{} in {}` and the rule reaches it only as {}[{}] (facts.entry)",
                        entry.key, state, state, entry.key
                    ),
                );
                body.push(Stmt::Comment(unit.anchor(origin)));
                let m = env.world.clone().member(&state);
                body.push(Stmt::Var {
                    ty: Ty::Auto.konst(),
                    name: "found".into(),
                    init: Some(m.clone().method("find", vec![Expr::id(ident(&entry.key))])),
                });
                body.push(Stmt::If {
                    cond: Expr::bin("==", Expr::id("found"), m.method("end", vec![])),
                    then: vec![fail],
                    els: vec![],
                });
                let st = g.f.states.iter().find(|s| s.id == entry.state);
                let vty = match st.map(|s| &s.ty) {
                    Some(T::Map(_, v)) => (**v).clone(),
                    _ => T::Unit,
                };
                let name = match &vty {
                    T::Named(id) => snake(&prog.ty(*id).info.name),
                    _ => "entry".into(),
                };
                let name = if r.params.iter().any(|p| p.name == name) { "entry".to_string() } else { name };
                let t = if entry.mutable { g.cty(&vty).mref() } else { g.cty(&vty).cref() };
                body.push(Stmt::Var {
                    ty: t,
                    name: name.clone(),
                    init: Some(Expr::Arrow(Box::new(Expr::id("found")), "second".into())),
                });
                env.entry = Some((entry.state, entry.key.clone(), Expr::id(name)));
            }
            _ => {
                body.push(Stmt::Comment(unit.anchor(origin)));
                let c = env.expr(&gd.cond)?;
                body.push(Stmt::If { cond: c.negate(), then: vec![fail], els: vec![] });
                // Plain: after `k in m` held, `m[k]` is a lookup that cannot miss.
                if let (Some(entry), 0) = (&rf.entry, i) {
                    let m = env.world.clone().member(&ident(&prog.state(entry.state).info.name));
                    env.entry =
                        Some((entry.state, entry.key.clone(), m.method("at", vec![Expr::id(ident(&entry.key))])));
                }
            }
        }
    }
    let mut i = 0;
    while i < r.body.len() {
        let s = &r.body[i];
        if let Some(rt) = &rf.retain
            && rt.at == i
            && g.on("filter-remove.retain")
        {
            let S::Let { value, .. } = &s.kind else { break };
            let K::Core(Op::Filter, ops) = &value.kind else { break };
            let K::Lambda { param, body: pred } = &ops[1].kind else { break };
            env.bind(param, Local::Value(Expr::id("entry").member("second")));
            let p = env.expr(pred)?;
            let state = ident(&prog.state(rt.state).info.name);
            let origin = Origin::new(g.path(info), g.at(s.span), "rule body").strategy(
                "filter-remove.retain",
                format!("`{}` is removed from `{state}` by its own key field and used after only for its size (facts.retain)", rt.name),
            );
            body.push(Stmt::Comment(unit.anchor(origin)));
            let erase = Expr::call(
                "std::erase_if",
                vec![
                    env.world.clone().member(&state),
                    Expr::Lambda {
                        capture: "&",
                        params: vec![(Ty::Auto.cref(), "entry".into())],
                        ret: None,
                        body: vec![Stmt::Return(Some(p))],
                    },
                ],
            );
            body.push(Stmt::Var {
                ty: Ty::Auto.konst(),
                name: ident(&rt.name),
                init: Some(Expr::Cast(Ty::name("std::int64_t"), Box::new(erase))),
            });
            env.bind(&rt.name, Local::Count(Expr::id(ident(&rt.name))));
            i += 2;
            continue;
        }
        if skip_remove && matches!(s.kind, S::Remove { .. }) {
            i += 1;
            continue;
        }
        stmt(&mut env, s, &mut body)?;
        i += 1;
    }
    for o in std::mem::take(&mut env.anchors) {
        unit.anchor(o);
    }
    if env.fallible && r.result == T::Unit && !matches!(body.last(), Some(Stmt::Return(_))) {
        body.push(Stmt::Return(Some(Expr::Brace(None, vec![]))));
    }
    let mut doc = Vec::new();
    if let Some(why) = &info.why {
        doc.push(format!("{why}."));
    }
    doc.push(signature(g, r));
    Ok(Func {
        doc,
        prefix: vec![],
        ret: ret(g, r),
        name: ident(&info.name),
        params: params(g, r),
        suffix: vec![],
        body: Some(body),
    })
}

fn expr_uses(e: &TExpr, name: &str) -> usize {
    let mut n = 0;
    super::expr::count(e, name, &mut n);
    n
}

/// One statement of a rule body.
pub fn stmt(env: &mut Env<'_, '_, '_>, s: &TStmt, out: &mut Vec<Stmt>) -> R<()> {
    let g = env.g;
    let no = |what: &str| g.unsupported(s.span, env.decl, what.to_string());
    match &s.kind {
        S::Let { name, value } => match &value.kind {
            K::Core(Op::Sort, ops) if !matches!(ops[0].ty, T::Set(_)) => {
                let [range, key] = ops.as_slice() else { return Err(no("`sort` without a key")) };
                let K::Lambda { param, body } = &key.kind else { return Err(no("`sort` without a lambda")) };
                let r = env.expr(range)?;
                out.push(Stmt::Var { ty: Ty::Auto, name: ident(name), init: Some(Expr::call("rt::refs", vec![r])) });
                env.bind(param, Local::Ptr(Expr::id(ident(param))));
                let k = env.expr(body)?;
                let cmp = Expr::Lambda {
                    capture: "&",
                    params: vec![(Ty::Auto.cptr(), ident(param))],
                    ret: None,
                    body: vec![Stmt::Return(Some(k))],
                };
                let unique = spec_lang_emit_rust::front::facts::sort_key_unique(g.f, &g.f.facts, range, key)
                    .filter(|_| g.on("sort.unstable"));
                let sort = match unique {
                    Some(licence) => {
                        let o = Origin::new(g.path(env.decl), g.at(s.span), "rule body: sort")
                            .strategy("sort.unstable", licence);
                        out.push(Stmt::Comment(env.anchor(o)));
                        "std::ranges::sort"
                    }
                    None => {
                        out.push(Stmt::Comment("Stable, ascending by the key, as `sort` means.".into()));
                        "std::ranges::stable_sort"
                    }
                };
                out.push(Stmt::Expr(Expr::call(sort, vec![Expr::id(ident(name)), Expr::Brace(None, vec![]), cmp])));
                env.bind(name, Local::PtrList(Expr::id(ident(name))));
            }
            K::Core(Op::Filter | Op::Values | Op::Keys, _) => {
                let r = env.expr(value)?;
                out.push(Stmt::Var {
                    ty: Ty::Auto.konst(),
                    name: ident(name),
                    init: Some(Expr::call("rt::refs", vec![r])),
                });
                env.bind(name, Local::PtrList(Expr::id(ident(name))));
            }
            _ => {
                let v = env.expr(value)?;
                out.push(Stmt::Var { ty: Ty::Auto.konst(), name: ident(name), init: Some(v) });
                env.value(name);
            }
        },
        S::Assign { state, path, value } => {
            let target = env.world.clone().member(&ident(&g.f.prog.state(*state).info.name));
            let v = env.owned(value)?;
            match path.as_slice() {
                [] => out.push(Stmt::Expr(Expr::bin("=", target, v))),
                [PathStep::Index(k)] => {
                    let k = env.expr(k)?;
                    out.push(Stmt::Expr(target.method("insert_or_assign", vec![k, v])));
                }
                [PathStep::Index(k), rest @ ..] => {
                    let K::Local(kn) = &k.kind else {
                        return Err(no("a write to an entry whose key is not a parameter"));
                    };
                    let Some((st, key, with)) = env.entry.clone() else {
                        return Err(no("a write to m[k] with no guard that k is in m"));
                    };
                    if st != *state || &key != kn {
                        return Err(no("a write to an entry other than the guarded one"));
                    }
                    let mut place = with;
                    let mut ty = match g.f.states.iter().find(|s| s.id == *state).map(|s| &s.ty) {
                        Some(T::Map(_, v)) => (**v).clone(),
                        _ => T::Unit,
                    };
                    for step in rest {
                        let PathStep::Field(i) = step else { return Err(no("a nested index")) };
                        let T::Named(id) = ty else { return Err(no("a field of a value that is not a record")) };
                        let fields = spec_lang_emit_rust::front::record_fields(g.f.prog, id).unwrap_or(&[]);
                        place = place.member(&ident(&fields[*i].name));
                        ty = spec_lang_emit_rust::front::typer::Typer::new(g.f.prog, String::new())
                            .ty(&fields[*i].ty, fields[*i].span)?;
                    }
                    out.push(Stmt::Expr(Expr::bin("=", place, v)));
                }
                _ => return Err(no("this place")),
            }
        }
        S::Remove { state, key } => {
            // A copy of the key: it may live in the entry the erase destroys.
            let k = Expr::call("auto", vec![env.expr(key)?]);
            out.push(Stmt::Expr(
                env.world.clone().member(&ident(&g.f.prog.state(*state).info.name)).method("erase", vec![k]),
            ));
        }
        S::Expr(e) => out.push(Stmt::Expr(env.expr(e)?)),
        S::Return(None) => out.push(Stmt::Return(if env.fallible { Some(Expr::Brace(None, vec![])) } else { None })),
        S::Return(Some(e)) => out.push(Stmt::Return(Some(env.owned(e)?))),
        S::If { cond, then, els } => {
            let c = env.expr(cond)?;
            let mut t = Vec::new();
            for x in then {
                stmt(env, x, &mut t)?;
            }
            let mut e = Vec::new();
            for x in els {
                stmt(env, x, &mut e)?;
            }
            out.push(Stmt::If { cond: c, then: t, els: e });
        }
        S::For { var, iter, body } => {
            let (ty, how) = match &iter.kind {
                K::Local(n) if matches!(env.locals.get(n), Some(Local::PtrList(_))) => {
                    (Ty::Auto.cptr(), Local::Ptr(Expr::id(ident(var))))
                }
                _ => (Ty::Auto.cref(), Local::Value(Expr::id(ident(var)))),
            };
            let range = env.expr(iter)?;
            env.bind(var, how);
            let mut b = Vec::new();
            for x in body {
                stmt(env, x, &mut b)?;
            }
            out.push(Stmt::For { ty, name: ident(var), range, body: b });
        }
        S::Assert(_) | S::ExpectValue { .. } | S::ExpectFail { .. } => return Err(no("a test statement in a rule")),
    }
    Ok(())
}

/// `src/<module>/apply.cpp`: an event to its rule.
pub fn apply_source(g: &Gen<'_, '_>, module: ModuleId) -> R<Unit> {
    let prog = g.f.prog;
    let short = g.mods.get(&module).cloned().unwrap_or_default();
    let rules: Vec<&TRule> = g.f.rules.iter().filter(|r| prog.rule(r.id).info.module == module).collect();
    let first = &prog.rule(rules[0].id).info;
    let mut unit = Unit::new(
        format!("src/{short}/apply.cpp"),
        g.header(
            &rules
                .iter()
                .map(|r| (format!("rule {}", prog.rule(r.id).info.name), prog.rule(r.id).info.span))
                .collect::<Vec<_>>(),
        ),
        g.origin(first, "file header"),
    );
    unit.include(format!("{}/{short}.hpp", g.ns));
    unit.include("rt/core.hpp");
    let result = Ty::generic("std::expected", vec![Ty::Void, refusal()]);
    let mut lambdas = Vec::new();
    for r in &rules {
        let info = &prog.rule(r.id).info;
        let mut args = vec![Expr::id("w"), Expr::id("out")];
        for p in &r.params {
            let field = Expr::id("e").member(&ident(&p.name));
            args.push(if g.trivial(&p.ty) { field } else { field.moved() });
        }
        let call = Expr::call(ident(&info.name), args);
        let body = match (r.guards.is_empty(), &r.result) {
            (true, T::Unit) => vec![Stmt::Expr(call), Stmt::Return(Some(Expr::Brace(None, vec![])))],
            (true, _) => {
                vec![Stmt::Expr(Expr::Cast(Ty::Void, Box::new(call))), Stmt::Return(Some(Expr::Brace(None, vec![])))]
            }
            (false, T::Unit) => vec![Stmt::Return(Some(call))],
            (false, _) => vec![Stmt::Return(Some(call.method(
                "transform",
                vec![Expr::Lambda {
                    capture: "",
                    params: vec![(Ty::Auto.cref(), String::new())],
                    ret: None,
                    body: vec![],
                }],
            )))],
        };
        let param = if r.params.is_empty() { String::new() } else { "e".into() };
        lambdas.push(Expr::Lambda {
            capture: "&",
            params: vec![(Ty::name(camel(&info.name)).mref(), param)],
            ret: Some(result.clone()),
            body,
        });
    }
    let visit =
        Expr::call("std::visit", vec![Expr::Brace(Some(Ty::name("rt::Overloaded")), lambdas), Expr::id("event")]);
    let items = vec![
        unit.mark(g.origin(first, "dispatch of an event to its rule")),
        Decl::Func(Func {
            doc: vec![],
            prefix: vec![],
            ret: result,
            name: "apply".into(),
            params: vec![
                Param { ty: Ty::name("Event"), name: "event".into(), default: None },
                Param { ty: Ty::name("World").mref(), name: "w".into(), default: None },
                Param { ty: Ty::name("Lines").mref(), name: "out".into(), default: None },
            ],
            suffix: vec![],
            body: Some(vec![Stmt::Return(Some(visit))]),
        }),
    ];
    unit.decls.push(Decl::Namespace(format!("{}::{short}", g.ns), items));
    Ok(unit)
}
