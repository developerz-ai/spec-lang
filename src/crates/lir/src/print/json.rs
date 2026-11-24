//! The LIR as one JSON document, for tools and printers written outside
//! Rust. Every node is an object with `"node"` naming its kind; every
//! expression has `"ty"`. Keys keep a fixed order, so output is byte-stable.

use crate::manifest::{Manifest, Named, Note, NoteArg, NoteValue};
use crate::node::*;
use crate::program::*;
use crate::ty::{DTy, LTy};
use spec_lang_diag::JsonWriter as J;

fn s(x: impl Into<String>) -> J {
    J::str(x)
}

fn int(n: impl Into<i128>) -> J {
    J::Int(n.into())
}

fn arr<T>(xs: &[T], f: impl Fn(&T) -> J) -> J {
    J::Arr(xs.iter().map(f).collect())
}

fn node(kind: &str) -> J {
    J::obj().set("node", s(kind))
}

fn rules(rs: &[spec_lang_ir::RuleId]) -> J {
    arr(rs, |r| int(r.0))
}

pub fn lty(t: &LTy) -> J {
    let k = |n: &str| J::obj().set("ty", s(n));
    match t {
        LTy::Unit => k("Unit"),
        LTy::Bool => k("Bool"),
        LTy::Int => k("Int"),
        LTy::Text => k("Text"),
        LTy::Named(id) => k("Named").set("type", int(id.0)),
        LTy::Option(x) => k("Option").set("of", lty(x)),
        LTy::List(x) => k("List").set("of", lty(x)),
        LTy::Set(x) => k("Set").set("of", lty(x)),
        LTy::Map(a, b) => k("Map").set("key", lty(a)).set("value", lty(b)),
        LTy::Tuple(ts) => k("Tuple").set("items", arr(ts, lty)),
        LTy::Range => k("Range"),
        LTy::Func(ps, r) => k("Func").set("params", arr(ps, lty)).set("result", lty(r)),
        LTy::Event(rs) => k("Event").set("rules", rules(rs)),
        LTy::Reply(rs) => k("Reply").set("rules", rules(rs)),
        LTy::Failure => k("Failure"),
        LTy::Step(rs) => k("Step").set("rules", rules(rs)),
        LTy::World => k("World"),
        LTy::Trace(rs) => k("Trace").set("rules", rules(rs)),
        LTy::Never => k("Never"),
        LTy::Var(v) => k("Var").set("var", int(*v)),
    }
}

pub fn dty(t: &DTy) -> J {
    let k = |n: &str| J::obj().set("ty", s(n));
    match t {
        DTy::Unit => k("Unit"),
        DTy::Bool => k("Bool"),
        DTy::Int => k("Int"),
        DTy::Text => k("Text"),
        DTy::Named(id) => k("Named").set("type", int(id.0)),
        DTy::Option(x) => k("Option").set("of", dty(x)),
        DTy::List(x) => k("List").set("of", dty(x)),
        DTy::Set(x) => k("Set").set("of", dty(x)),
        DTy::Map(a, b) => k("Map").set("key", dty(a)).set("value", dty(b)),
        DTy::Tuple(ts) => k("Tuple").set("items", arr(ts, dty)),
        DTy::Range { lo, hi, inclusive } => {
            k("Range").set("lo", int(*lo)).set("hi", int(*hi)).set("inclusive", J::Bool(*inclusive))
        }
        DTy::Where(b, c) => k("Where").set("base", dty(b)).set("claim", int(c.0)),
        DTy::Trace(rs) => k("Trace").set("rules", rules(rs)),
    }
}

fn opt(x: &Option<Expr>) -> J {
    x.as_ref().map_or(J::Null, expr)
}

fn path(p: &[PathStep]) -> J {
    arr(p, |st| match st {
        PathStep::VariantField { ty, tag, index } => {
            node("variant_field").set("type", int(ty.0)).set("tag", int(*tag)).set("index", int(*index))
        }
        PathStep::TupleItem(i) => node("tuple_item").set("index", int(*i)),
        PathStep::EventArg { rule, index } => node("event_arg").set("rule", int(rule.0)).set("index", int(*index)),
    })
}

