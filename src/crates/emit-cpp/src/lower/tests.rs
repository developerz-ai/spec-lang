//! The spec's tests as native tests: every `@cases` row, every `test` by name
//! (a test with parameters is a seeded property), and every `law` checked over
//! seeded traces. No mocks: rules are called directly on a world.

use std::collections::BTreeSet;

use spec_lang_emit_rust::front::typed::*;
use spec_lang_emit_rust::front::typer::{alias_base, int_claim_lower_bound};
use spec_lang_emit_rust::front::{TGiven, TLaw, TParam, TTest};
use spec_lang_emit_rust::provenance::Origin;
use spec_lang_ir::{CmpOp, ModuleId, RuleId, TestId, TypeDef};

use super::expr::{Env, Local, R};
use super::{Gen, camel, ident, snake};
use crate::ast::*;
use crate::format::Unit;

/// Traces per law, and steps per trace.
pub const TRACES: i128 = 240;
pub const STEPS: i128 = 40;
/// Runs per property test.
pub const RUNS: i128 = 200;

/// Every test file, with the test names each registers (for CMake).
pub fn units(g: &Gen<'_, '_>, board: &str) -> R<Vec<(Unit, String)>> {
    let mut out = Vec::new();
    out.push((support(g, board)?, String::new()));
    // @cases, one file per module that has any.
    let mut modules: Vec<ModuleId> =
        g.f.fns.iter().filter(|f| !f.cases.is_empty()).map(|f| g.f.prog.func(f.id).info.module).collect();
    modules.dedup();
    for m in modules {
        let short = g.mods.get(&m).cloned().unwrap_or_default();
        out.push((cases(g, m, &short)?, format!("{short}_cases")));
    }
    let mut test_mods: Vec<ModuleId> = g.f.tests.iter().map(|t| g.f.prog.test(t.id).info.module).collect();
    test_mods.dedup();
    for m in test_mods {
        let name = last(&g.f.prog.module(m).name);
        out.push((scenarios(g, m, board, &name)?, name));
    }
    let mut law_mods: Vec<ModuleId> = g.f.laws.iter().map(|l| g.f.prog.law(l.id).info.module).collect();
    law_mods.dedup();
    for m in law_mods {
        let name = last(&g.f.prog.module(m).name);
        out.push((laws(g, m, board, &name)?, name));
    }
    Ok(out)
}

fn last(module: &str) -> String {
    module.rsplit('.').next().unwrap_or(module).to_string()
}

fn test_env<'g, 'f, 'p>(g: &'g Gen<'f, 'p>, info: &'p spec_lang_ir::DeclInfo) -> Env<'g, 'f, 'p> {
    let mut env = Env::new(g, info);
    env.product = false;
    env
}

/// Tests used as another test's `@given`, or as a law's start world.
fn given_tests(g: &Gen<'_, '_>) -> Vec<TestId> {
    let mut out = BTreeSet::new();
    for t in &g.f.tests {
        for gv in &t.given {
            if let TGiven::Test { test, .. } = gv {
                out.insert(*test);
            }
        }
    }
    out.into_iter().collect()
}

