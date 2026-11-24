//! One lowering per test: what the LIR says for a form, and that the
//! evaluator gives the engine's answer on it.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use spec_lang_lir::eval::{self, Config, Status};
use spec_lang_lir::node::{ExprKind, PatStep, StmtKind};
use spec_lang_lir::{Body, DTy, LTy, Lir, Prim, TypeDef};

fn program(src: &str) -> spec_lang_ir::Program {
    let text = format!("use cli;\n{src}");
    spec_lang_ir::load_sources(vec![("m".into(), "m.slg".into(), text)]).unwrap_or_else(|f| panic!("{}", f.render()))
}

fn lir(src: &str) -> (spec_lang_ir::Program, Lir) {
    let p = program(src);
    let l = spec_lang_lir::lower(&p).unwrap_or_else(|ds| panic!("{ds:?}"));
    (p, l)
}

/// The body of the `fn` named `name`.
fn body<'a>(l: &'a Lir, name: &str) -> &'a spec_lang_lir::node::Expr {
    let f = l.fns.iter().find(|f| f.info.name == name).unwrap();
    match &f.body {
        Body::Expr(e) => e,
        Body::Extern(_) => panic!("extern"),
    }
}

fn prims(e: &spec_lang_lir::node::Expr) -> Vec<Prim> {
    let mut out = Vec::new();
    spec_lang_lir::walk::exprs(e, &mut |x| match &x.kind {
        ExprKind::Prim { op, .. } => out.push(*op),
        ExprKind::Compare { links, .. } => out.extend(links.iter().map(|(op, _)| *op)),
        _ => {}
    });
    out
}

/// Every check passes on the LIR evaluator.
fn green(l: &Lir) {
    let cfg = Config { jobs: 1, ..Config::default() };
    for c in eval::run_all(l, cfg) {
        assert_eq!(c.status, Status::Pass, "{}: {:?}", c.name, c.diag);
    }
}

#[test]
fn a_member_is_resolved_by_the_receivers_type() {
    let (_, l) = lir("fn a(t: Text, xs: List[Int], s: Set[Int]) -> Int = t.size + xs.size + s.size;");
    assert_eq!(prims(body(&l, "a")), [Prim::IntAdd, Prim::IntAdd, Prim::TextSize, Prim::ListSize, Prim::SetSize]);
}

#[test]
fn filter_on_a_set_is_a_list_filter_rebuilt_as_a_set() {
    let (_, l) = lir("fn a(s: Set[Int]) -> Set[Int] = s.filter(it > 1);");
    let e = body(&l, "a");
    assert_eq!(prims(e)[..3], [Prim::SetOfList, Prim::ListFilter, Prim::SetList]);
    assert_eq!(e.ty, LTy::set(LTy::Int));
}

#[test]
fn a_range_type_is_int_and_its_bounds_are_a_check() {
    let (_, l) = lir("type Small = 1..=5;\nfn a(n: Small) -> Int = n;");
    let f = l.fns.iter().find(|f| f.info.name == "a").unwrap();
    assert_eq!(f.params[0].ty, LTy::Int);
    let small = l.types.iter().find(|t| t.info.name == "Small").unwrap();
    assert!(matches!(&small.def, TypeDef::Alias { of: DTy::Range { lo: 1, hi: 5, inclusive: true } }));
}

#[test]
fn a_claim_is_a_function_of_self() {
    let (_, l) = lir("type Pos = Int where self >= 1;\n@cases(1 -> 1)\nfn a(n: Pos) -> Int = n;");
    assert_eq!(l.claims.len(), 1);
    assert_eq!(l.claims[0].self_ty, LTy::Int);
    assert_eq!(prims(&l.claims[0].body), [Prim::IntGe]);
    green(&l);
}

#[test]
fn a_pattern_is_tests_then_bindings() {
    let (_, l) =
        lir("@cases(Some(2) -> 2, none -> 0)\nfn a(o: Option[Int]) -> Int = match o { Some(x) => x; none => 0; }");
    let ExprKind::Match { arms, .. } = &body(&l, "a").kind else { panic!() };
    assert!(matches!(arms[0].pat.steps.as_slice(), [PatStep::Tag { tag: 0, .. }, PatStep::Bind { .. }]));
    assert!(matches!(arms[1].pat.steps.as_slice(), [PatStep::Tag { tag: 1, .. }]));
    green(&l);
}

#[test]
fn else_unwraps_an_option_and_not_an_index() {
    let (_, l) =
        lir("fn a(o: Option[Int]) -> Int = o else 0;\nfn b(m: Map[Int, Option[Int]]) -> Option[Int] = m[1] else none;");
    assert!(matches!(body(&l, "a").kind, ExprKind::Else { unwrap: true, .. }));
    assert!(matches!(body(&l, "b").kind, ExprKind::Else { unwrap: false, .. }));
}

#[test]
fn a_chain_of_options_with_else_has_one_type() {
    let (_, l) = lir(
        "@cases((none, Some(2)) -> 2, (none, none) -> 9, (Some(1), none) -> 1)\nfn a(x: Option[Int], y: Option[Int]) -> Int = x else y else 9;",
    );
    assert_eq!(body(&l, "a").ty, LTy::Int);
    green(&l);
}

#[test]
fn in_names_a_containment_primitive() {
    let (_, l) = lir("@cases(2 -> true)\nfn a(n: Int) -> Bool = n in Set(1, 2) and n in 1..=3 and \"b\" in \"abc\";");
    let ps = prims(body(&l, "a"));
    assert!(ps.contains(&Prim::SetContains) && ps.contains(&Prim::RangeHas) && ps.contains(&Prim::TextContains));
    green(&l);
}