fn pat(p: &Pat) -> J {
    arr(&p.steps, |st| match st {
        PatStep::Tag { path: p, ty, tag } => {
            node("tag").set("path", path(p)).set("type", int(ty.0)).set("tag", int(*tag))
        }
        PatStep::Event { path: p, rule } => node("event").set("path", path(p)).set("rule", int(rule.0)),
        PatStep::Equals { path: p, value } => node("equals").set("path", path(p)).set("value", expr(value)),
        PatStep::Never => node("never"),
        PatStep::Bind { name, path: p, ty } => {
            node("bind").set("name", s(name)).set("path", path(p)).set("ty", lty(ty))
        }
    })
}

pub fn expr(e: &Expr) -> J {
    let o = match &e.kind {
        ExprKind::Unit => node("unit"),
        ExprKind::Bool(b) => node("bool").set("value", J::Bool(*b)),
        ExprKind::Int(n) => node("int").set("value", int(*n)),
        ExprKind::Text(t) => node("text").set("value", s(t)),
        ExprKind::Interp(parts) => node("interp").set(
            "parts",
            arr(parts, |p| match p {
                Part::Lit(t) => node("lit").set("value", s(t)),
                Part::Show(x) => node("show").set("value", expr(x)),
            }),
        ),
        ExprKind::Local(n) => node("local").set("name", s(n)),
        ExprKind::Const(c) => node("const").set("const", int(c.0)),
        ExprKind::State(st) => node("state").set("state", int(st.0)),
        ExprKind::Variant { ty, tag } => node("variant").set("type", int(ty.0)).set("tag", int(*tag)),
        ExprKind::Some(x) => node("some").set("value", expr(x)),
        ExprKind::None => node("none"),
        ExprKind::Construct { ty, tag, args } => {
            node("construct").set("type", int(ty.0)).set("tag", tag.map_or(J::Null, int)).set("args", arr(args, opt))
        }
        ExprKind::CallFn { func, args } => node("call_fn").set("fn", int(func.0)).set("args", arr(args, opt)),
        ExprKind::CallRule { rule, args } => node("call_rule").set("rule", int(rule.0)).set("args", arr(args, opt)),
        ExprKind::Effect { handler, op, name, args } => node("effect")
            .set("handler", int(handler.0))
            .set("op", int(*op))
            .set("name", s(name))
            .set("args", arr(args, expr)),
        ExprKind::EffectRead { handler, op, name } => {
            node("effect_read").set("handler", int(handler.0)).set("op", int(*op)).set("name", s(name))
        }
        ExprKind::Prim { op, args } => node("prim").set("op", s(op.name())).set("args", arr(args, expr)),
        ExprKind::Lambda(l) => node("lambda")
            .set("params", arr(&l.params, |(n, t)| J::obj().set("name", s(n)).set("ty", lty(t))))
            .set("captures", arr(&l.captures, |(n, t)| J::obj().set("name", s(n)).set("ty", lty(t))))
            .set("result", lty(&l.result))
            .set(
                "body",
                match &l.body {
                    LambdaBody::Expr(x) => expr(x),
                    LambdaBody::Block(b) => block(b),
                },
            ),
        ExprKind::CallLambda { callee, args } => {
            node("call_lambda").set("callee", expr(callee)).set("args", arr(args, expr))
        }
        ExprKind::Field { recv, ty, index, name } => {
            node("field").set("recv", expr(recv)).set("type", int(ty.0)).set("index", int(*index)).set("name", s(name))
        }
        ExprKind::VariantField { recv, ty, name, by_tag } => node("variant_field")
            .set("recv", expr(recv))
            .set("type", int(ty.0))
            .set("name", s(name))
            .set("by_tag", arr(by_tag, |i| i.map_or(J::Null, int))),
        ExprKind::WorldState { recv, state } => node("world_state").set("recv", expr(recv)).set("state", int(state.0)),
        ExprKind::EventArg { recv, name, by_rule } => node("event_arg")
            .set("recv", expr(recv))
            .set("name", s(name))
            .set("by_rule", arr(by_rule, |(r, i)| J::obj().set("rule", int(r.0)).set("index", i.map_or(J::Null, int)))),
        ExprKind::With { recv, ty, changes } => node("with")
            .set("recv", expr(recv))
            .set("type", int(ty.0))
            .set("changes", arr(changes, |(i, x)| J::obj().set("index", int(*i)).set("value", expr(x)))),
        ExprKind::Is { value, pat: p } => node("is").set("value", expr(value)).set("pattern", pat(p)),
        ExprKind::If { cond, then, els } => {
            node("if").set("cond", expr(cond)).set("then", expr(then)).set("else", expr(els))
        }
        ExprKind::Logic { op, left, right } => {
            node("logic").set("op", s(op.name())).set("left", expr(left)).set("right", expr(right))
        }
        ExprKind::Compare { first, links } => node("compare")
            .set("first", expr(first))
            .set("links", arr(links, |(op, x)| J::obj().set("op", s(op.name())).set("right", expr(x)))),
        ExprKind::Else { value, fallback, unwrap } => {
            node("else").set("value", expr(value)).set("fallback", expr(fallback)).set("unwrap", J::Bool(*unwrap))
        }
        ExprKind::Fail { name, args } => node("fail").set("name", s(name)).set("args", arr(args, expr)),
        ExprKind::Match { scrutinee, arms } => node("match")
            .set("scrutinee", expr(scrutinee))
            .set("arms", arr(arms, |a| J::obj().set("pattern", pat(&a.pat)).set("value", expr(&a.value)))),
        ExprKind::Block(b) => node("block").set("block", block(b)),
        ExprKind::Hole(h) => node("hole").set("name", s(h)),
        ExprKind::Tuple(xs) => node("tuple").set("items", arr(xs, expr)),
        ExprKind::List(xs) => node("list").set("items", arr(xs, expr)),
        ExprKind::SetOf(xs) => node("set").set("items", arr(xs, expr)),
        ExprKind::MapOf(ps) => {
            node("map").set("entries", arr(ps, |(k, v)| J::obj().set("key", expr(k)).set("value", expr(v))))
        }
        ExprKind::Range { lo, hi, inclusive } => {
            node("range").set("lo", expr(lo)).set("hi", expr(hi)).set("inclusive", J::Bool(*inclusive))
        }
    };
    o.set("ty", lty(&e.ty))
}