/// `tests/support.hpp`: start worlds of tests used as givens, and value generators.
fn support(g: &Gen<'_, '_>, board: &str) -> R<Unit> {
    let app = &g.f.app().info;
    let mut unit = Unit::new(
        "tests/support.hpp",
        g.header(&[(format!("the tests and laws of app {}", app.name), app.span)]),
        g.origin(app, "file header"),
    );
    unit.include("rt/harness.hpp");
    unit.include("rt/random.hpp");
    unit.include("rt/trace.hpp");
    for m in g.mods.values() {
        unit.include(format!("{}/{m}.hpp", g.ns));
    }
    let mut items = Vec::new();
    for (i, t) in g.f.prog.types.iter().enumerate() {
        let id = spec_lang_ir::TypeId(i as u32);
        let (base, param) = match alias_base(g.f.prog, id) {
            Some((T::Int, Some(_))) => (T::Int, Ty::name("std::int64_t")),
            Some((T::Text, Some(_))) => (T::Text, Ty::name("std::string")),
            _ => continue,
        };
        let value = if base == T::Text { Expr::id("value").moved() } else { Expr::id("value") };
        items.push(unit.mark(g.origin(&t.info, "test literal of a refined type")));
        items.push(Decl::Func(Func {
            doc: vec![format!("A test literal as a {}: one outside the claim fails the test.", t.info.name)],
            prefix: vec!["inline"],
            ret: Ty::Name(g.type_path(id)),
            name: snake(&t.info.name),
            params: vec![Param { ty: param, name: "value".into(), default: None }],
            suffix: vec![],
            body: Some(vec![Stmt::Return(Some(Expr::call(
                "rt::test::must",
                vec![Expr::call(format!("{}::make", g.type_path(id)), vec![value])],
            )))]),
        }));
    }
    for id in given_tests(g) {
        let Some(t) = g.f.test(id) else { continue };
        let info = &g.f.prog.test(id).info;
        items.push(unit.mark(g.origin(info, "test end world, used as a @given")));
        let mut body = world_setup(g, t, board)?;
        let mut env = test_env(g, info);
        for s in &t.body {
            test_stmt(&mut env, s, &mut body)?;
        }
        body.push(Stmt::Return(Some(Expr::id("w"))));
        items.push(Decl::Func(Func {
            doc: vec![format!("The world test {} ends in.", info.name)],
            prefix: vec!["inline"],
            ret: Ty::name(format!("{board}::World")),
            name: format!("given_{}", info.name),
            params: vec![],
            suffix: vec![],
            body: Some(body),
        }));
    }
    // Generators for every type a property or a law draws.
    let mut needed: Vec<T> = Vec::new();
    let add = |t: &T, needed: &mut Vec<T>| collect_types(t, needed);
    for t in &g.f.tests {
        for p in &t.params {
            add(&p.ty, &mut needed);
        }
    }
    for l in &g.f.laws {
        for r in &l.rules {
            if let Some(tr) = g.f.rule(*r) {
                for p in &tr.params {
                    add(&p.ty, &mut needed);
                }
            }
        }
        for p in &l.others {
            add(&p.ty, &mut needed);
        }
    }
    for t in &needed {
        items.push(unit.mark(Origin::new(
            format!("{} generator {}", g.path(app), gen_name(g, t)),
            g.at(app.span),
            "value generator for properties and traces",
        )));
        items.push(Decl::Func(generator(g, t)?));
    }
    unit.decls.push(Decl::Namespace(format!("{}::support", g.ns), items));
    Ok(unit)
}

/// Element types first, so a generator is declared before it is used.
fn collect_types(t: &T, out: &mut Vec<T>) {
    if let T::Option(x) | T::Set(x) | T::List(x) = t {
        collect_types(x, out);
    }
    if !out.contains(t) {
        out.push(t.clone());
    }
}

fn gen_name(g: &Gen<'_, '_>, t: &T) -> String {
    match t {
        T::Named(id) => format!("gen_{}", snake(&g.f.prog.ty(*id).info.name)),
        T::Option(x) => format!("{}_option", gen_name(g, x)),
        T::Set(x) => format!("{}_set", gen_name(g, x)),
        T::List(x) => format!("{}_list", gen_name(g, x)),
        T::Bool => "gen_bool".into(),
        T::Int => "gen_int".into(),
        T::Text => "gen_text".into(),
        _ => "gen_value".into(),
    }
}

fn prng() -> Expr {
    Expr::id("prng")
}