#[test]
fn a_constructor_leaves_defaults_to_the_type() {
    let (_, l) =
        lir("type P(a: Int, b: Int = 7);\n@cases(1 -> 8)\nfn f(n: Int) -> Int = { let p = P(n); return p.a + p.b; }");
    let mut absent = 0;
    spec_lang_lir::walk::exprs(body(&l, "f"), &mut |x| {
        if let ExprKind::Construct { args, .. } = &x.kind {
            absent = args.iter().filter(|a| a.is_none()).count();
        }
    });
    assert_eq!(absent, 1);
    green(&l);
}

#[test]
fn out_line_is_a_named_effect_and_out_text_reads_its_pretend() {
    let (_, l) = lir("rule hi() = { out.line(\"hi\"); }\ntest says_hi = { hi(); assert out.text == \"hi\\n\"; }");
    let r = l.rules.iter().find(|r| r.info.name == "hi").unwrap();
    let Body::Expr(e) = &r.body else { panic!() };
    let mut names = Vec::new();
    spec_lang_lir::walk::exprs(e, &mut |x| {
        if let ExprKind::Effect { name, .. } = &x.kind {
            names.push(name.clone());
        }
    });
    assert_eq!(names, ["out.line"]);
    assert!(l.handlers.iter().any(|h| h.name == "out" && h.pretend.is_some()));
    green(&l);
}

#[test]
fn a_foreign_body_is_an_extern_and_blocks_its_check() {
    let (_, l) = lir("@cases(1 -> 2)\nfn f(n: Int) -> Int = foreign c \"lib#twice\";");
    let f = l.fns.iter().find(|f| f.info.name == "f").unwrap();
    let Body::Extern(x) = &f.body else { panic!() };
    assert_eq!((x.lang.as_str(), x.library.as_str(), x.symbol.as_str()), ("c", "lib", "twice"));
    let r = eval::run_all(&l, Config::default());
    assert_eq!(r[0].status, Status::Blocked);
    assert!(r[0].blocked_by_foreign());
}

#[test]
fn equality_of_two_types_is_false_after_both_sides() {
    let (_, l) = lir("test t = { assert Some(Some(1)) != Some(1); }");
    let t = &l.tests[0];
    let StmtKind::Assert(e) = &t.body.stmts[0].kind else { panic!() };
    assert!(matches!(e.kind, ExprKind::Block(_)));
    green(&l);
}

#[test]
fn a_lambda_names_what_it_captures() {
    let (_, l) = lir("@cases(([1, 2], 1) -> [2, 3])\nfn f(xs: List[Int], k: Int) -> List[Int] = xs.map(x => x + k);");
    let mut caps = Vec::new();
    spec_lang_lir::walk::exprs(body(&l, "f"), &mut |x| {
        if let ExprKind::Lambda(lam) = &x.kind {
            caps = lam.captures.iter().map(|(n, _)| n.clone()).collect();
        }
    });
    assert_eq!(caps, ["k"]);
    green(&l);
}

#[test]
fn an_empty_literal_nothing_types_is_unit() {
    let (_, l) = lir("test t = { assert [] + [] == []; }");
    green(&l);
}

#[test]
fn a_mistyped_expression_is_an_error_at_its_span() {
    let p = program("fn f(t: Text) -> Int = t + 1;");
    let ds = spec_lang_lir::lower(&p).unwrap_err();
    assert_eq!(ds[0].code, "E-TYPE-MISMATCH");
    assert!(ds[0].span.is_some());
}

#[test]
fn a_handled_inner_refusal_undoes_the_inner_rules_writes() {
    // The card (How it runs 1): a failure leaves the world unchanged.
    let (_, l) = lir(
        "state n: Int = 0;\nrule inner() -> Int = { n = 5; fail No; }\nrule outer() -> Int = { return inner() else n; }\ntest t = { outer() -> 0; assert n == 0; }",
    );
    green(&l);
}

#[test]
fn a_rule_is_a_guarded_atomic_transition() {
    let (_, l) = lir(
        "state n: Int = 0;\n@guard(k > 0 else Bad)\nrule add(k: Int) = { n = n + k; if n > 10 { fail Big; } }\ntest t = { add(0) -> fail Bad; add(4); add(9) -> fail Big; assert n == 4; }",
    );
    let r = l.rules.iter().find(|r| r.info.name == "add").unwrap();
    assert_eq!(r.guards[0].error, "Bad");
    green(&l);
}

#[test]
fn an_apps_manifest_is_its_notes_and_reachable_rules() {
    let (_, l) = lir(
        "state n: Int = 0;\nrule bump() = { n = n + 1; }\n@on(cli(\"go\"))\nrule go() = { bump(); }\n@target(rust)\n@bin(\"x\")\napp A(rules: [go]);",
    );
    let m = &l.apps[0];
    assert_eq!(m.target.as_deref(), Some("rust"));
    assert_eq!(m.notes[0].name, "bin");
    let names: Vec<&str> = m.rules.iter().map(|r| l.rule(*r).info.name.as_str()).collect();
    assert_eq!(names, ["bump", "go"]);
    let go = l.rules.iter().find(|r| r.info.name == "go").unwrap();
    assert_eq!(go.triggers[0].name, "cli");
}

#[test]
fn the_text_and_json_renderings_are_stable() {
    let (_, l) = lir("type P(a: Int);\n@cases(P(1) -> 1)\nfn f(p: P) -> Int = p.a;");
    assert_eq!(spec_lang_lir::print::text::program(&l), spec_lang_lir::print::text::program(&l));
    let j = spec_lang_lir::print::json::program(&l).to_string();
    assert!(j.contains("\"check\":\"m.f case[0]\""));
}
