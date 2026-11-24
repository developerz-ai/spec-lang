//! `tests/checks.cpp`: every check of the program as the suite runs it, with
//! the generators its properties and laws draw from.

use crate::decls::{ModuleDecls, Unit, banner, origin};
use crate::emit::{Fx, Gen, Ret};
use crate::names::{flat, ident, path, string_literal};
use crate::source::Lines;
use spec_lang_ir::{StateId, TestId};
use spec_lang_lir::node::{Expected, Expr, ExprKind};
use spec_lang_lir::{CaseTarget, CheckWhat, DTy, Given, LTy, TypeDef};
use std::fmt::Write;

/// The generators of every declared type some check draws, numbered as they are first asked for.
struct Gens<'g, 'a> {
    g: &'g Gen<'a>,
    types: Vec<DTy>,
    /// The types whose values a world keeps: keys of `Map` states and elements of `Set` states.
    pools: Vec<DTy>,
}

impl<'g, 'a> Gens<'g, 'a> {
    fn new(g: &'g Gen<'a>) -> Self {
        let mut pools: Vec<DTy> = Vec::new();
        for s in &g.lir.states {
            if let DTy::Map(k, _) | DTy::Set(k) = &s.check
                && !pools.contains(k)
            {
                pools.push((**k).clone());
            }
        }
        Gens { g, types: Vec::new(), pools }
    }

    /// The call that draws one value of `d`.
    fn call(&mut self, d: &DTy) -> String {
        let i = match self.types.iter().position(|t| t == d) {
            Some(i) => i,
            None => {
                self.types.push(d.clone());
                self.types.len() - 1
            }
        };
        format!("draw_{i}(g)")
    }

    /// A fresh base value of `d`, as the statements of a function that returns it.
    fn base(&mut self, d: &DTy) -> String {
        let g = self.g;
        let ty = g.dty(d);
        match d {
            DTy::Unit => "return rt::unit;".into(),
            DTy::Bool => "return g.rng.chance(1, 2);".into(),
            DTy::Int => "return rt::test::draw_int(g.rng);".into(),
            DTy::Text => "return rt::test::draw_text(g.rng);".into(),
            DTy::Range { lo, hi, inclusive } => {
                let hi = if *inclusive { *hi } else { hi - 1 };
                format!(
                    "return rt::test::draw_range(g.rng, {}, {});",
                    crate::emit::int_literal(*lo).s,
                    crate::emit::int_literal(hi).s
                )
            }
            DTy::Option(inner) => format!("if (g.rng.chance(1, 3)) {{\nreturn {{}};\n}}\nreturn {ty}{{{}}};", self.call(inner)),
            DTy::List(t) => format!("return rt::test::draw_list<{}>(g.rng, [&] {{ return {}; }});", g.dty(t), self.call(t)),
            DTy::Set(t) => format!("return rt::test::draw_set<{}>(g.rng, [&] {{ return {}; }});", g.dty(t), self.call(t)),
            DTy::Map(k, v) => format!(
                "return rt::test::draw_map<{}, {}>(g.rng, [&] {{ return {}; }}, [&] {{ return {}; }});",
                g.dty(k),
                g.dty(v),
                self.call(k),
                self.call(v)
            ),
            // A braced list is evaluated in order.
            DTy::Tuple(ts) => {
                let items: Vec<String> = ts.iter().map(|t| self.call(t)).collect();
                format!("return {ty}{{{}}};", items.join(", "))
            }
            DTy::Where(b, _) => format!("return {};", self.call(b)),
            DTy::Named(t) => match &g.lir.ty(*t).def {
                TypeDef::Alias { of } => format!("return {};", self.call(of)),
                TypeDef::Record { fields } => {
                    let items: Vec<String> = fields.iter().map(|f| self.call(&f.check)).collect();
                    format!("return {ty}{{{}}};", items.join(", "))
                }
                TypeDef::Sum { variants } if g.is_enum(*t) => {
                    format!("return static_cast<{ty}>(g.rng.below({}));", variants.len())
                }
                TypeDef::Sum { variants } if !g.lir.ty(*t).builtin => {
                    let mut s = format!("switch (g.rng.below({})) {{\n", variants.len());
                    for (i, v) in variants.iter().enumerate() {
                        let items: Vec<String> = v.fields.iter().map(|f| self.call(&f.check)).collect();
                        let label = if i + 1 == variants.len() { "default".to_string() } else { format!("case {i}") };
                        let _ = writeln!(s, "{label}:\nreturn {ty}{{{}{{{}}}}};", g.variant_name(*t, i as u32), items.join(", "));
                    }
                    s + "}"
                }
                TypeDef::Sum { .. } => "rt::stop(\"E-GEN\", \"no value of a built-in sum can be drawn\");".into(),
                TypeDef::RuleSet { .. } => "rt::stop(\"E-GEN\", \"no value of a rule set can be drawn\");".into(),
            },
            DTy::Trace(_) => "rt::stop(\"E-GEN\", \"no value of `Trace` can be drawn\");".into(),
        }
    }

