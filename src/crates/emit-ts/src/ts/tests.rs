//! The generated tests: every `@cases` row, every `test` by name, every `law`
//! on seeded random traces, and the help text.

use super::build as b;
use super::lower::*;
use super::product::{APP, Gen};
use crate::sem::app::{Given, Law, Test};
use crate::sem::infer::{Named, R};
use crate::sem::names::{camel, kebab};
use crate::sem::tree::*;
use swc_common::DUMMY_SP;
use swc_ecma_ast::{Stmt, TsKeywordTypeKind};

const BUN_TEST: &str = "bun:test";
const ARB: &str = "test/support/arb.ts";
const BOARD: &str = "test/support/board.ts";
const TRACE: &str = "test/support/trace.ts";
const EVENTS: &str = "test/support/events.ts";

/// Where the steps of a module's tests live: importable, unlike a test file.
fn steps_file(app: &crate::sem::app::App<'_>, m: spec_lang_ir::ModuleId) -> String {
    format!("test/{}.steps.ts", module_dir(app, m))
}

/// `test("name", () => { body });`
fn test_call(lo: &mut Lower<'_, '_>, name: &str, body: Vec<Stmt>) -> Stmt {
    let test = lo.f.value(BUN_TEST, "test");
    b::expr_stmt(b::call(test, vec![b::text(name), b::arrow_block(vec![], body)]))
}

/// An arbitrary for a type: values are generated from the type, as the card says.
fn arb(lo: &mut Lower<'_, '_>, t: &Type) -> R<b::E> {
    Ok(match t {
        Type::Int => lo.f.value(ARB, "intArb"),
        Type::Text => lo.f.value(ARB, "textArb"),
        Type::Bool => b::method(lo.f.default("fast-check", "fc"), "boolean", vec![]),
        Type::Set(inner) => {
            let of = arb(lo, inner)?;
            b::call(lo.f.value(ARB, "setArb"), vec![of])
        }
        Type::Option(inner) => {
            let of = arb(lo, inner)?;
            b::call(lo.f.value(ARB, "optionArb"), vec![of])
        }
        Type::Named(id) => match lo.inf.named(*id)? {
            Named::Sum(_) => {
                let variants = lo.variants_ref(*id);
                b::call_spread(b::member(lo.f.default("fast-check", "fc"), "constantFrom"), variants)
            }
            Named::Alias { base, claim: Some(_) } => {
                let of = arb(lo, &base)?;
                let claimed = lo.claimed_ref(*id);
                b::call(lo.f.value(ARB, "claimedArb"), vec![of, claimed])
            }
            Named::Alias { base, claim: None } => arb(lo, &base)?,
            Named::Record(_) => {
                return lo.inf.unsupported(lo.app.p.ty(*id).info.span, "generating values of a record type");
            }
        },
        _ => return lo.inf.unsupported(lo.app.decl.info.span, format!("generating values of {}", lo.inf.show(t))),
    })
}

fn expect(lo: &mut Lower<'_, '_>, got: b::E, how: &str, want: b::E) -> Stmt {
    let e = lo.f.value(BUN_TEST, "expect");
    b::expr_stmt(b::method(b::call(e, vec![got]), how, vec![want]))
}

impl Gen<'_, '_> {
    pub fn tests(&mut self) -> R<()> {
        let app = self.app;
        self.cases()?;
        for t in &app.tests {
            self.scenario(t)?;
        }
        if !app.laws.is_empty() {
            self.events()?;
        }
        for l in &app.laws {
            self.law(l)?;
        }
        self.help()
    }

    fn cases(&mut self) -> R<()> {
        let app = self.app;
        for func in app.fns.iter().filter(|f| !f.cases.is_empty()) {
            let info = &app.p.func(func.id).info;
            let path = format!("test/{}/{}.cases.test.ts", module_dir(app, info.module), kebab(&info.name));
            for (i, case) in func.cases.iter().enumerate() {
                let entry = self.entry(
                    info,
                    case.span,
                    &format!("case[{i}]"),
                    "case.test: one test per `@cases` row",
                    "the row as written",
                );
                let f = self.file(&path, "@cases of fn", info);
                let mut lo = Lower::new(app, f, Mode::Pure, &app.p.qualified(info));
                let args = case.args.iter().map(|a| match a {
                    Some(a) => lo.expr(a),
                    None => Ok(b::undefined()),
                });
                let args = args.collect::<R<Vec<_>>>()?;
                let got = b::call(lo.fn_ref(func.id), args);
                let want = lo.expr(&case.expect)?;
                let check = expect(&mut lo, got, "toEqual", want);
                let name = format!(
                    "{} @cases[{i}]: {}",
                    info.name,
                    app.p.snippet(case.span).split_whitespace().collect::<Vec<_>>().join(" ")
                );
                let stmt = test_call(&mut lo, &name, vec![check]);
                let at = f.anchor(entry, None);
                f.items.push(b::module_stmt(stmt, at));
            }
        }
        Ok(())
    }

