//! The generated tests: every `@cases` row, every `test` by name, every `law`
//! on seeded random traces, and the help text. Minitest.

use super::ast::*;
use super::lower::*;
use super::product::{CLI, Gen};
use emit_ts::sem::app::{Given, Law, Test};
use emit_ts::sem::infer::{Named, R};
use emit_ts::sem::names::{pascal, snake};
use emit_ts::sem::tree::*;
use spec_lang_ir::{DeclInfo, ModuleId};

const TESTING: &str = "test/support/testing.rb";
const EVENTS: &str = "test/support/events.rb";

/// Where the steps of a module's tests live, and the module that holds them.
fn steps_of(app: &emit_ts::sem::app::App<'_>, m: ModuleId) -> (String, String) {
    let name = &app.p.module(m).name;
    (
        format!("test/{}_steps.rb", name.replace('.', "/")),
        format!("{}Steps", name.split(['.', '_']).map(pascal).collect::<String>()),
    )
}

fn testing(name: &str) -> Expr {
    konst(&format!("Runtime::Testing::{name}"))
}

fn bare(name: &str, args: Vec<Expr>) -> Expr {
    Expr::Call { recv: None, name: name.to_string(), args: args.into_iter().map(Arg::Pos).collect(), block: None }
}

fn block_call(recv: Expr, name: &str, args: Vec<Expr>, params: &[&str], body: Vec<Stmt>) -> Expr {
    call_block(recv, name, args, params, body)
}

/// A generated value of a type: values are generated from the type, as the card says.
fn generate(lo: &mut Lower<'_, '_>, t: &Type) -> R<Expr> {
    let g = || ident("gen");
    Ok(match t {
        Type::Int => call(g(), "int", vec![]),
        Type::Text => call(g(), "text", vec![]),
        Type::Bool => call(g(), "bool", vec![]),
        Type::Set(inner) => {
            let of = generate(lo, inner)?;
            block_call(g(), "set", vec![], &[], vec![Stmt::Expr(of)])
        }
        Type::Option(inner) => {
            let of = generate(lo, inner)?;
            block_call(g(), "option", vec![], &[], vec![Stmt::Expr(of)])
        }
        Type::Named(id) => match lo.inf.named(*id)? {
            Named::Sum(_) => call(g(), "pick", vec![child(lo.type_ref(*id), "ALL")]),
            Named::Alias { base, claim: Some(_) } => {
                let of = generate(lo, &base)?;
                block_call(g(), "claimed", vec![lo.type_ref(*id)], &[], vec![Stmt::Expr(of)])
            }
            Named::Alias { base, claim: None } => generate(lo, &base)?,
            Named::Record(_) => {
                return lo.inf.unsupported(lo.app.p.ty(*id).info.span, "generating values of a record type");
            }
        },
        _ => return lo.inf.unsupported(lo.app.decl.info.span, format!("generating values of {}", lo.inf.show(t))),
    })
}

impl Gen<'_, '_> {
    /// The test file of a module: one Minitest class, named for the module.
    fn test_file(&mut self, path: &str, class_of: &str, kind: &str, info: &DeclInfo) -> &mut super::file::RFile {
        let class = format!("{}Test", class_of.split(['.', '/', '_']).map(pascal).collect::<String>());
        let f = self.file(path, &[], kind, info);
        f.libs.insert("minitest/autorun".into());
        if f.items.is_empty() {
            f.items.push(Stmt::Class { name: class, superclass: Some(konst("Minitest::Test")), body: vec![] });
        }
        f
    }