    /// Every generator asked for so far, and those they ask for in turn.
    fn print(&mut self) -> String {
        let g = self.g;
        let mut bodies = Vec::new();
        let mut i = 0;
        while i < self.types.len() {
            let d = self.types[i].clone();
            let ty = g.dty(&d);
            let base = self.base(&d);
            let pool = self.pools.iter().position(|p| *p == d);
            let mut fx = Fx::new(g, None, crate::emit::no_span());
            let conform = fx.conform(&d, "value");
            let body = if pool.is_none() && conform.is_none() {
                base
            } else {
                let pool = pool.map_or("nullptr".to_string(), |p| format!("&g.pools.pool_{p}"));
                let name = string_literal(&ty);
                let ok = conform.map_or("[](const auto&) { return true; }".to_string(), |c| {
                    format!("[&](const {ty}& value) {{ return {c}; }}")
                });
                format!("return rt::test::keep<{ty}>(g.rng, {pool}, [&]() -> {ty} {{\n{base}\n}}, {ok}, {name});")
            };
            bodies.push(format!("{ty} draw_{i}([[maybe_unused]] Gen& g) {{\n{body}\n}}\n"));
            i += 1;
        }
        let mut out = String::from("// One generator per declared type a property or a law draws: the engine's algorithm, step for step.\n");
        for (i, d) in self.types.iter().enumerate() {
            let _ = writeln!(out, "{} draw_{i}(Gen& g);", g.dty(d));
        }
        out.push('\n');
        out + &bodies.join("\n")
    }

    fn pools_struct(&self) -> String {
        let g = self.g;
        let mut s = String::from(
            "// Values already in a world, by declared type: the keys of each `Map` state and the elements of each `Set` state.\nstruct Pools {\n",
        );
        for (i, p) in self.pools.iter().enumerate() {
            let _ = writeln!(s, "rt::Set<{}> pool_{i};", g.dty(p));
        }
        s.push_str("void add([[maybe_unused]] const World& world) {\n");
        for (i, st) in g.lir.states.iter().enumerate() {
            let field = g.state_field(StateId(i as u32));
            match &st.check {
                DTy::Map(k, _) => {
                    let p = self.pools.iter().position(|x| x == &**k).unwrap_or(0);
                    let _ = writeln!(s, "for (const auto& entry : world.{field}) {{\npool_{p}.insert(entry.first);\n}}");
                }
                DTy::Set(k) => {
                    let p = self.pools.iter().position(|x| x == &**k).unwrap_or(0);
                    let _ = writeln!(s, "pool_{p}.insert(world.{field}.begin(), world.{field}.end());");
                }
                _ => {}
            }
        }
        s + "}\n};\n"
    }
}

fn test_fn(g: &Gen<'_>, t: TestId) -> String {
    flat(&g.lir.test(t).info.qualified)
}

