//! `checks.rs`: every check of the program as data for the runner (`rt::runner`), in the LIR's order and with
//! `spec-lang test`'s names: `@cases` rows, scenarios with their `@given` worlds, properties, laws.

use crate::code::{Code, Fx};
use crate::cx::Cx;
use crate::dty::gen_fn;
use crate::names::ident;
use spec_lang_lir::node::{Expected, Expr, ExprKind};
use spec_lang_lir::program::{CaseTarget, CheckWhat, Given};
use spec_lang_lir::{LTy, Param};

fn test_path(cx: &Cx<'_>, t: spec_lang_ir::TestId) -> String {
    let d = cx.lir.test(t);
    cx.names.path(None, d.info.module, &ident(&d.info.name))
}

/// `Box::new(move |m| test(m, a0, ..))` over named arguments, `Some(..)` where the parameter has a default.
fn body_of(path: &str, params: &[Param], args: &[String]) -> String {
    let mut list = vec!["m".to_string()];
    for (p, a) in params.iter().zip(args) {
        list.push(if p.default.is_some() && a != "None" { format!("Some({a})") } else { a.clone() });
    }
    format!("Ok(Box::new(move |m: &mut Machine| {path}({})))", list.join(", "))
}

pub fn file(cx: &mut Cx<'_>) -> String {
    let lir = cx.lir;
    let mut items = Vec::new();

    // Tests, by `TestId`.
    let mut tests = Vec::new();
    for (ti, t) in lir.tests.iter().enumerate() {
        let mut given = Vec::new();
        for (k, g) in t.given.iter().enumerate() {
            match g {
                Given::Test(p) => given.push(format!("Given::Test({})", p.index())),
                Given::State { state, value } => {
                    let f = format!("given_{ti}_{k}");
                    let s = lir.state(*state);
                    let mut fx = Fx::new(cx, None, true);
                    let v = fx.value(value);
                    let mut lines = v.pre.clone();
                    lines.push(format!("let v = {};", v.text));
                    if let Some(line) = fx.check(&s.check, "v", &format!("the `@given` value of {}", s.info.qualified)) {
                        lines.push(line);
                    }
                    lines.push(format!("m.world.{} = v;", cx.names.state(*state)));
                    items.push(format!(
                        "/// `@given({} -> ..)` of `{}`.\nfn {f}(m: &mut Machine) -> R<()> {{ {} Ok(()) }}",
                        s.info.qualified,
                        t.info.qualified,
                        lines.join(" ")
                    ));
                    given.push(format!("Given::State({f})"));
                }
            }
        }
        let run = if t.params.is_empty() { test_path(cx, spec_lang_ir::TestId(ti as u32)) } else { "no_end".into() };
        tests.push(format!(
            "TestDecl {{ name: {:?}, has_params: {}, given: &[{}], run: {run} }},",
            t.info.qualified,
            !t.params.is_empty(),
            given.join(", ")
        ));
    }
    items.push(
        "/// A test with parameters has no one end world.\nfn no_end(_: &mut Machine) -> R<()> { Err(rt::stop(\"E-TYPE-MISMATCH\", \"a test with parameters has no one end world\")) }"
            .into(),
    );

    // Laws, by `LawId`.
    let mut laws = Vec::new();
    for (li, l) in lir.laws.iter().enumerate() {
        let f = format!("law_{li}");
        let mut lines = Vec::new();
        let mut args = Vec::new();
        for (pi, p) in l.params.iter().enumerate() {
            if pi == l.trace {
                args.push("t".to_string());
                continue;
            }
            let g = gen_fn(cx, &p.check);
            lines.push(format!("let a{pi} = crate::draw::{g}(m, rng, pools)?;"));
            args.push(format!("a{pi}"));
        }
        let path = cx.names.path(None, l.info.module, &ident(&l.info.name));
        items.push(format!(
            "/// The parameters of `{}` besides its trace, drawn after it.\nfn {f}(m: &mut Machine, rng: &mut Rng, pools: &Pools) -> R<LawBody> {{ {} Ok(Box::new(move |m: &mut Machine, t: Trace| {path}(m, {}))) }}",
            l.info.qualified,
            lines.join(" "),
            args.join(", ")
        ));
        let traced: Vec<String> = l.traced.iter().map(|r| r.index().to_string()).collect();
        let start: Vec<String> = l.start_rules.iter().map(|r| r.index().to_string()).collect();
        laws.push(format!(
            "LawDecl {{ name: {:?}, traced: &[{}], start: &[{}], draw: {f} }},",
            l.info.qualified,
            traced.join(", "),
            start.join(", ")
        ));
    }

    // Checks.
    let mut checks = Vec::new();
    for (k, c) in lir.checks.iter().enumerate() {
        let what = match &c.what {
            CheckWhat::Test(t) => format!("What::Test({})", t.index()),
            CheckWhat::Law(l) => format!("What::Law({})", l.index()),
            CheckWhat::Property(t) => {
                let f = format!("draw_{k}");
                let test = lir.test(*t);
                let mut lines = vec!["let pools = Pools::default();".to_string()];
                let mut args = Vec::new();
                for (pi, p) in test.params.iter().enumerate() {
                    let g = gen_fn(cx, &p.check);
                    lines.push(format!("let a{pi} = crate::draw::{g}(m, rng, &pools)?;"));
                    args.push(format!("a{pi}"));
                }
                lines.push(body_of(&test_path(cx, *t), &test.params, &args));
                items.push(format!(
                    "/// Arguments of `{}` drawn from its parameters' types.\nfn {f}(m: &mut Machine, rng: &mut Rng) -> R<Body> {{ {} }}",
                    test.info.qualified,
                    lines.join(" ")
                ));
                format!("What::Property {{ test: {}, draw: {f} }}", t.index())
            }
            CheckWhat::Case { target: CaseTarget::Test(t), args, .. } => {
                let f = format!("case_{k}");
                let test = lir.test(*t);
                let mut fx = Fx::new(cx, None, true);
                let mut lines = Vec::new();
                let mut names = Vec::new();
                for (pi, (p, a)) in test.params.iter().zip(args).enumerate() {
                    match a {
                        Some(e) => {
                            let v = fx.value(e);
                            lines.extend(v.pre.iter().cloned());
                            lines.push(format!("let a{pi} = {};", v.text));
                            names.push(format!("a{pi}"));
                        }
                        None if p.default.is_some() => names.push("None".into()),
                        None => {
                            fx.cx.unsupported(test.info.span, format!("a `@cases` row without `{}`", p.name));
                            names.push("None".into());
                        }
                    }
                }
                lines.push(body_of(&test_path(cx, *t), &test.params, &names));
                items.push(format!("/// `{}`: the row's values.\nfn {f}(m: &mut Machine) -> R<Body> {{ {} }}", c.name, lines.join(" ")));
                format!("What::TestCase {{ test: {}, args: {f} }}", t.index())
            }
            CheckWhat::Case { target: CaseTarget::Fn(func), args, expect } => {
                let f = format!("case_{k}");
                let d = lir.func(*func);
                let call = Expr::new(ExprKind::CallFn { func: *func, args: args.clone() }, d.result.clone(), d.info.span);
                let mut fx = Fx::new(cx, None, true);
                let got: Code = fx.expr(&call);
                let mut lines = got.pre.clone();
                let text = got.text.strip_suffix('?').unwrap_or(&got.text).to_string();
                lines.push(format!("let got = rt::settle({text})?;"));
                match expect {
                    Expected::Value(e) => {
                        let w = fx.value(e);
                        lines.extend(w.pre.iter().cloned());
                        let ty = fx.ty(&d.result);
                        lines.push(format!("let want: {ty} = {};", w.text));
                        lines.push("Ok(got == Ok(want))".into());
                    }
                    Expected::Fail { name, .. } => lines.push(format!("Ok(matches!(&got, Err(f) if f.name == {name:?}))")),
                }
                items.push(format!("/// `{}`.\nfn {f}(m: &mut Machine) -> R<bool> {{ {} }}", c.name, lines.join(" ")));
                format!("What::Case({f})")
            }
        };
        checks.push(format!("Check {{ name: {:?}, what: {what} }},", c.name));
    }
    let _ = LTy::Unit;
    items.push(format!("/// Every test, by number.\npub static TESTS: &[TestDecl] = &[{}];", tests.join(" ")));
    items.push(format!("/// Every law, by number.\npub static LAWS: &[LawDecl] = &[{}];", laws.join(" ")));
    items.push(format!("/// Every check, in the order `spec-lang test` runs them.\npub static CHECKS: &[Check] = &[{}];", checks.join(" ")));
    items.join("\n\n")
}