fn generator(g: &Gen<'_, '_>, t: &T) -> R<Func> {
    let app = &g.f.app().info;
    let ty = g.cty(t);
    let below = |n: Expr| prng().method("below", vec![n]);
    let (doc, body): (&str, Vec<Stmt>) = match t {
        T::Bool => ("Either, evenly.", vec![Stmt::Return(Some(prng().method("coin", vec![])))]),
        T::Int => (
            "Small numbers.",
            vec![Stmt::Return(Some(Expr::Cast(Ty::name("std::int64_t"), Box::new(below(Expr::Int(16))))))],
        ),
        T::Text => (
            "A candidate text.",
            vec![Stmt::Return(Some(Expr::call(
                "std::string",
                vec![Expr::id("rt::random::texts").method("at", vec![below(Expr::id("rt::random::texts.size()"))])],
            )))],
        ),
        T::Named(id) => {
            let d = g.f.prog.ty(*id);
            match (&d.def, alias_base(g.f.prog, *id)) {
                (TypeDef::Sum(_), _) => (
                    "Any variant, evenly.",
                    vec![Stmt::Return(Some(Expr::Cast(
                        ty.clone(),
                        Box::new(below(Expr::id(format!("{}::{}_count", g.module_of(&d.info), snake(&d.info.name))))),
                    )))],
                ),
                (_, Some((T::Int, claim))) => {
                    let lb = claim.and_then(|c| int_claim_lower_bound(&c.expr)).unwrap_or(0);
                    let draw = Expr::Cond(
                        Box::new(Expr::bin("==", below(Expr::Int(20)), Expr::Int(0))),
                        Box::new(Expr::Cast(
                            Ty::name("std::int64_t"),
                            Box::new(Expr::bin(">>", prng().method("next", vec![]), Expr::id("2U"))),
                        )),
                        Box::new(Expr::bin(
                            "+",
                            Expr::Int(lb),
                            Expr::Cast(Ty::name("std::int64_t"), Box::new(below(Expr::Int(16)))),
                        )),
                    );
                    (
                        "Mostly small numbers from the claim's least value, which name what a trace made; now and then one far away.",
                        vec![Stmt::Return(Some(Expr::call(
                            "rt::test::must",
                            vec![Expr::call(format!("{}::make", g.type_path(*id)), vec![draw])],
                        )))],
                    )
                }
                (_, Some((T::Text, _))) => {
                    let name = &d.info.name;
                    let make = Expr::call(
                        format!("{}::make", g.type_path(*id)),
                        vec![Expr::call("std::string", vec![Expr::id("text")])],
                    );
                    let fill = Expr::Call(
                        Box::new(Expr::Lambda {
                            capture: "",
                            params: vec![],
                            ret: None,
                            body: vec![
                                Stmt::Var {
                                    ty: Ty::generic("std::vector", vec![ty.clone()]),
                                    name: "out".into(),
                                    init: None,
                                },
                                Stmt::For {
                                    ty: Ty::Auto.konst(),
                                    name: "text".into(),
                                    range: Expr::id("rt::random::texts"),
                                    body: vec![Stmt::IfInit {
                                        name: "v".into(),
                                        init: make.clone(),
                                        then: vec![Stmt::Expr(
                                            Expr::id("out").method("push_back", vec![Expr::id("v").deref()]),
                                        )],
                                    }],
                                },
                                Stmt::Return(Some(Expr::id("out"))),
                            ],
                        }),
                        vec![],
                    );
                    (
                        "A candidate text the claim admits.",
                        vec![
                            Stmt::Static {
                                ty: Ty::generic("std::vector", vec![ty.clone()]),
                                name: "pool".into(),
                                init: fill,
                            },
                            Stmt::If {
                                cond: Expr::id("pool").method("empty", vec![]),
                                then: vec![Stmt::Expr(Expr::call(
                                    "harness::fail",
                                    vec![Expr::str(format!("E-GEN: no candidate text is a {name}"))],
                                ))],
                                els: vec![],
                            },
                            Stmt::Return(Some(
                                Expr::id("pool").method("at", vec![below(Expr::id("pool").method("size", vec![]))]),
                            )),
                        ],
                    )
                }
                _ => return Err(g.unsupported(app.span, app, format!("generating a {}", d.info.name))),
            }
        }
        T::Option(x) => (
            "None half the time.",
            vec![
                Stmt::If {
                    cond: prng().method("coin", vec![]),
                    then: vec![Stmt::Return(Some(Expr::id("std::nullopt")))],
                    els: vec![],
                },
                Stmt::Return(Some(Expr::call(gen_name(g, x), vec![prng()]))),
            ],
        ),
        T::Set(x) => (
            "Up to three elements; repeats collapse.",
            vec![
                Stmt::Var { ty: ty.clone(), name: "out".into(), init: None },
                Stmt::Var { ty: Ty::Auto.konst(), name: "n".into(), init: Some(below(Expr::Int(4))) },
                Stmt::Count {
                    name: "i".into(),
                    ty: Ty::name("std::uint64_t"),
                    from: Expr::Int(0),
                    below: Expr::id("n"),
                    body: vec![Stmt::Expr(
                        Expr::id("out").method("insert", vec![Expr::call(gen_name(g, x), vec![prng()])]),
                    )],
                },
                Stmt::Return(Some(Expr::id("out"))),
            ],
        ),
        _ => return Err(g.unsupported(app.span, app, "generating a value of this type")),
    };
    Ok(Func {
        doc: vec![doc.into()],
        prefix: vec!["inline"],
        ret: ty,
        name: gen_name(g, t),
        params: vec![Param { ty: Ty::name("rt::random::Prng").mref(), name: "prng".into(), default: None }],
        suffix: vec![],
        body: Some(body),
    })
}