/// Whether following `@given(test)` from `t` comes back to it.
fn given_cycle(g: &Gen<'_>, t: TestId) -> bool {
    let mut seen = vec![t];
    let mut next = vec![t];
    while let Some(cur) = next.pop() {
        for item in &g.lir.test(cur).given {
            if let Given::Test(p) = item {
                if *p == t {
                    return true;
                }
                if !seen.contains(p) {
                    seen.push(*p);
                    next.push(*p);
                }
            }
        }
    }
    false
}

/// A value bound to a name, standing where an argument would.
fn local(name: &str, like: &Expr) -> Expr {
    Expr::new(ExprKind::Local(name.to_string()), like.ty.clone(), like.span)
}

/// `tests/checks.cpp`.
pub fn checks(g: &Gen<'_>, lines: &Lines) -> Unit {
    let lir = g.lir;
    let mut gens = Gens::new(g);
    let mut tests = String::new();
    let mut main = String::new();

    // Each test: its `@given` world, its steps, and one run of both.
    for (i, t) in lir.tests.iter().enumerate() {
        let id = TestId(i as u32);
        let name = test_fn(g, id);
        tests.push_str(&origin(g, lines, "test", &t.info));
        let mut fx = Fx::new(g, Some("ctx"), t.info.span);
        let mut given = String::new();
        if given_cycle(g, id) {
            given.push_str("rt::stop(\"E-TYPE-MISMATCH\", \"`@given` tests form a cycle\");\n");
        }
        for item in &t.given {
            match item {
                Given::State { state, value } => {
                    let s = lir.state(*state);
                    let field = g.state_field(*state);
                    let v = fx.fresh("value");
                    let _ = writeln!(given, "{} {v} = {};", g.dty(&s.check), fx.expr_typed(value));
                    if let Some(c) = fx.conform(&s.check, &v) {
                        let _ = writeln!(given, "rt::claim({c}, {});", string_literal(&format!("this `@given` value of {}", s.info.qualified)));
                    }
                    let _ = writeln!(given, "ctx.world.{field} = std::move({v});");
                }
                Given::Test(parent) => {
                    let p = lir.test(*parent);
                    if !p.params.is_empty() {
                        given.push_str("rt::stop(\"E-TYPE-MISMATCH\", \"`@given` names a test with parameters; it has no one end world\");\n");
                    } else if !given_cycle(g, id) {
                        let _ = writeln!(given, "ctx.world = suite.end_world({}, {}, run_{});", parent.0, string_literal(&p.info.qualified), test_fn(g, *parent));
                    }
                }
            }
        }
        let _ = writeln!(tests, "{}", fx.resolve(&format!("void given_{name}([[maybe_unused]] Suite& suite, [[maybe_unused]] Machine& ctx) {{\n{given}}}")));
        let mut fx = Fx::new(g, Some("ctx"), t.info.span);
        fx.reserve("suite");
        let mut params = vec!["[[maybe_unused]] Machine& ctx".to_string()];
        let mut run_params = vec!["Suite& suite".to_string()];
        let mut names = Vec::new();
        for p in &t.params {
            let cpp = fx.declare(&p.name);
            params.push(format!("{}{} {cpp}", fx.mark(&cpp), g.param_dty(&p.ty, &p.check)));
            run_params.push(format!("{} {cpp}", g.param_dty(&p.ty, &p.check)));
            names.push(cpp);
        }
        let mut body = String::new();
        fx.block(&t.body, &mut body, Ret::Void);
        let _ = writeln!(tests, "{}", fx.resolve(&format!("void steps_{name}({}) {{\n{body}}}", params.join(", "))));
        let mut args = vec!["ctx".to_string()];
        args.extend(names);
        let _ = writeln!(tests, "Run run_{name}({}) {{\nreturn suite.run(given_{name}, [&](Machine& ctx) {{ steps_{name}({}); }});\n}}\n", run_params.join(", "), args.join(", "));
    }

    // Laws and the rules their traces call.
    let mut laws = String::new();
    let mut driven: Vec<spec_lang_ir::RuleId> = Vec::new();
    for l in &lir.laws {
        let mut fx = Fx::new(g, Some("ctx"), l.info.span);
        let mut draws = String::new();
        let mut trace = String::from("trace");
        for (i, p) in l.params.iter().enumerate() {
            let cpp = fx.declare(&p.name);
            if i == l.trace {
                trace = cpp;
            } else {
                let _ = writeln!(draws, "{}const auto {cpp} = {};", fx.mark(&cpp), gens.call(&p.check));
            }
        }
        let mut body = String::new();
        fx.tail(&l.body, &mut body, Ret::Value);
        let ids = |rs: &[spec_lang_ir::RuleId]| rs.iter().map(|r| g.rule_const(*r)).collect::<Vec<_>>().join(", ");
        laws.push_str(&origin(g, lines, "law", &l.info).replace("// ", "    // "));
        let name = string_literal(&l.info.qualified);
        let text = format!("suite.laws.push_back({{{name}, {name}, []([[maybe_unused]] Gen& g) -> Suite::Bound {{\n{draws}return [=]([[maybe_unused]] Machine& ctx, [[maybe_unused]] const rt::Trace<World>& {trace}) -> bool {{\n{body}}};\n}}, {{{}}}, {{{}}}}});", ids(&l.traced), ids(&l.start_rules));
        let _ = writeln!(laws, "{}", fx.resolve(&text));
        for r in l.traced.iter().chain(&l.start_rules) {
            if !driven.contains(r) {
                driven.push(*r);
            }
        }
    }
    driven.sort();
    let mut drivers = String::new();
    for r in &driven {
        let d = lir.rule(*r);
        let draws: Vec<String> = d.params.iter().map(|p| format!("rt::Dyn{{{}}}", gens.call(&p.check))).collect();
        let args: Vec<String> = std::iter::once("ctx".to_string())
            .chain(d.params.iter().enumerate().map(|(i, p)| format!("e.args.at({i}).as<{}>()", g.ty(&p.ty))))
            .collect();
        let id = g.rule_const(*r);
        let give = if d.result == LTy::Unit { "" } else { "return " };
        // A braced list is evaluated in order: the arguments are drawn left to right.
        let _ = writeln!(drivers, "suite.rules[{id}] = {{[]([[maybe_unused]] Gen& g) {{ return rt::Event{{{id}, {{{}}}}}; }},\n[](Machine& ctx, [[maybe_unused]] const rt::Event& e) {{ return rt::reply_of([&] {{ {give}{}({}); }}); }}}};", draws.join(", "), g.rule_name(*r), args.join(", "));
    }
    let mut world = String::from("suite.draw_world = []([[maybe_unused]] Gen& g) {\nWorld world;\n");
    for (i, st) in lir.states.iter().enumerate() {
        let _ = writeln!(world, "world.{} = {};", g.state_field(StateId(i as u32)), gens.call(&st.check));
    }
    world.push_str("return world;\n};\n");
    let mut rule_names = String::from("rt::name_rules({\n");
    for r in &lir.rules {
        let module = lir.modules[r.info.module.index()].name.rsplit('.').next().unwrap_or("").to_string();
        let ps: Vec<String> = r.params.iter().map(|p| string_literal(&p.name)).collect();
        let _ = writeln!(rule_names, "{{{}, {}, {{{}}}}},", string_literal(&module), string_literal(&r.info.name), ps.join(", "));
    }
    rule_names.push_str("});\n");

    // The checks, in the order and with the names `spec-lang test` uses.
    let mut law_index = 0;
    for c in &lir.checks {
        let name = string_literal(&c.name);
        match &c.what {
            CheckWhat::Test(t) => {
                let _ = writeln!(main, "suite.test({name}, {}, run_{});", t.0, test_fn(g, *t));
            }
            CheckWhat::Property(t) => {
                let test = lir.test(*t);
                let mut body = String::new();
                let mut args = vec!["suite".to_string()];
                for (i, p) in test.params.iter().enumerate() {
                    let _ = writeln!(body, "const auto arg_{i} = {};", gens.call(&p.check));
                    args.push(format!("arg_{i}"));
                }
                let _ = writeln!(main, "suite.property({name}, {}, [](Suite& suite, [[maybe_unused]] Gen& g) {{\n{body}return run_{}({});\n}});", string_literal(&test.info.qualified), test_fn(g, *t), args.join(", "));
            }
            CheckWhat::Law(_) => {
                let _ = writeln!(main, "suite.law({law_index});");
                law_index += 1;
            }
            CheckWhat::Case { target, args, expect } => {
                let mut fx = Fx::new(g, Some("ctx"), crate::emit::no_span());
                let mut body = String::new();
                let mut given: Vec<Option<Expr>> = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    given.push(a.as_ref().map(|a| {
                        fx.span = a.span;
                        let n = format!("arg_{i}");
                        let _ = writeln!(body, "const auto {n} = {};", fx.expr(a).at(2));
                        local(&n, a)
                    }));
                }
                match target {
                    CaseTarget::Test(t) => {
                        let mut all = vec!["suite".to_string()];
                        all.extend(given.iter().flatten().map(|a| fx.expr(a).s));
                        let test = test_fn(g, *t);
                        let text = format!("suite.case_test({name}, given_{test}, [](Suite& suite, [[maybe_unused]] Machine& ctx) {{\n{body}return run_{test}({});\n}});", all.join(", "));
                        let _ = writeln!(main, "{}", fx.resolve(&text));
                    }
                    CaseTarget::Fn(f) => {
                        let d = lir.func(*f);
                        let call = Expr::new(ExprKind::CallFn { func: *f, args: given }, d.result.clone(), fx.span);
                        let row = match expect {
                            Expected::Value(x) => {
                                format!("rt::test::gives({}, {})", fx.thunk(&call), fx.thunk(x))
                            }
                            Expected::Fail { name: error, .. } => {
                                let mut stmts = String::new();
                                fx.tail(&call, &mut stmts, Ret::Void);
                                format!("rt::test::refuses([&] {{\n{stmts}}}, {})", string_literal(error))
                            }
                        };
                        let text = format!("suite.case_fn({name}, [] {{\n[[maybe_unused]] Machine ctx;\n{body}return {row};\n}});");
                        let _ = writeln!(main, "{}", fx.resolve(&text));
                    }
                }
            }
        }
    }

    let mut text = banner("every check of the program");
    text.push_str("// Each check as `spec-lang test` runs it; one JSON line per check on standard output.\n#include \"program/machine.hpp\"\n#include \"rt/suite.hpp\"\n");
    for m in &lir.modules {
        if !ModuleDecls::of(g, m.id).is_empty() {
            let _ = writeln!(text, "#include \"{}.hpp\"", path(&m.name));
        }
    }
    let gens_text = gens.print();
    let _ = writeln!(text, "\nnamespace {{\n\nusing program::Machine;\nusing program::World;\n\n{}\nusing Suite = rt::test::Suite<Machine, World, Pools>;\nusing Gen = rt::test::Gen<Machine, Pools>;\nusing Run = rt::test::Run<World>;\n", gens.pools_struct());
    let _ = writeln!(text, "{gens_text}");
    for i in 0..lir.tests.len() {
        let t = &lir.tests[i];
        let mut ps = vec!["Suite& suite".to_string()];
        ps.extend(t.params.iter().map(|p| format!("{} {}", g.param_dty(&p.ty, &p.check), ident(&p.name))));
        let _ = writeln!(text, "Run run_{}({});", test_fn(g, TestId(i as u32)), ps.join(", "));
    }
    let _ = writeln!(text, "\n{tests}}}  // namespace\n\nint main() {{\nSuite suite;\n{rule_names}{world}{drivers}{laws}\n{main}return 0;\n}}");
    Unit { path: "tests/checks.cpp".into(), text }
}