    fn module_test_path(&self, m: ModuleId) -> (String, String) {
        let name = self.app.p.module(m).name.clone();
        (format!("test/{}_test.rb", name.replace('.', "/")), name)
    }

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
        self.help();
        Ok(())
    }

    fn cases(&mut self) -> R<()> {
        let app = self.app;
        for func in app.fns.iter().filter(|f| !f.cases.is_empty()) {
            let info = &app.p.func(func.id).info;
            let stem = format!("{}/{}_cases", module_dir(app, info.module), snake(&info.name));
            let path = format!("test/{stem}_test.rb");
            for (i, case) in func.cases.iter().enumerate() {
                let entry = self.entry(
                    info,
                    case.span,
                    &format!("case[{i}]"),
                    "case.test: one test per `@cases` row",
                    "the row as written",
                );
                let f = self.test_file(&path, &stem, "@cases of fn", info);
                let mut lo = Lower::new(app, f, Mode::Pure, &app.p.qualified(info));
                let args = case.args.clone();
                let got =
                    lo.expr(&TExpr::new(TKind::CallFn { func: func.id, args }, func.result.clone(), case.span))?;
                let want = lo.expr(&case.expect)?;
                let anchor = f.anchor(entry);
                push_method(
                    f,
                    vec![
                        anchor,
                        Stmt::Def {
                            singleton: false,
                            name: format!("test_case_{i}"),
                            params: vec![],
                            body: vec![Stmt::Expr(assert_equal(want, got))],
                        },
                    ],
                );
            }
        }
        Ok(())
    }

    fn scenario(&mut self, t: &Test) -> R<()> {
        let app = self.app;
        let decl = app.p.test(t.id);
        let info = &decl.info;
        let (path, class_of) = self.module_test_path(info.module);
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

        let (steps, steps_module) = steps_of(app, info.module);
        let f = self.file(&steps, &[], "test", info);
        if f.items.is_empty() {
            f.items.push(Stmt::Module { name: steps_module.clone(), body: vec![] });
        }
        f.need(TESTING);
        f.need(WORLD);
        let top_anchor = f.anchor(top);
        let mut lo = Lower::new(app, f, Mode::Test, &qualified);
        let mut params = vec![Param::Req("board".into())];
        let mut generated = Vec::new();
        let mut names = Vec::new();
        for p in &t.params {
            let n = lo.local(&p.name);
            params.push(Param::Req(n.clone()));
            generated.push(Stmt::Assign(n.clone(), generate(&mut lo, &p.ty)?));
            names.push(ident(&n));
        }
        let mut body = Vec::new();
        let mut uses: Vec<String> = Vec::new();
        for (g, entry) in t.given.iter().zip(given_entries) {
            let stmts = match g {
                Given::State { state, value, .. } => {
                    let v = lo.expr(value)?;
                    let world = call(ident("board"), "world", vec![]);
                    vec![Stmt::Expr(bin(
                        "=",
                        call(ident("board"), "world", vec![]),
                        call_args(world, "with", vec![Arg::Key(state_name(app, *state), v)]),
                    ))]
                }
                Given::Test { test, .. } => {
                    let other = &app.p.test(*test).info;
                    let (other_steps, other_module) = steps_of(app, other.module);
                    lo.f.need(&other_steps);
                    if other.module != info.module {
                        uses.push(other_module);
                    }
                    vec![
                        Stmt::Expr(bare(&other.name, vec![ident("board")])),
                        Stmt::Expr(call(ident("board"), "forget_output", vec![])),
                    ]
                }
            };
            body.push(lo.f.anchor(entry));
            body.extend(stmts);
        }
        for (s, entry) in t.body.iter().zip(stmt_entries) {
            let stmt = lo.stmt(s)?;
            body.push(lo.f.anchor(entry));
            body.push(stmt);
        }
        let helper = Stmt::Def { singleton: false, name: info.name.clone(), params, body };
        let board = call(testing("Board"), "new", vec![call(konst("App::World"), "initial", vec![])]);
        let mut args = vec![board];
        args.extend(names);
        let run = Stmt::Expr(bare(&info.name, args));
        let run_body = if generated.is_empty() {
            vec![run]
        } else {
            let mut inner = generated;
            inner.push(run);
            vec![Stmt::Expr(block_call(konst("Runtime::Testing"), "runs", vec![], &["gen"], inner))]
        };
        // The steps live in a module outside the test class, so another module's test can start from them.
        if let Some(Stmt::Module { body, .. }) = f.items.first_mut() {
            if !body.is_empty() {
                body.push(Stmt::Blank);
            }
            for other in &uses {
                body.push(Stmt::Expr(bare("include", vec![konst(other)])));
            }
            body.extend([top_anchor, helper]);
        }
        let f = self.test_file(&path, &class_of, "test", info);
        f.need(&steps);
        f.need(TESTING);
        f.need(WORLD);
        let include = Stmt::Expr(bare("include", vec![konst(&steps_module)]));
        if let Some(Stmt::Class { body, .. }) = f.items.first_mut()
            && !body.contains(&include)
        {
            body.insert(0, include);
        }
        let run_anchor = f.anchor(run_entry);
        push_method(
            f,
            vec![
                run_anchor,
                Stmt::Def { singleton: false, name: format!("test_{}", info.name), params: vec![], body: run_body },
            ],
        );
        Ok(())
    }

    fn events(&mut self) -> R<()> {
        let app = self.app;
        let info = &app.decl.info;
        let entry = self.entry(
            info,
            info.span,
            "trace events",
            "trace.events: one event per rule, generated from the parameter types; `apply` calls the rule",
            "the parameters of the app's rules",
        );
        let f = self.file(EVENTS, &["App".to_string(), "Events".to_string()], "app", info);
        let mut lo = Lower::new(app, f, Mode::Pure, &app.p.qualified(info));
        let mut generators = Vec::new();
        let mut whens = Vec::new();
        for r in &app.rules {
            let mut fields = vec![(sym("rule"), sym(&r.name))];
            let mut named = Vec::new();
            for p in &r.params {
                if p.name == "rule" {
                    return lo.inf.unsupported(p.span, "a rule parameter named `rule` in a trace");
                }
                fields.push((sym(&p.name), generate(&mut lo, &p.ty)?));
                named.push((lo.local(&p.name), call(ident("event"), "fetch", vec![sym(&p.name)])));
            }
            let param = if r.params.is_empty() { "_gen" } else { "gen" };
            generators.push((sym(&r.name), Expr::Lambda(vec![Param::Req(param.into())], Box::new(Expr::Hash(fields)))));
            whens.push((sym(&r.name), lo.rule_apply(r.id, ident("world"), named)));
        }
        let anchor = f.anchor(entry);
        f.items.push(anchor);
        f.items.push(Stmt::ConstAssign("GENERATORS".into(), call(Expr::Hash(generators), "freeze", vec![])));
        f.items.push(Stmt::Blank);
        let one_of = call(
            call(konst("GENERATORS"), "fetch", vec![call(ident("gen"), "pick", vec![ident("rules")])]),
            "call",
            vec![ident("gen")],
        );
        f.items.push(Stmt::Def {
            singleton: true,
            name: "one_of".into(),
            params: vec![Param::Req("gen".into()), Param::Req("rules".into())],
            body: vec![Stmt::Expr(one_of)],
        });
        f.items.push(Stmt::Blank);
        let any = bare("one_of", vec![ident("gen"), call(konst("GENERATORS"), "keys", vec![])]);
        f.items.push(Stmt::Def {
            singleton: true,
            name: "any".into(),
            params: vec![Param::Req("gen".into())],
            body: vec![Stmt::Expr(any)],
        });
        f.items.push(Stmt::Blank);
        let apply = Stmt::Case { on: call(ident("event"), "fetch", vec![sym("rule")]), whens };
        f.items.push(Stmt::Def {
            singleton: true,
            name: "apply".into(),
            params: vec![Param::Req("world".into()), Param::Req("event".into())],
            body: vec![apply],
        });
        Ok(())
    }

    fn law(&mut self, law: &Law) -> R<()> {
        let app = self.app;
        let info = &app.p.law(law.id).info;
        let (path, class_of) = self.module_test_path(info.module);
        let qualified = app.p.qualified(info);
        let rule_names: Vec<String> = law.rules.iter().map(|r| app.p.rule(*r).info.name.clone()).collect();
        let top =
            self.entry(info, info.span, "law", "law.predicate: the law as a method of a trace", "the law as written");
        let run_entry = self.entry(
            info,
            info.name_span,
            "law.run",
            "law.sampled: seeded random traces of the trace's rules, each from a world the app's rules can reach; the law is evaluated on the whole trace",
            &format!("Trace[{}]", rule_names.join(" | ")),
        );
        let f = self.test_file(&path, &class_of, "law", info);
        f.need(TESTING);
        f.need(EVENTS);
        f.need(WORLD);
        let top_anchor = f.anchor(top);
        let mut lo = Lower::new(app, f, Mode::Pure, &qualified);
        let trace = lo.local(&law.trace);
        let mut params = vec![Param::Req(trace.clone())];
        let mut run = Vec::new();
        let events = || konst("App::Events");
        let apply = |recv: Expr, name: &str, args: Vec<Expr>| {
            block_call(
                recv,
                name,
                args,
                &["world", "event"],
                vec![Stmt::Expr(call(events(), "apply", vec![ident("world"), ident("event")]))],
            )
        };
        run.push(Stmt::Assign(
            "prefix".into(),
            block_call(
                ident("gen"),
                "list",
                vec![testing("PREFIX_MAX")],
                &[],
                vec![Stmt::Expr(call(events(), "any", vec![ident("gen")]))],
            ),
        ));
        let rules = Expr::Array(rule_names.iter().map(|n| sym(n)).collect());
        run.push(Stmt::Assign(
            "events".into(),
            block_call(
                konst("Array"),
                "new",
                vec![testing("TRACE_LENGTH")],
                &[],
                vec![Stmt::Expr(call(events(), "one_of", vec![ident("gen"), rules]))],
            ),
        ));
        let mut law_args = vec![ident(&trace)];
        for p in &law.params {
            let n = lo.local(&p.name);
            params.push(Param::Req(n.clone()));
            run.push(Stmt::Assign(n.clone(), generate(&mut lo, &p.ty)?));
            law_args.push(ident(&n));
        }
        let start = apply(
            konst("Runtime::Testing"),
            "reach",
            vec![call(konst("App::World"), "initial", vec![]), ident("prefix")],
        );
        run.push(Stmt::Assign("start".into(), start));
        run.push(Stmt::Assign(
            trace.clone(),
            apply(konst("Runtime::Testing"), "run_trace", vec![ident("start"), ident("events")]),
        ));
        let seed = Expr::Interp(vec![
            Part::Lit(format!("{}: seed ", info.name)),
            Part::Expr(call(ident("gen"), "seed", vec![])),
        ]);
        run.push(Stmt::Expr(bare("assert", vec![bare(&info.name, law_args), seed])));
        let body = vec![Stmt::Expr(lo.expr(&law.body)?)];
        let helper = Stmt::Def { singleton: false, name: info.name.clone(), params, body };
        let test = Stmt::Def {
            singleton: false,
            name: format!("test_{}", info.name),
            params: vec![],
            body: vec![Stmt::Expr(block_call(konst("Runtime::Testing"), "runs", vec![], &["gen"], run))],
        };
        let run_anchor = f.anchor(run_entry);
        push_method(f, vec![top_anchor, helper, Stmt::Blank, run_anchor, test]);
        Ok(())
    }

    fn help(&mut self) {
        let app = self.app;
        let info = &app.decl.info;
        let entry = self.entry(
            info,
            info.span,
            "help test",
            "help.pinned: the text the command table gives at load equals the text the compiler derived",
            "one `--help` line per cli rule, in the app's order",
        );
        let f = self.test_file("test/app/help_test.rb", "app/help", "app", info);
        f.need(CLI);
        let check = assert_equal(text(&app.help), call(konst("App::APP"), "help", vec![]));
        let anchor = f.anchor(entry);
        push_method(
            f,
            vec![
                anchor,
                Stmt::Def {
                    singleton: false,
                    name: "test_help_is_the_text_the_cli_vocabulary_derives".into(),
                    params: vec![],
                    body: vec![Stmt::Expr(check)],
                },
            ],
        );
    }
}

/// Add statements to the file's test class, a blank line after the ones before.
fn push_method(f: &mut super::file::RFile, stmts: Vec<Stmt>) {
    if let Some(Stmt::Class { body, .. }) = f.items.first_mut() {
        if !body.is_empty() {
            body.push(Stmt::Blank);
        }
        body.extend(stmts);
    }
}