/// `board::World w; board::Lines out;` and the test's `@given`.
fn world_setup(g: &Gen<'_, '_>, t: &TTest, board: &str) -> R<Vec<Stmt>> {
    let info = &g.f.prog.test(t.id).info;
    let mut out = vec![
        Stmt::Var { ty: Ty::name(format!("{board}::World")), name: "w".into(), init: None },
        Stmt::Var { ty: Ty::name(format!("{board}::Lines")), name: "out".into(), init: None },
    ];
    let mut env = test_env(g, info);
    for gv in &t.given {
        match gv {
            TGiven::Test { test, .. } => {
                let name = &g.f.prog.test(*test).info.name;
                out.push(Stmt::Expr(Expr::bin(
                    "=",
                    Expr::id("w"),
                    Expr::call(format!("support::given_{name}"), vec![]),
                )));
            }
            TGiven::State { state, value, .. } => {
                let v = env.expr(value)?;
                out.push(Stmt::Expr(Expr::bin(
                    "=",
                    Expr::id("w").member(&ident(&g.f.prog.state(*state).info.name)),
                    v,
                )));
            }
        }
    }
    Ok(out)
}

/// One statement of a test body.
fn test_stmt(env: &mut Env<'_, '_, '_>, s: &spec_lang_emit_rust::front::typed::TStmt, out: &mut Vec<Stmt>) -> R<()> {
    let g = env.g;
    let fallible = |call: &TExpr| match &call.kind {
        K::CallRule { rule, .. } => g.f.rule(*rule).is_some_and(|r| !r.guards.is_empty()),
        _ => false,
    };
    match &s.kind {
        S::Let { name, value } => {
            let v = env.expr(value)?;
            out.push(Stmt::Var { ty: Ty::Auto.konst(), name: ident(name), init: Some(v) });
            env.value(name);
        }
        S::Expr(call @ TExpr { kind: K::CallRule { .. }, .. }) => {
            let c = env.expr(call)?;
            out.push(if fallible(call) {
                Stmt::Expr(Expr::call("CHECK", vec![c.method("has_value", vec![]).macro_arg()]))
            } else if call.ty == T::Unit {
                Stmt::Expr(c)
            } else {
                Stmt::Expr(Expr::Cast(Ty::Void, Box::new(c)))
            });
        }
        S::Expr(e) => out.push(Stmt::Expr(env.expr(e)?)),
        S::ExpectValue { call, value } => {
            let c = env.expr(call)?;
            let c = if fallible(call) { c.method("value", vec![]) } else { c };
            out.push(Stmt::Expr(Expr::call("CHECK_EQ", vec![c.macro_arg(), env.expr(value)?.macro_arg()])));
        }
        S::ExpectFail { call, error } => {
            if !fallible(call) {
                return Err(g.unsupported(s.span, env.decl, "expecting a failure of a rule with no guard"));
            }
            let c = env.expr(call)?;
            let board = match &call.kind {
                K::CallRule { rule, .. } => g.module_of(&g.f.prog.rule(*rule).info).to_string(),
                _ => String::new(),
            };
            let want =
                Expr::Brace(Some(Ty::name("std::optional")), vec![Expr::id(format!("{board}::Refusal::{error}"))]);
            out.push(Stmt::Expr(Expr::call(
                "CHECK_EQ",
                vec![Expr::call("rt::test::error_of", vec![c]), want.macro_arg()],
            )));
        }
        S::Assert(e) => {
            if let K::Compare(a, links) = &e.kind
                && let [(CmpOp::Eq, b)] = links.as_slice()
            {
                out.push(Stmt::Expr(Expr::call("CHECK_EQ", vec![env.expr(a)?.macro_arg(), env.expr(b)?.macro_arg()])));
            } else {
                out.push(Stmt::Expr(Expr::call("CHECK", vec![env.expr(e)?.macro_arg()])));
            }
        }
        S::Return(None) => out.push(Stmt::Return(None)),
        S::If { cond, then, els } => {
            let c = env.expr(cond)?;
            let mut t = Vec::new();
            for x in then {
                test_stmt(env, x, &mut t)?;
            }
            let mut e = Vec::new();
            for x in els {
                test_stmt(env, x, &mut e)?;
            }
            out.push(Stmt::If { cond: c, then: t, els: e });
        }
        S::For { var, iter, body } => {
            let range = env.expr(iter)?;
            env.value(var);
            let mut b = Vec::new();
            for x in body {
                test_stmt(env, x, &mut b)?;
            }
            out.push(Stmt::For { ty: Ty::Auto.cref(), name: ident(var), range, body: b });
        }
        _ => return Err(g.unsupported(s.span, env.decl, "this statement in a test")),
    }
    Ok(())
}