    fn scenario(&mut self, t: &Test) -> R<()> {
        let app = self.app;
        let decl = app.p.test(t.id);
        let info = &decl.info;
        let path = format!("test/{}.test.ts", module_dir(app, info.module));
        let qualified = app.p.qualified(info);
        let kind = if t.params.is_empty() {
            "test.scenario: rules are called directly on a test board; no mocks"
        } else {
            "test.property: the parameters are generated from their types, seeded"
        };
        let top = self.entry(info, info.span, "test", kind, "the test as written");
        let given_entries: Vec<_> = t
            .given
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let span = match g {
                    Given::State { span, .. } | Given::Test { span, .. } => *span,
                };
                self.entry(
                    info,
                    span,
                    &format!("given[{i}]"),
                    "given.install: the start world is installed directly",
                    "`@given`",
                )
            })
            .collect();
        let stmt_entries: Vec<_> = t
            .body
            .iter()
            .enumerate()
            .map(|(i, s)| self.entry(info, s.span, &format!("body.stmt[{i}]"), "stmt.direct", "the step as written"))
            .collect();
        let run_entry = self.entry(
            info,
            info.name_span,
            "test.run",
            "test.named: registered under the spec's own name",
            "the test's name",
        );

        let steps = steps_file(app, info.module);
        let f = self.file(&steps, "test", info);
        let top_at = f.anchor(top, info.why.as_deref());
        let mut lo = Lower::new(app, f, Mode::Test, &qualified);
        let world = lo.f.type_ref(WORLD, "World", vec![]);
        let mut params = vec![b::param("b", lo.f.type_ref(BOARD, "TestBoard", vec![world]))];
        for p in &t.params {
            let ty = lo.ty(&p.ty)?;
            params.push(b::param(&lo.local(&p.name), ty));
        }
        let mut body = Vec::new();
        for (g, entry) in t.given.iter().zip(given_entries) {
            let stmts = match g {
                Given::State { state, value, .. } => {
                    let v = lo.expr(value)?;
                    let world = b::member(b::name("b"), "world");
                    vec![b::assign_member(
                        b::name("b"),
                        "world",
                        b::object_spread(world, vec![(state_name(app, *state), v)]),
                    )]
                }
                Given::Test { test, .. } => {
                    let other = &app.p.test(*test).info;
                    let step = lo.f.value(&steps_file(app, other.module), &camel(&other.name));
                    vec![
                        b::expr_stmt(b::call(step, vec![b::name("b")])),
                        b::expr_stmt(b::method(b::name("b"), "forgetOutput", vec![])),
                    ]
                }
            };
            let at = lo.f.anchor(entry, None);
            let mut stmts = stmts.into_iter();
            if let Some(first) = stmts.next() {
                body.push(b::at(first, at));
            }
            body.extend(stmts);
        }
        for (s, entry) in t.body.iter().zip(stmt_entries) {
            let stmt = lo.stmt(s)?;
            let at = lo.f.anchor(entry, None);
            body.push(b::at(stmt, at));
        }
        let fn_name = camel(&info.name);
        let function = b::function(&fn_name, params, Some(b::keyword(TsKeywordTypeKind::TsVoidKeyword)), body);

        f.items.push(b::export(function, top_at));

        // The steps live outside the test file, so another module's test can start from them.
        let f = self.file(&path, "test", info);
        let mut lo = Lower::new(app, f, Mode::Test, &qualified);
        let step = lo.f.value(&steps, &fn_name);
        let board = b::new("TestBoard", vec![], vec![lo.f.value(WORLD, "EMPTY_WORLD")]);
        lo.f.value(BOARD, "TestBoard");
        let run = if t.params.is_empty() {
            b::expr_stmt(b::call(step, vec![board]))
        } else {
            let mut arbs = Vec::new();
            let mut names = Vec::new();
            for p in &t.params {
                arbs.push(arb(&mut lo, &p.ty)?);
                names.push(lo.local(&p.name));
            }
            let mut call_args = vec![board];
            call_args.extend(names.iter().map(|n| b::name(n)));
            let check = b::arrow_block(
                names.iter().map(|n| b::bare(n)).collect(),
                vec![b::expr_stmt(b::call(step, call_args))],
            );
            let fc = lo.f.default("fast-check", "fc");
            let fc2 = lo.f.default("fast-check", "fc");
            let mut property_args = arbs;
            property_args.push(check);
            let runs = lo.f.value(ARB, "RUNS");
            b::expr_stmt(b::method(fc, "assert", vec![b::method(fc2, "property", property_args), runs]))
        };
        let registered = test_call(&mut lo, &info.name, vec![run]);
        let at = f.anchor(run_entry, None);
        f.items.push(b::module_stmt(registered, at));
        Ok(())
    }

    /// The events of every rule of the app: their type, how each is generated, and how one is applied.
    fn events(&mut self) -> R<()> {
        let app = self.app;
        let info = &app.decl.info;
        let entry = self.entry(
            info,
            info.span,
            "trace events",
            "trace.events: one event type per rule, generated from the parameter types; `apply` calls the rule",
            "the parameters of the app's rules",
        );
        let f = self.file(EVENTS, "app", info);
        let mut lo = Lower::new(app, f, Mode::Pure, &app.p.qualified(info));
        let mut variants = Vec::new();
        let mut arbs = Vec::new();
        let mut cases = Vec::new();
        for r in &app.rules {
            let mut fields = vec![("rule".to_string(), b::t_str(&r.name))];
            let fc = lo.f.default("fast-check", "fc");
            let mut record = vec![("rule".to_string(), b::method(fc, "constant", vec![b::as_const(b::text(&r.name))]))];
            let mut args = vec![b::name("w")];
            for p in &r.params {
                if p.name == "rule" {
                    return lo.inf.unsupported(p.span, "a rule parameter named `rule` in a trace");
                }
                fields.push((camel(&p.name), lo.ty(&p.ty)?));
                record.push((camel(&p.name), arb(&mut lo, &p.ty)?));
                args.push(b::member(b::name("e"), &camel(&p.name)));
            }
            variants.push(b::t_object(fields));
            let fc = lo.f.default("fast-check", "fc");
            arbs.push((camel(&r.name), b::method(fc, "record", vec![b::object(record)])));
            cases.push((b::text(&r.name), b::call(lo.rule_ref(r.id), args)));
        }
        let at = f.anchor(entry, None);
        f.items.push(b::export(b::type_alias("Event", vec![], b::t_union(variants)), at));
        let names: Vec<String> = arbs.iter().map(|(n, _)| n.clone()).collect();
        f.items.push(b::export(b::const_decl("eventArbs", None, b::object(arbs)), DUMMY_SP));
        let fc = f.default("fast-check", "fc");
        let any = b::method(fc, "oneof", names.iter().map(|n| b::member(b::name("eventArbs"), n)).collect());
        let arb_ty = b::t_ref("fc.Arbitrary", vec![b::t_ref("Event", vec![])]);
        f.items.push(b::export(b::const_decl("anyEvent", Some(arb_ty), any), DUMMY_SP));
        let world = f.type_ref(WORLD, "World", vec![]);
        let outcome = f.type_ref(OUTCOME, "Outcome", vec![b::keyword(TsKeywordTypeKind::TsUnknownKeyword)]);
        let apply = b::function(
            "apply",
            vec![b::param("w", world), b::param("e", b::t_ref("Event", vec![]))],
            Some(outcome),
            vec![b::switch_return(b::member(b::name("e"), "rule"), cases)],
        );
        f.items.push(b::export(apply, DUMMY_SP));
        Ok(())
    }

    fn law(&mut self, law: &Law) -> R<()> {
        let app = self.app;
        let info = &app.p.law(law.id).info;
        let path = format!("test/{}.test.ts", module_dir(app, info.module));
        let qualified = app.p.qualified(info);
        let rule_names: Vec<String> = law.rules.iter().map(|r| app.p.rule(*r).info.name.clone()).collect();
        let top =
            self.entry(info, info.span, "law", "law.predicate: the law as a function of a trace", "the law as written");
        let run_entry = self.entry(
            info,
            info.name_span,
            "law.run",
            "law.sampled: seeded random traces of the trace's rules, each from a world the app's rules can reach; the law is evaluated on the whole trace",
            &format!("Trace[{}]", rule_names.join(" | ")),
        );
        let f = self.file(&path, "law", info);
        let top_at = f.anchor(top, info.why.as_deref());
        let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
        let world = lo.f.type_ref(WORLD, "World", vec![]);
        let event = lo.f.type_ref(EVENTS, "Event", vec![]);
        let only =
            b::t_object(vec![("rule".to_string(), b::t_union(rule_names.iter().map(|n| b::t_str(n)).collect()))]);
        let trace_ty = lo.f.type_ref(TRACE, "Trace", vec![world, b::t_ref("Extract", vec![event, only])]);
        let trace = lo.local(&law.trace);
        let mut params = vec![b::param(&trace, trace_ty)];
        let mut arbs = Vec::new();
        let mut names = Vec::new();
        for p in &law.params {
            let ty = lo.ty(&p.ty)?;
            params.push(b::param(&lo.local(&p.name), ty));
            arbs.push(arb(&mut lo, &p.ty)?);
            names.push(lo.local(&p.name));
        }
        let fn_name = camel(&info.name);
        let body = vec![b::ret(Some(lo.expr(&law.body)?))];
        let function = b::function(&fn_name, params, Some(b::keyword(TsKeywordTypeKind::TsBooleanKeyword)), body);

        let fc = |lo: &mut Lower<'_, '_>| lo.f.default("fast-check", "fc");
        let any = lo.f.value(EVENTS, "anyEvent");
        let prefix_max = lo.f.value(ARB, "PREFIX_MAX");
        let prefix = b::method(fc(&mut lo), "array", vec![any, b::object(vec![("maxLength".into(), prefix_max)])]);
        let arbs_obj = lo.f.value(EVENTS, "eventArbs");
        let of_rules = b::method(
            fc(&mut lo),
            "oneof",
            law.rules.iter().map(|r| b::member(arbs_obj.clone(), &camel(&app.p.rule(*r).info.name))).collect(),
        );
        let (len, len2) = (lo.f.value(ARB, "TRACE_LENGTH"), lo.f.value(ARB, "TRACE_LENGTH"));
        let events = b::method(
            fc(&mut lo),
            "array",
            vec![of_rules, b::object(vec![("minLength".into(), len), ("maxLength".into(), len2)])],
        );
        let apply = lo.f.value(EVENTS, "apply");
        let start = b::call(
            lo.f.value(TRACE, "reach"),
            vec![lo.f.value(WORLD, "EMPTY_WORLD"), b::name("prefix"), apply.clone()],
        );
        let run_trace = b::call(lo.f.value(TRACE, "runTrace"), vec![start, b::name("events"), apply]);
        let mut law_args = vec![b::name(&trace)];
        law_args.extend(names.iter().map(|n| b::name(n)));
        let holds = expect(&mut lo, b::call(b::name(&fn_name), law_args), "toBe", b::boolean(true));
        let mut check_params = vec![b::bare("prefix"), b::bare("events")];
        check_params.extend(names.iter().map(|n| b::bare(n)));
        let check = b::arrow_block(check_params, vec![b::const_(&trace, None, run_trace), holds]);
        let mut property_args = vec![prefix, events];
        property_args.extend(arbs);
        property_args.push(check);
        let runs = lo.f.value(ARB, "RUNS");
        let property = b::method(fc(&mut lo), "property", property_args);
        let assert = b::expr_stmt(b::method(fc(&mut lo), "assert", vec![property, runs]));
        let registered = test_call(&mut lo, &info.name, vec![assert]);
        f.items.push(b::local(function, top_at));
        let at = f.anchor(run_entry, None);
        f.items.push(b::module_stmt(registered, at));
        Ok(())
    }

    fn help(&mut self) -> R<()> {
        let app = self.app;
        let info = &app.decl.info;
        let entry = self.entry(
            info,
            info.span,
            "help test",
            "help.pinned: the text the command table gives at load equals the text the compiler derived",
            "one `--help` line per cli rule, in the app's order",
        );
        let f = self.file("test/app/help.test.ts", "app", info);
        let mut lo = Lower::new(app, f, Mode::Pure, &app.p.qualified(info));
        let got = b::member(lo.f.value(APP, "APP"), "help");
        let check = expect(&mut lo, got, "toBe", b::text(&app.help));
        let stmt = test_call(&mut lo, "--help is the text the cli vocabulary derives", vec![check]);
        let at = f.anchor(entry, None);
        f.items.push(b::module_stmt(stmt, at));
        Ok(())
    }
}