fn expected(e: &Expected) -> J {
    match e {
        Expected::Value(x) => node("value").set("value", expr(x)),
        Expected::Fail { name, args } => node("fail").set("name", s(name)).set("args", arr(args, expr)),
    }
}

fn place(p: &Place) -> J {
    let root = match &p.root {
        PlaceRoot::State(st) => node("state").set("state", int(st.0)),
        PlaceRoot::Local(n) => node("local").set("name", s(n)),
    };
    J::obj()
        .set("root", root)
        .set(
            "path",
            arr(&p.path, |st| match st {
                PlaceStep::Field { ty, index } => node("field").set("type", int(ty.0)).set("index", int(*index)),
                PlaceStep::MapKey(k) => node("map_key").set("key", expr(k)),
                PlaceStep::ListIndex(k) => node("list_index").set("index", expr(k)),
            }),
        )
        .set("ty", lty(&p.ty))
        .set("check", p.check.as_ref().map_or(J::Null, dty))
}

pub fn block(b: &Block) -> J {
    J::obj().set("ty", lty(&b.ty)).set("stmts", arr(&b.stmts, stmt))
}

fn stmt(st: &Stmt) -> J {
    match &st.kind {
        StmtKind::Let { name, mutable, ty, check, value } => node("let")
            .set("name", s(name))
            .set("mutable", J::Bool(*mutable))
            .set("ty", lty(ty))
            .set("check", check.as_ref().map_or(J::Null, dty))
            .set("value", expr(value)),
        StmtKind::Assign { place: p, value } => node("assign").set("place", place(p)).set("value", expr(value)),
        StmtKind::Mutate { place: p, op, args } => {
            node("mutate").set("place", place(p)).set("op", s(op.name())).set("args", arr(args, expr))
        }
        StmtKind::Expr(e) => node("expr").set("value", expr(e)),
        StmtKind::Return(v) => node("return").set("value", opt(v)),
        StmtKind::Assert(e) => node("assert").set("value", expr(e)),
        StmtKind::For { var, ty, iter, body } => {
            node("for").set("var", s(var)).set("ty", lty(ty)).set("iter", expr(iter)).set("body", block(body))
        }
        StmtKind::While { cond, body } => node("while").set("cond", expr(cond)).set("body", block(body)),
        StmtKind::If { cond, then, els } => {
            node("if").set("cond", expr(cond)).set("then", block(then)).set("else", els.as_ref().map_or(J::Null, block))
        }
        StmtKind::Expect { subject, expect } => {
            node("expect").set("subject", expr(subject)).set("expect", expected(expect))
        }
    }
}