fn test_file(g: &Gen<'_, '_>, path: String, header: String, origin: Origin) -> Unit {
    let mut unit = Unit::new(path, header, origin);
    unit.include("rt/harness.hpp");
    unit.include("support.hpp");
    for m in g.mods.values() {
        unit.include(format!("{}/{m}.hpp", g.ns));
    }
    unit.include("rt/core.hpp");
    unit.include("rt/text.hpp");
    unit
}

fn test_decl(name: &str, body: Vec<Stmt>) -> Decl {
    // TEST(name) { body } is a macro the harness defines.
    let mut text = format!("TEST({name}) {{\n");
    for s in &body {
        crate::ast::stmt(s, &mut text);
    }
    text.push('}');
    Decl::Line(text)
}

fn cases(g: &Gen<'_, '_>, module: ModuleId, short: &str) -> R<Unit> {
    let prog = g.f.prog;
    let fns: Vec<_> = g.f.fns.iter().filter(|f| !f.cases.is_empty() && prog.func(f.id).info.module == module).collect();
    let mut unit = test_file(
        g,
        format!("tests/{short}_cases_test.cpp"),
        g.header(
            &fns.iter()
                .map(|f| (format!("@cases of fn {}", prog.func(f.id).info.name), prog.func(f.id).info.span))
                .collect::<Vec<_>>(),
        ),
        Origin::new(prog.module(module).name.clone(), prog.module(module).path.clone(), "file header"),
    );
    let mut items = Vec::new();
    for f in fns {
        let info = &prog.func(f.id).info;
        let mut body = Vec::new();
        let mut env = test_env(g, info);
        let path = format!("{}::{}", g.module_of(info), ident(&info.name));
        for (i, c) in f.cases.iter().enumerate() {
            let o = Origin::new(g.path(info), g.at(c.span), format!("case[{i}]"));
            body.push(Stmt::Comment(unit.anchor(o)));
            let args = c.args.iter().map(|a| env.expr(a)).collect::<R<Vec<_>>>()?;
            body.push(Stmt::Expr(Expr::call(
                "CHECK_EQ",
                vec![Expr::call(path.clone(), args).macro_arg(), env.expr(&c.expect)?.macro_arg()],
            )));
        }
        items.push(unit.mark(g.origin(info, "@cases: a table test")));
        items.push(test_decl(&format!("{}_cases", info.name), body));
    }
    unit.decls.push(Decl::Namespace(g.ns.clone(), items));
    Ok(unit)
}

