//! The engine's meaning, one rule of the card or library per test.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use spec_lang_engine::{Halt, Interp, Outcome, Value};
use spec_lang_ir::{Program, load_sources};

fn program(src: &str) -> Program {
    let text = format!("use cli;\n{src}");
    match load_sources(vec![("m".into(), "m.slg".into(), text)]) {
        Ok(p) => p,
        Err(f) => panic!("{}", f.render()),
    }
}

fn rule(p: &Program, name: &str) -> spec_lang_ir::RuleId {
    spec_lang_ir::RuleId(p.rules.iter().position(|r| r.info.name == name).unwrap() as u32)
}

fn func(p: &Program, name: &str) -> spec_lang_ir::FnId {
    spec_lang_ir::FnId(p.fns.iter().position(|r| r.info.name == name).unwrap() as u32)
}

fn call(p: &Program, it: &mut Interp<'_>, name: &str, args: Vec<Value>) -> Result<Value, Halt> {
    it.call_fn(func(p, name), args.into_iter().map(Some).collect())
}

#[test]
fn a_failing_rule_leaves_the_world_and_output_unchanged() {
    let p = program(
        "state n: Int = 0;\n@guard(n < 1 else Full)\nrule up() = { n = n + 1; out.line(\"up\"); }\n\
         rule twice() = { n = n + 1; out.line(\"a\"); fail Late; }",
    );
    let mut it = Interp::new(&p).unwrap();
    assert!(matches!(it.step(rule(&p, "up"), vec![]).unwrap(), Outcome::Ok(Value::Unit)));
    assert!(matches!(it.step(rule(&p, "up"), vec![]).unwrap(), Outcome::Refused(f) if f.name == "Full"));
    assert!(matches!(it.step(rule(&p, "twice"), vec![]).unwrap(), Outcome::Refused(f) if f.name == "Late"));
    assert_eq!(it.world.states[0], Value::Int(1));
    assert_eq!(it.out_text(), "up\n");
}

#[test]
fn guards_run_in_order_and_a_failure_inside_one_is_its_error() {
    let p = program(
        "state m: Map[Int, Int];\n@guard(m[k] > 0 else Second, k in m else First)\nrule r(k: Int) = { return; }",
    );
    let mut it = Interp::new(&p).unwrap();
    assert!(matches!(it.step(rule(&p, "r"), vec![Value::Int(3)]).unwrap(), Outcome::Refused(f) if f.name == "Second"));
}

#[test]
fn missing_keys_fail_and_else_handles_them() {
    let p = program(
        "fn get(m: Map[Int, Text], k: Int) -> Text = m[k];\nfn or(m: Map[Int, Text], k: Int) -> Text = m[k] else \"-\";\n\
         fn opt(m: Map[Int, Text], k: Int) -> Text = m.get(k) else \"-\";",
    );
    let mut it = Interp::new(&p).unwrap();
    let empty = Value::Map(Default::default());
    assert!(
        matches!(call(&p, &mut it, "get", vec![empty.clone(), Value::Int(1)]), Err(Halt::Fail(f)) if f.name == "Missing")
    );
    assert_eq!(call(&p, &mut it, "or", vec![empty.clone(), Value::Int(1)]).unwrap(), Value::text("-"));
    assert_eq!(call(&p, &mut it, "opt", vec![empty, Value::Int(1)]).unwrap(), Value::text("-"));
}

#[test]
fn claims_are_checked_at_construction_and_on_arguments() {
    let p = program(
        "type Small = Int where self < 10;\ntype Box(n: Small);\nfn make(n: Int) -> Box = Box(n);\nfn id(s: Small) -> Small = s;",
    );
    let mut it = Interp::new(&p).unwrap();
    assert!(matches!(call(&p, &mut it, "make", vec![Value::Int(10)]), Err(Halt::Fatal(d)) if d.code == "E-CLAIM"));
    assert!(matches!(call(&p, &mut it, "id", vec![Value::Int(11)]), Err(Halt::Fatal(d)) if d.code == "E-CLAIM"));
    assert!(call(&p, &mut it, "make", vec![Value::Int(9)]).is_ok());
}

#[test]
fn text_counts_code_points_and_trims_unicode_white_space() {
    let p = program("fn n(t: Text) -> Int = t.trim().size;\nfn show(x: Int) -> Text = \"<{x}>\";");
    let mut it = Interp::new(&p).unwrap();
    assert_eq!(call(&p, &mut it, "n", vec![Value::text("\u{3000}é😀\u{85}")]).unwrap(), Value::Int(2));
    assert_eq!(call(&p, &mut it, "n", vec![Value::text("\u{FEFF}")]).unwrap(), Value::Int(1));
    assert_eq!(call(&p, &mut it, "show", vec![Value::Int(-3)]).unwrap(), Value::text("<-3>"));
}

#[test]
fn collections_are_ordered_and_sort_is_stable() {
    let p = program(
        "fn keys(s: Set[Int]) -> Text = s.sort(it).map(x => \"{x}\").join(\",\");\n\
         fn by_len(xs: List[Text]) -> List[Text] = xs.sort(it.size);",
    );
    let mut it = Interp::new(&p).unwrap();
    let s: std::collections::BTreeSet<Value> = [3, 1, 2].into_iter().map(Value::Int).collect();
    assert_eq!(call(&p, &mut it, "keys", vec![Value::Set(s.into())]).unwrap(), Value::text("1,2,3"));
    let xs = Value::list(vec![Value::text("bb"), Value::text("a"), Value::text("cc"), Value::text("d")]);
    let want = Value::list(vec![Value::text("a"), Value::text("d"), Value::text("bb"), Value::text("cc")]);
    assert_eq!(call(&p, &mut it, "by_len", vec![xs]).unwrap(), want);
}

#[test]
fn integers_stop_instead_of_wrapping_and_division_is_a_question() {
    let p = program("fn sq(x: Int) -> Int = x * x;\nfn half(x: Int) -> Int = x / 2;\nfn m(x: Int) -> Int = x mod 3;");
    let mut it = Interp::new(&p).unwrap();
    assert!(
        matches!(call(&p, &mut it, "sq", vec![Value::Int(i128::MAX)]), Err(Halt::Fatal(d)) if d.code == "E-RUNTIME")
    );
    assert!(
        matches!(call(&p, &mut it, "half", vec![Value::Int(4)]), Err(Halt::Fatal(d)) if d.code == "E-OPEN" && d.alternatives.len() == 2)
    );
    assert_eq!(call(&p, &mut it, "m", vec![Value::Int(-1)]).unwrap(), Value::Int(2));
}

#[test]
fn unsupported_forms_are_errors_with_a_place() {
    let f = load_sources(vec![("m".into(), "m.slg".into(), "const t = 10ms;\n".into())]).unwrap_err();
    assert_eq!(f.errors[0].code, "E-UNSUPPORTED");
    assert!(f.errors[0].human(&f.sources).contains("m.slg:1:11"));
}