fn info(i: &Info) -> J {
    J::obj()
        .set("module", int(i.module.0))
        .set("name", s(&i.name))
        .set("qualified", s(&i.qualified))
        .set("pub", J::Bool(i.is_pub))
        .set("why", i.why.as_ref().map_or(J::Null, s))
}

fn params(ps: &[Param]) -> J {
    arr(ps, |p| {
        J::obj()
            .set("name", s(&p.name))
            .set("ty", lty(&p.ty))
            .set("check", dty(&p.check))
            .set("default", opt(&p.default))
    })
}

fn body(b: &Body) -> J {
    match b {
        Body::Expr(e) => node("expr").set("value", expr(e)),
        Body::Extern(x) => node("extern")
            .set("lang", s(&x.lang))
            .set("library", s(&x.library))
            .set("symbol", s(&x.symbol))
            .set("effects", arr(&x.effects, |e| s(e))),
    }
}

fn note_value(v: &NoteValue) -> J {
    match v {
        NoteValue::Int(i) => node("int").set("value", int(*i)),
        NoteValue::Text(t) => node("text").set("value", s(t)),
        NoteValue::Quantity { value, unit } => node("quantity").set("value", int(*value)).set("unit", s(unit)),
        NoteValue::Variant { name, args } => node("variant").set("name", s(name)).set("args", note_args(args)),
    }
}

fn note_args(args: &[NoteArg]) -> J {
    arr(args, |a| J::obj().set("param", s(&a.param)).set("value", note_value(&a.value)))
}

pub fn note(n: &Note) -> J {
    J::obj().set("library", s(&n.library)).set("name", s(&n.name)).set("args", note_args(&n.args))
}

fn fields(fs: &[Field]) -> J {
    arr(fs, |f| {
        J::obj()
            .set("name", s(&f.name))
            .set("ty", lty(&f.ty))
            .set("check", dty(&f.check))
            .set("default", opt(&f.default))
    })
}

/// One app's manifest, its rules spelled out.
pub fn manifest(lir: &Lir, m: &Manifest) -> J {
    J::obj()
        .set("name", s(&m.name))
        .set("qualified", s(&m.qualified))
        .set("module", s(&m.module))
        .set("target", m.target.as_ref().map_or(J::Null, s))
        .set("notes", arr(&m.notes, note))
        .set(
            "params",
            arr(&m.params, |p| {
                J::obj().set("name", s(&p.name)).set(
                    "decls",
                    arr(&p.decls, |d| match d {
                        Named::Rule(r) => {
                            node("rule").set("id", int(r.0)).set("qualified", s(&lir.rule(*r).info.qualified))
                        }
                        Named::Fn(f) => {
                            node("fn").set("id", int(f.0)).set("qualified", s(&lir.func(*f).info.qualified))
                        }
                    }),
                )
            }),
        )
        .set(
            "rules",
            arr(&m.rules, |r| {
                let rule = lir.rule(*r);
                J::obj()
                    .set("id", int(r.0))
                    .set("qualified", s(&rule.info.qualified))
                    .set("pub", J::Bool(rule.info.is_pub))
                    .set("params", params(&rule.params))
                    .set("result", lty(&rule.result))
                    .set("triggers", arr(&rule.triggers, note))
                    .set("why", rule.info.why.as_ref().map_or(J::Null, s))
                    .set("uses", arr(&rule.uses, |u| s(u)))
                    .set(
                        "extern",
                        match &rule.body {
                            Body::Extern(_) => body(&rule.body),
                            Body::Expr(_) => J::Null,
                        },
                    )
            }),
        )
}