fn scenarios(g: &Gen<'_, '_>, module: ModuleId, board: &str, name: &str) -> R<Unit> {
    let prog = g.f.prog;
    let tests: Vec<&TTest> = g.f.tests.iter().filter(|t| prog.test(t.id).info.module == module).collect();
    let mut unit = test_file(
        g,
        format!("tests/{name}_test.cpp"),
        g.header(
            &tests
                .iter()
                .map(|t| (format!("test {}", prog.test(t.id).info.name), prog.test(t.id).info.span))
                .collect::<Vec<_>>(),
        ),
        Origin::new(prog.module(module).name.clone(), prog.module(module).path.clone(), "file header"),
    );
    let mut items = Vec::new();
    for (n, t) in tests.iter().enumerate() {
        let info = &prog.test(t.id).info;
        let mut env = test_env(g, info);
        let mut inner = Vec::new();
        for p in &t.params {
            env.value(&p.name);
        }
        for s in &t.body {
            test_stmt(&mut env, s, &mut inner)?;
        }
        let setup = world_setup(g, t, board)?;
        let body = if t.params.is_empty() {
            let mut b = setup;
            b.extend(inner);
            // A test with no step still runs its @given.
            if t.body.is_empty() {
                b.push(Stmt::Expr(Expr::Cast(Ty::Void, Box::new(Expr::id("out")))));
            }
            b
        } else {
            property(g, t, n, setup, inner)
        };
        let role = if t.params.is_empty() { "test: a scenario" } else { "test: a property over generated parameters" };
        items.push(unit.mark(g.origin(info, role)));
        items.push(test_decl(&info.name, body));
    }
    unit.decls.push(Decl::Namespace(g.ns.clone(), items));
    Ok(unit)
}

fn property(g: &Gen<'_, '_>, t: &TTest, n: usize, setup: Vec<Stmt>, inner: Vec<Stmt>) -> Vec<Stmt> {
    let mut run = Vec::new();
    for p in &t.params {
        run.push(Stmt::Var {
            ty: Ty::Auto.konst(),
            name: ident(&p.name),
            init: Some(Expr::call(format!("support::{}", gen_name(g, &p.ty)), vec![prng()])),
        });
    }
    run.extend(setup);
    run.push(Stmt::Expr(Expr::Call(
        Box::new(Expr::Lambda { capture: "&", params: vec![], ret: None, body: inner }),
        vec![],
    )));
    vec![
        Stmt::Var {
            ty: Ty::name("rt::random::Prng"),
            name: "prng".into(),
            init: Some(Expr::call("rt::random::Prng", vec![Expr::Int(0x5EED + n as i128)])),
        },
        Stmt::Count { name: "run".into(), ty: Ty::name("int"), from: Expr::Int(0), below: Expr::Int(RUNS), body: run },
    ]
}

