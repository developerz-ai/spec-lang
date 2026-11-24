//! Parser unit tests. Each `accepts`/`rejects` pair below was checked against
//! the Lark reference checker (`pocs/todo-app/scripts/slg/check.py`).

use super::parse_file;
use crate::ast::{ExprKind, StmtKind, Tail};
use crate::span::FileId;

fn ok(src: &str) -> crate::ast::File {
    match parse_file(FileId(0), src) {
        Ok(f) => f,
        Err(e) => panic!("{src:?} should parse: {e}"),
    }
}

fn bad(src: &str) {
    assert!(parse_file(FileId(0), src).is_err(), "{src:?} should not parse");
}

#[test]
fn accepts_what_lark_accepts() {
    for src in [
        "",
        "use a.b; type X = A | B(x: T);",
        "x y = match;\n".replace("match;", "a;").as_str(),
        "x y { use z; }",
        "a b; use c;",
        "f x = format;",
        "f x = { for in in xs { } }",
        "f x = 1e5m + 0xg + 2em;",
        "f x = a.is(fail(_));",
        "f x = in;",
        "f x = notx;",
        "f x = a elseb;",
        "f x = 5 modx;",
        "f x = a wherey;",
        "f x = a bandb;",
        "f x = if a thenb else c;",
        "f x = { returnx; }",
        "pubx y;",
        "f x = { letx = 1; }",
        "f x = {match x {}}",
        "f x = {matchx;}",
        "f x = machinex;",
        "f x = a.if;",
        "f x = (match: 1);",
        "a b;\nif c = 2;",
        "f x = match y { if => 2; }",
        "f x = µs + 5µs + ½ + a²;",
        "const x = re\"a\";",
        "fn f() = foreign c\"x\";",
        "f x = { if a { } else iffy { } }",
        "f x = { if a then b else c; }",
        "f x = { if a then b else c -> d = 1; }",
        "f x = a == not;",
        "@n(x: 1, y) pub rule r[T](a: Int = 1, @p b) -> T where self > 0 = { x.y = z; }",
        "law l(t: Trace[a | b]) = t.steps.all(it.after == it.before);",
    ] {
        ok(src);
    }
}

#[test]
fn rejects_what_lark_rejects() {
    for src in [
        "x y = match;",
        "foreign c\"x\";",
        "f x = machine {};",
        "f x = (if: 1);",
        "f x = a²b;",
        "f x = {for x inxs {}}",
        "f x = { if a { } elsex }",
        "f x = a band b band c;",
        "f x = a..=b..=c;",
        "f x = x[];",
        "f x = 1 +;",
        "f x",
        "f x = { if a else b then c else d; }",
        "rule pub() = 1;",
        "a b = 1;\nif c = 2;",
        "a b = { }\nmatch c = 2;",
        "f x = match y { A => 1; if => 2; }",
        "f x(@p pub) = 1;",
    ] {
        bad(src);
    }
}

#[test]
fn keywords_are_names_where_a_keyword_cannot_stand() {
    let f = ok("f x = { for in in xs { } }");
    let Tail::Body(crate::ast::Body::Value(crate::ast::Value::Block(b))) = &f.decls[0].tail else {
        panic!("block body")
    };
    let StmtKind::For { var, .. } = &b.stmts[0].kind else { panic!("for") };
    assert_eq!(var.name, "in");
}

#[test]
fn keyword_prefixes_split_after_an_operand() {
    let f = ok("f x = a elseb;");
    let Tail::Body(crate::ast::Body::Value(crate::ast::Value::Expr(e))) = &f.decls[0].tail else { panic!("expr body") };
    let ExprKind::Else(_, r) = &e.kind else { panic!("else") };
    assert_eq!(r.as_name(), Some("b"));
}

#[test]
fn errors_name_line_column_and_what_was_expected() {
    let mut map = crate::span::SourceMap::new();
    let src = "type A = 1;\nfn f( = 2;\n";
    let id = map.add(crate::span::SourceFile::new("m.slg", src, 1));
    let e = parse_file(id, src).unwrap_err();
    assert_eq!(e.render(&map), "m.slg:2:7: does not parse: expected a name, found `=`");
}