/// The whole program.
pub fn program(lir: &Lir) -> J {
    J::obj()
        .set("option_type", int(lir.option_type.0))
        .set("result_type", int(lir.result_type.0))
        .set(
            "modules",
            arr(&lir.modules, |m| {
                J::obj()
                    .set("id", int(m.id.0))
                    .set("name", s(&m.name))
                    .set("path", s(&m.path))
                    .set("vocabularies", arr(&m.vocabularies, |v| s(v)))
            }),
        )
        .set(
            "types",
            arr(&lir.types, |t| {
                let def = match &t.def {
                    TypeDef::Alias { of } => node("alias").set("of", dty(of)),
                    TypeDef::Record { fields: fs } => node("record").set("fields", fields(fs)),
                    TypeDef::Sum { variants } => node("sum").set(
                        "variants",
                        arr(variants, |v| J::obj().set("name", s(&v.name)).set("fields", fields(&v.fields))),
                    ),
                    TypeDef::RuleSet { rules: rs } => node("rule_set").set("rules", rules(rs)),
                };
                J::obj().set("info", info(&t.info)).set("builtin", J::Bool(t.builtin)).set("def", def)
            }),
        )
        .set(
            "claims",
            arr(&lir.claims, |c| {
                J::obj()
                    .set("module", int(c.module.0))
                    .set("self", lty(&c.self_ty))
                    .set("body", expr(&c.body))
                    .set("text", s(&c.text))
            }),
        )
        .set(
            "consts",
            arr(&lir.consts, |c| {
                J::obj()
                    .set("info", info(&c.info))
                    .set("ty", lty(&c.ty))
                    .set("declared", c.declared.as_ref().map_or(J::Null, dty))
                    .set("value", expr(&c.value))
            }),
        )
        .set(
            "states",
            arr(&lir.states, |st| {
                J::obj()
                    .set("info", info(&st.info))
                    .set("ty", lty(&st.ty))
                    .set("check", dty(&st.check))
                    .set("init", opt(&st.init))
            }),
        )
        .set(
            "fns",
            arr(&lir.fns, |f| {
                J::obj()
                    .set("info", info(&f.info))
                    .set("params", params(&f.params))
                    .set("result", lty(&f.result))
                    .set("result_check", f.result_check.as_ref().map_or(J::Null, dty))
                    .set("uses", arr(&f.uses, |u| s(u)))
                    .set("body", body(&f.body))
            }),
        )
        .set(
            "rules",
            arr(&lir.rules, |r| {
                J::obj()
                    .set("info", info(&r.info))
                    .set("params", params(&r.params))
                    .set("result", lty(&r.result))
                    .set("result_check", r.result_check.as_ref().map_or(J::Null, dty))
                    .set("guards", arr(&r.guards, |g| J::obj().set("cond", expr(&g.cond)).set("error", s(&g.error))))
                    .set("uses", arr(&r.uses, |u| s(u)))
                    .set("triggers", arr(&r.triggers, note))
                    .set("body", body(&r.body))
            }),
        )
        .set(
            "tests",
            arr(&lir.tests, |t| {
                J::obj()
                    .set("info", info(&t.info))
                    .set("params", params(&t.params))
                    .set(
                        "given",
                        arr(&t.given, |g| match g {
                            Given::State { state, value } => {
                                node("state").set("state", int(state.0)).set("value", expr(value))
                            }
                            Given::Test(id) => node("test").set("test", int(id.0)),
                        }),
                    )
                    .set("body", block(&t.body))
            }),
        )
        .set(
            "laws",
            arr(&lir.laws, |l| {
                J::obj()
                    .set("info", info(&l.info))
                    .set("params", params(&l.params))
                    .set("trace", int(i128::try_from(l.trace).unwrap_or(0)))
                    .set("traced", rules(&l.traced))
                    .set("start_rules", rules(&l.start_rules))
                    .set("body", expr(&l.body))
            }),
        )
        .set(
            "checks",
            arr(&lir.checks, |c| {
                let what = match &c.what {
                    CheckWhat::Case { target, args, expect } => node("case")
                        .set(
                            "target",
                            match target {
                                CaseTarget::Fn(f) => node("fn").set("id", int(f.0)),
                                CaseTarget::Test(t) => node("test").set("id", int(t.0)),
                            },
                        )
                        .set("args", arr(args, opt))
                        .set("expect", expected(expect)),
                    CheckWhat::Test(t) => node("test").set("test", int(t.0)),
                    CheckWhat::Property(t) => node("property").set("test", int(t.0)),
                    CheckWhat::Law(l) => node("law").set("law", int(l.0)),
                };
                J::obj()
                    .set("check", s(&c.name))
                    .set("module", s(&c.module))
                    .set("kind", s(c.kind.as_str()))
                    .set("decl", s(&c.decl))
                    .set("what", what)
            }),
        )
        .set("apps", arr(&lir.apps, |m| manifest(lir, m)))
}