fn laws(g: &Gen<'_, '_>, module: ModuleId, board: &str, name: &str) -> R<Unit> {
    let prog = g.f.prog;
    let laws: Vec<&TLaw> = g.f.laws.iter().filter(|l| prog.law(l.id).info.module == module).collect();
    let mut unit = test_file(
        g,
        format!("tests/{name}_test.cpp"),
        g.header(
            &laws
                .iter()
                .map(|l| (format!("law {}", prog.law(l.id).info.name), prog.law(l.id).info.span))
                .collect::<Vec<_>>(),
        ),
        Origin::new(prog.module(module).name.clone(), prog.module(module).path.clone(), "file header"),
    );
    unit.include("rt/trace.hpp");
    let first = &prog.law(laws[0].id).info;
    let world = Ty::name(format!("{board}::World"));
    let event = Ty::name(format!("{board}::Event"));
    let mut inner = Vec::new();
    inner.push(unit.mark(g.origin(first, "law traces: size, start worlds, the step")));
    inner.push(Decl::Var {
        doc: vec![format!("Traces per law; each of {STEPS} steps, every law checked after every step.")],
        prefix: vec!["constexpr"],
        ty: Ty::name("std::uint64_t"),
        name: "traces".into(),
        init: Expr::Int(TRACES),
    });
    inner.push(Decl::Var {
        doc: vec![],
        prefix: vec!["constexpr"],
        ty: Ty::name("int"),
        name: "steps".into(),
        init: Expr::Int(STEPS),
    });
    let start = match given_tests(g).first() {
        Some(t) => Expr::Cond(
            Box::new(Expr::bin("==", Expr::bin("%", Expr::id("seed"), Expr::Int(3)), Expr::Int(0))),
            Box::new(Expr::call(format!("support::given_{}", prog.test(*t).info.name), vec![])),
            Box::new(Expr::Brace(Some(world.clone()), vec![])),
        ),
        None => Expr::Brace(Some(world.clone()), vec![]),
    };
    inner.push(Decl::Func(Func {
        doc: vec!["A trace starts from the initial world, or every third from a world a test's @given set up.".into()],
        prefix: vec![],
        ret: world.clone(),
        name: "start".into(),
        params: vec![Param { ty: Ty::name("std::uint64_t"), name: "seed".into(), default: None }],
        suffix: vec![],
        body: Some(vec![Stmt::Return(Some(start))]),
    }));
    inner.push(Decl::Func(Func {
        doc: vec!["Runs one event; true when its rule refused.".into()],
        prefix: vec![],
        ret: Ty::name("bool"),
        name: "refused".into(),
        params: vec![
            Param { ty: event.clone(), name: "event".into(), default: None },
            Param { ty: world.clone().mref(), name: "w".into(), default: None },
        ],
        suffix: vec![],
        body: Some(vec![
            Stmt::Var { ty: Ty::name(format!("{board}::Lines")), name: "out".into(), init: None },
            Stmt::Return(Some(
                Expr::call(format!("{board}::apply"), vec![Expr::id("event").moved(), Expr::id("w"), Expr::id("out")])
                    .method("has_value", vec![])
                    .negate(),
            )),
        ]),
    }));
    // Event parameter accessors the laws read (`s.event.no`).
    let mut args = BTreeSet::new();
    for l in &laws {
        find_event_args(&l.body, &mut args);
    }
    for (arg, ty) in &args {
        let cty = g.cty(ty);
        inner.push(Decl::Func(Func {
            doc: vec![format!("`event.{arg}`: every rule of the trace has it.")],
            prefix: vec![],
            ret: cty.clone(),
            name: format!("event_{arg}"),
            params: vec![Param { ty: event.clone().cref(), name: "e".into(), default: None }],
            suffix: vec![],
            body: Some(vec![Stmt::Return(Some(Expr::call(
                "std::visit",
                vec![
                    Expr::Lambda {
                        capture: "",
                        params: vec![(Ty::Auto.cref(), "x".into())],
                        ret: Some(cty.clone()),
                        body: vec![Stmt::IfHas {
                            value: "x".into(),
                            member: ident(arg),
                            then: vec![Stmt::Return(Some(Expr::id("x").member(&ident(arg))))],
                            els: vec![Stmt::Expr(Expr::call(
                                "harness::fail",
                                vec![Expr::str(format!("the event has no {arg}"))],
                            ))],
                        }],
                    },
                    Expr::id("e"),
                ],
            )))]),
        }));
    }
    let mut drawn: Vec<Vec<RuleId>> = Vec::new();
    for l in &laws {
        if drawn.contains(&l.rules) {
            continue;
        }
        drawn.push(l.rules.clone());
        let info = &prog.law(l.id).info;
        inner.push(unit.mark(g.origin(info, "law: the events a trace over these rules draws")));
        inner.push(Decl::Func(draw(g, l, board)?));
    }
    let mut items = vec![Decl::Namespace(String::new(), inner)];
    for l in &laws {
        let info = &prog.law(l.id).info;
        let mut env = test_env(g, info);
        env.bind(&l.trace, Local::Value(Expr::id("t")));
        for p in &l.others {
            env.value(&p.name);
        }
        let check = Stmt::Expr(Expr::call("CHECK", vec![env.expr(&l.body)?.macro_arg()]));
        let trace = Stmt::Var {
            ty: Ty::Auto.konst(),
            name: "t".into(),
            init: Some(Expr::CallT(
                Box::new(Expr::id("rt::test::trace")),
                vec![world.clone(), event.clone()],
                vec![
                    Expr::id("seed"),
                    Expr::call("start", vec![Expr::id("seed")]),
                    Expr::id("steps"),
                    Expr::id(draw_name(g, l)),
                    Expr::id("refused"),
                ],
            )),
        };
        let mut per = vec![trace];
        if l.others.is_empty() {
            per.push(check);
        } else {
            per.push(Stmt::Var {
                ty: Ty::name("rt::random::Prng"),
                name: "prng".into(),
                init: Some(Expr::call("rt::random::Prng", vec![Expr::bin("+", Expr::id("seed"), Expr::Int(0x5EED))])),
            });
            let mut each: Vec<Stmt> = l
                .others
                .iter()
                .map(|p: &TParam| Stmt::Var {
                    ty: Ty::Auto.konst(),
                    name: ident(&p.name),
                    init: Some(Expr::call(format!("support::{}", gen_name(g, &p.ty)), vec![prng()])),
                })
                .collect();
            each.push(check);
            per.push(Stmt::Count {
                name: "q".into(),
                ty: Ty::name("int"),
                from: Expr::Int(0),
                below: Expr::Int(8),
                body: each,
            });
        }
        let body = vec![Stmt::Count {
            name: "seed".into(),
            ty: Ty::name("std::uint64_t"),
            from: Expr::Int(0),
            below: Expr::id("traces"),
            body: per,
        }];
        let mut doc = String::new();
        if let Some(why) = &info.why {
            doc = format!("// {why}.\n");
        }
        items.push(unit.mark(g.origin(info, "law: checked on every trace")));
        if !doc.is_empty() {
            items.push(Decl::Line(doc.trim_end().to_string()));
        }
        items.push(test_decl(&info.name, body));
    }
    unit.decls.push(Decl::Namespace(g.ns.clone(), items));
    Ok(unit)
}

fn find_event_args(e: &TExpr, out: &mut BTreeSet<(String, T)>) {
    if let K::EventArg { name, .. } = &e.kind {
        out.insert((name.clone(), e.ty.clone()));
    }
    spec_lang_emit_rust::front::facts::children(e, &mut |c| find_event_args(c, out));
}

/// `draw_<rule set>`: named by the `type X = a | b` that lists the trace's rules, else by its rules.
fn draw_name(g: &Gen<'_, '_>, l: &TLaw) -> String {
    let prog = g.f.prog;
    let named = prog.types.iter().find(|t| matches!(&t.def, TypeDef::Rules(rs) if *rs == l.rules));
    match named {
        Some(t) => format!("draw_{}", snake(&t.info.name)),
        None => {
            format!("draw_{}", l.rules.iter().map(|r| prog.rule(*r).info.name.clone()).collect::<Vec<_>>().join("_or_"))
        }
    }
}

/// `draw_<rule set>(prng)`: one event of the trace's rules, its parameters generated.
fn draw(g: &Gen<'_, '_>, l: &TLaw, board: &str) -> R<Func> {
    let prog = g.f.prog;
    let info = &prog.law(l.id).info;
    let mut cases = Vec::new();
    for (i, r) in l.rules.iter().enumerate() {
        let rule: RuleId = *r;
        let Some(tr) = g.f.rule(rule) else {
            return Err(g.unsupported(info.span, info, "a trace over a rule outside the app"));
        };
        let rinfo = &prog.rule(rule).info;
        let ty = Ty::name(format!("{board}::{}", camel(&rinfo.name)));
        let fields: Vec<(String, Expr)> = tr
            .params
            .iter()
            .map(|p| (ident(&p.name), Expr::call(format!("support::{}", gen_name(g, &p.ty)), vec![prng()])))
            .collect();
        let v = if fields.is_empty() { Expr::Brace(Some(ty), vec![]) } else { Expr::Designated(ty, fields) };
        cases.push((Expr::Int(i as i128), vec![Stmt::Return(Some(v))]));
    }
    Ok(Func {
        doc: vec![format!(
            "An event of a trace over {}: each rule evenly.",
            l.rules.iter().map(|r| prog.rule(*r).info.name.clone()).collect::<Vec<_>>().join(" | ")
        )],
        prefix: vec![],
        ret: Ty::name(format!("{board}::Event")),
        name: draw_name(g, l),
        params: vec![Param { ty: Ty::name("rt::random::Prng").mref(), name: "prng".into(), default: None }],
        suffix: vec![],
        body: Some(vec![
            Stmt::Switch { on: prng().method("below", vec![Expr::Int(l.rules.len() as i128)]), cases },
            Stmt::Expr(Expr::call("std::unreachable", vec![])),
        ]),
    })
}
