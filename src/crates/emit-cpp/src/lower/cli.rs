//! The `cli` vocabulary's derived surface for this app: the value types its
//! rules take, the parameter table, the help text, and the binding of parsed
//! values to events. The parser that reads the table is the runtime shim `rt/cli.hpp`.

use spec_lang_emit_rust::front::cli::{self as fcli, FlagKind};
use spec_lang_emit_rust::front::typed::T;
use spec_lang_emit_rust::provenance::Origin;
use spec_lang_ir::TypeDef;

use super::expr::{Env, R};
use super::{Gen, camel, ident, snake};
use crate::ast::*;
use crate::format::Unit;

fn value_name(g: &Gen<'_, '_>, t: &T) -> String {
    match t {
        T::Named(id) => g.f.prog.ty(*id).info.name.clone(),
        T::Int => "Int".into(),
        T::Text => "Text".into(),
        T::Bool => "Bool".into(),
        _ => "Value".into(),
    }
}

/// `include/<ns>/cli.hpp`.
pub fn header(g: &Gen<'_, '_>, board: &str) -> R<Unit> {
    let app = g.f.app();
    let mut unit = Unit::new(
        format!("include/{}/cli.hpp", g.ns),
        g.header(&g.f.cli.iter().map(|c| (format!("the parameters of rule {}", c.name), c.span)).collect::<Vec<_>>()),
        g.origin(&app.info, "file header"),
    );
    unit.include(format!("{}/{board}.hpp", g.ns));
    for m in g.mods.values() {
        if m != board {
            unit.include(format!("{}/{m}.hpp", g.ns));
        }
    }
    unit.include("rt/cli.hpp");
    let types = fcli::value_types(&g.f.cli);
    let names: Vec<String> = types.iter().map(|t| value_name(g, t)).collect();
    let mut items =
        vec![unit.mark(g.origin(&app.info, "cli value types: every parameter type a command line carries"))];
    items.push(Decl::Enum {
        doc: vec!["The parameter types a command line carries, each checked at the edge.".into()],
        name: "ValueType".into(),
        underlying: Ty::name("std::uint8_t"),
        variants: names.clone(),
    });
    items.push(Decl::Using {
        name: "Value".into(),
        ty: Ty::generic("std::variant", types.iter().map(|t| g.cty(t)).collect()),
    });
    items.push(Decl::Struct {
        doc: vec!["The value types as the parser in rt/cli.hpp reads them.".into()],
        keyword: "struct",
        name: "Values".into(),
        members: vec![
            Member::Func(Func {
                doc: vec![],
                prefix: vec!["[[nodiscard]]", "static"],
                ret: Ty::generic("std::optional", vec![Ty::name("cli::Value")]),
                name: "parse".into(),
                params: vec![
                    Param { ty: Ty::name("ValueType"), name: "type".into(), default: None },
                    Param { ty: Ty::name("std::string_view"), name: "text".into(), default: None },
                ],
                suffix: vec![],
                body: None,
            }),
            Member::Func(Func {
                doc: vec!["The type's name as the spec writes it, for \"not a ...\".".into()],
                prefix: vec!["[[nodiscard]]", "static", "constexpr"],
                ret: Ty::name("std::string_view"),
                name: "name".into(),
                params: vec![Param { ty: Ty::name("ValueType"), name: "type".into(), default: None }],
                suffix: vec!["noexcept"],
                body: Some(vec![
                    Stmt::Switch {
                        on: Expr::id("type"),
                        cases: names
                            .iter()
                            .map(|n| {
                                (Expr::id(format!("ValueType::{n}")), vec![Stmt::Return(Some(Expr::str(n.clone())))])
                            })
                            .collect(),
                    },
                    Stmt::Expr(Expr::call("std::unreachable", vec![])),
                ]),
            }),
        ],
    });
    if let Some(Decl::Struct { members, .. }) = items.last_mut() {
        members.insert(0, Member::Using { name: "Value".into(), ty: Ty::name("cli::Value") });
        members.insert(0, Member::Using { name: "Type".into(), ty: Ty::name("ValueType") });
    }
    let vt = |t: &T| Expr::id(format!("ValueType::{}", value_name(g, t)));
    let mut table = Vec::new();
    let mut commands = Vec::new();
    for c in &g.f.cli {
        let info = &g.f.prog.rule(c.rule).info;
        let name = snake(&c.name);
        let pos_name = format!("{name}_positionals");
        let flag_name = format!("{name}_flags");
        let mut row = vec![Expr::str(c.name.clone())];
        if !c.positionals.is_empty() {
            table.push(unit.mark(g.origin(info, "cli param table row: positionals").strategy(
                "a parameter without a default is a positional, in declaration order",
                "cli vocabulary, Derived: a parameter without a default",
            )));
            let rows: Vec<Expr> =
                c.positionals.iter().map(|p| Expr::Brace(None, vec![Expr::str(p.name.clone()), vt(&p.ty)])).collect();
            table.push(Decl::Var {
                doc: vec![],
                prefix: vec!["inline", "constexpr"],
                ty: Ty::name(format!("std::array<rt::cli::Positional<ValueType>, {}>", rows.len())),
                name: pos_name.clone(),
                init: Expr::Brace(None, vec![Expr::Brace(None, rows)]),
            });
            row.push(Expr::id(format!("table::{pos_name}")));
        } else {
            row.push(Expr::Brace(None, vec![]));
        }
        if !c.flags.is_empty() {
            let derivations: Vec<&str> = c.flags.iter().map(|f| f.derivation).collect();
            table.push(
                unit.mark(
                    g.origin(info, "cli param table row: flags")
                        .strategy(derivations.join("; "), "cli vocabulary, Derived, never written"),
                ),
            );
            let rows: Vec<Expr> = c
                .flags
                .iter()
                .map(|f| {
                    let (kind, t) = match &f.kind {
                        FlagKind::Switch => ("Switch", None),
                        FlagKind::Single(t) => ("Single", Some(t)),
                        FlagKind::Set(t) => ("Repeated", Some(t)),
                    };
                    Expr::Brace(
                        None,
                        vec![
                            Expr::str(f.name.clone()),
                            Expr::id(format!("rt::cli::FlagKind::{kind}")),
                            t.map_or(Expr::Brace(None, vec![]), vt),
                        ],
                    )
                })
                .collect();
            table.push(Decl::Var {
                doc: vec![],
                prefix: vec!["inline", "constexpr"],
                ty: Ty::name(format!("std::array<rt::cli::Flag<ValueType>, {}>", rows.len())),
                name: flag_name.clone(),
                init: Expr::Brace(None, vec![Expr::Brace(None, rows)]),
            });
            row.push(Expr::id(format!("table::{flag_name}")));
        } else {
            row.push(Expr::Brace(None, vec![]));
        }
        commands.push(Expr::Brace(None, row));
    }
    items.push(Decl::Namespace("table".into(), table));
    items.push(unit.mark(g.origin(&app.info, "cli command table: every @on(cli(...)) rule, in the app's order")));
    items.push(Decl::Var {
        doc: vec!["Every cli rule in the app's order, which is the order of --help.".into()],
        prefix: vec!["inline", "constexpr"],
        ty: Ty::name(format!("std::array<rt::cli::CommandSpec<ValueType>, {}>", commands.len())),
        name: "commands".into(),
        init: Expr::Brace(None, vec![Expr::Brace(None, commands)]),
    });
    let bin = app.bin.clone().unwrap_or_else(|| g.ns.clone());
    items.push(unit.mark(g.origin(&app.info, "help text").strategy(
        "folded at build time from the table",
        "cli vocabulary: every cli rule is a line of --help; a sum shows its variants, anything else <type name>",
    )));
    items.push(Decl::Var {
        doc: vec!["The help text (contract section 2).".into()],
        prefix: vec!["inline", "constexpr"],
        ty: Ty::name("std::string_view"),
        name: "help_text".into(),
        init: Expr::str(fcli::help(g.f, &bin)),
    });
    items.push(unit.mark(g.origin(&app.info, "binding of parsed values to an event")));
    items.push(Decl::Func(Func {
        doc: vec!["The values of one command, already of their types, as the event of its rule.".into()],
        prefix: vec!["[[nodiscard]]"],
        ret: Ty::name(format!("{board}::Event")),
        name: "bind".into(),
        params: vec![Param { ty: Ty::name("rt::cli::Bound<Value>"), name: "bound".into(), default: None }],
        suffix: vec![],
        body: None,
    }));
    unit.decls.push(Decl::Namespace(format!("{}::cli", g.ns), items));
    Ok(unit)
}

/// `src/cli.cpp`: parsing each value type, and binding.
pub fn source(g: &Gen<'_, '_>, board: &str) -> R<Unit> {
    let app = g.f.app();
    let mut unit = Unit::new(
        "src/cli.cpp",
        g.header(&g.f.cli.iter().map(|c| (format!("the parameters of rule {}", c.name), c.span)).collect::<Vec<_>>()),
        g.origin(&app.info, "file header"),
    );
    unit.include(format!("{}/cli.hpp", g.ns));
    unit.include("rt/unicode.hpp");
    let types = fcli::value_types(&g.f.cli);
    let mut parsers = Vec::new();
    for t in &types {
        let name = value_name(g, t);
        parsers.push(unit.mark(Origin::new(
            format!("{} value type {name}", g.path(&app.info)),
            g.at(app.info.span),
            "cli value parser",
        )));
        parsers.push(Decl::Func(parser(g, t)?));
    }
    let mut cases = Vec::new();
    for t in &types {
        cases.push((
            Expr::id(format!("ValueType::{}", value_name(g, t))),
            vec![Stmt::Return(Some(Expr::CallT(
                Box::new(Expr::id("rt::cli::as_value")),
                vec![Ty::name("Value")],
                vec![Expr::call(format!("parse_{}", snake(&value_name(g, t))), vec![Expr::id("text")])],
            )))],
        ));
    }
    let mut items = vec![Decl::Namespace(String::new(), parsers)];
    items.push(unit.mark(g.origin(&app.info, "cli value parser")));
    items.push(Decl::Func(Func {
        doc: vec![],
        prefix: vec![],
        ret: Ty::generic("std::optional", vec![Ty::name("Value")]),
        name: "Values::parse".into(),
        params: vec![
            Param { ty: Ty::name("ValueType"), name: "type".into(), default: None },
            Param { ty: Ty::name("std::string_view"), name: "text".into(), default: None },
        ],
        suffix: vec![],
        body: Some(vec![
            Stmt::Switch { on: Expr::id("type"), cases },
            Stmt::Expr(Expr::call("std::unreachable", vec![])),
        ]),
    }));
    // bind
    let mut cases = Vec::new();
    for (i, c) in g.f.cli.iter().enumerate() {
        let r = g.f.rule(c.rule).ok_or_else(|| g.unsupported(c.span, &app.info, "a cli rule outside the app"))?;
        let info = &g.f.prog.rule(c.rule).info;
        let mut fields = Vec::new();
        for (pi, p) in r.params.iter().enumerate() {
            let b = Expr::id("bound");
            let v = if let Some(k) = c.positionals.iter().position(|x| x.param == pi) {
                let at = Expr::Call(Box::new(b.member("positionals").member("at")), vec![Expr::Int(k as i128)]);
                Expr::CallT(Box::new(Expr::id("std::get")), vec![g.cty(&p.ty)], vec![at.moved()])
            } else if let Some(k) = c.flags.iter().position(|x| x.param == pi) {
                let f = &c.flags[k];
                let vals =
                    Expr::Call(Box::new(b.clone().member("flag_values").member("at")), vec![Expr::Int(k as i128)]);
                match &f.kind {
                    FlagKind::Switch => {
                        Expr::Call(Box::new(b.member("flag_seen").member("at")), vec![Expr::Int(k as i128)])
                    }
                    FlagKind::Set(t) => Expr::CallT(Box::new(Expr::id("rt::cli::all")), vec![g.cty(t)], vec![vals]),
                    FlagKind::Single(t) if matches!(p.ty, T::Option(_)) => {
                        Expr::CallT(Box::new(Expr::id("rt::cli::first")), vec![g.cty(t)], vec![vals])
                    }
                    FlagKind::Single(t) => {
                        let d = match &p.default {
                            Some(d) => Env::new(g, info).expr(d)?,
                            None => return Err(g.unsupported(p.span, info, "a flag without a default")),
                        };
                        Expr::CallT(Box::new(Expr::id("rt::cli::first_or")), vec![g.cty(t)], vec![vals, d])
                    }
                }
            } else {
                return Err(g.unsupported(p.span, info, "a parameter the cli table has no row for"));
            };
            fields.push((ident(&p.name), v));
        }
        let event = Ty::name(format!("{board}::{}", camel(&info.name)));
        let value = if fields.is_empty() { Expr::Brace(Some(event), vec![]) } else { Expr::Designated(event, fields) };
        cases.push((Expr::Int(i as i128), vec![Stmt::Return(Some(value))]));
    }
    items.push(unit.mark(g.origin(&app.info, "binding of parsed values to an event")));
    items.push(Decl::Func(Func {
        doc: vec!["The parser checked every count and type; each value is taken as its field's type.".into()],
        prefix: vec![],
        ret: Ty::name(format!("{board}::Event")),
        name: "bind".into(),
        params: vec![Param { ty: Ty::name("rt::cli::Bound<Value>"), name: "bound".into(), default: None }],
        suffix: vec![],
        body: Some(vec![
            Stmt::Switch { on: Expr::id("bound").member("command"), cases },
            Stmt::Expr(Expr::call("std::unreachable", vec![])),
        ]),
    }));
    unit.decls.push(Decl::Namespace(format!("{}::cli", g.ns), items));
    Ok(unit)
}

/// `parse_<type>(text)`: the type's text form, then its claim (contract section 2, the Accepts table).
fn parser(g: &Gen<'_, '_>, t: &T) -> R<Func> {
    let name = value_name(g, t);
    let ty = g.cty(t);
    let none = || Stmt::Return(Some(Expr::id("std::nullopt")));
    let text = Expr::id("text");
    let (doc, body) = match t {
        T::Named(id) => match (&g.f.prog.ty(*id).def, g.alias_base(*id)) {
            (TypeDef::Sum(vs), _) => {
                let mut b = Vec::new();
                for v in vs {
                    b.push(Stmt::If {
                        cond: Expr::bin("==", text.clone(), Expr::str(v.name.to_lowercase())),
                        then: vec![Stmt::Return(Some(Expr::id(format!("{}::{}", g.type_path(*id), v.name))))],
                        els: vec![],
                    });
                }
                b.push(none());
                ("A sum with no data is its variant names in lower case, exactly.", b)
            }
            (_, Some(T::Int)) => (
                "ASCII digits only, one or more; leading zeros allowed. A value past 64 bits saturates: no store holds a number above 2^53, so it is in no world either.",
                digits(Expr::call(format!("{}::make", g.type_path(*id)), vec![Expr::id("value")])),
            ),
            (_, Some(T::Text)) => (
                "Text is Unicode: bytes that are not UTF-8 are no Text, so not this type either.",
                vec![
                    Stmt::If {
                        cond: Expr::call("rt::unicode::is_valid_utf8", vec![text.clone()]).negate(),
                        then: vec![none()],
                        els: vec![],
                    },
                    Stmt::Return(Some(Expr::call(
                        format!("{}::make", g.type_path(*id)),
                        vec![Expr::call("std::string", vec![text.clone()])],
                    ))),
                ],
            ),
            _ => {
                return Err(g.unsupported(
                    g.f.app().info.span,
                    &g.f.app().info,
                    format!("a cli parameter of type {name}"),
                ));
            }
        },
        T::Text => (
            "Text is Unicode: bytes that are not UTF-8 are no Text.",
            vec![
                Stmt::If {
                    cond: Expr::call("rt::unicode::is_valid_utf8", vec![text.clone()]).negate(),
                    then: vec![none()],
                    els: vec![],
                },
                Stmt::Return(Some(Expr::call("std::string", vec![text]))),
            ],
        ),
        T::Int => ("ASCII digits only, one or more.", digits(Expr::id("value"))),
        _ => {
            return Err(g.unsupported(g.f.app().info.span, &g.f.app().info, format!("a cli parameter of type {name}")));
        }
    };
    Ok(Func {
        doc: vec![doc.into()],
        prefix: vec!["[[nodiscard]]"],
        ret: Ty::generic("std::optional", vec![ty]),
        name: format!("parse_{}", snake(&name)),
        params: vec![Param { ty: Ty::name("std::string_view"), name: "text".into(), default: None }],
        suffix: vec![],
        body: Some(body),
    })
}

fn digits(result: Expr) -> Vec<Stmt> {
    let max = Expr::id("std::numeric_limits<std::int64_t>::max()");
    vec![
        Stmt::If {
            cond: Expr::id("text").method("empty", vec![]),
            then: vec![Stmt::Return(Some(Expr::id("std::nullopt")))],
            els: vec![],
        },
        Stmt::Var { ty: Ty::name("std::int64_t"), name: "value".into(), init: Some(Expr::Int(0)) },
        Stmt::For {
            ty: Ty::name("char").konst(),
            name: "c".into(),
            range: Expr::id("text"),
            body: vec![
                Stmt::If {
                    cond: Expr::bin(
                        "||",
                        Expr::bin("<", Expr::id("c"), Expr::id("'0'")),
                        Expr::bin(">", Expr::id("c"), Expr::id("'9'")),
                    ),
                    then: vec![Stmt::Return(Some(Expr::id("std::nullopt")))],
                    els: vec![],
                },
                Stmt::Var {
                    ty: Ty::Auto.konst(),
                    name: "digit".into(),
                    init: Some(Expr::Cast(
                        Ty::name("std::int64_t"),
                        Box::new(Expr::bin("-", Expr::id("c"), Expr::id("'0'"))),
                    )),
                },
                Stmt::Expr(Expr::bin(
                    "=",
                    Expr::id("value"),
                    Expr::Cond(
                        Box::new(Expr::bin(
                            ">",
                            Expr::id("value"),
                            Expr::bin("/", Expr::bin("-", max.clone(), Expr::id("digit")), Expr::Int(10)),
                        )),
                        Box::new(max),
                        Box::new(Expr::bin("+", Expr::bin("*", Expr::id("value"), Expr::Int(10)), Expr::id("digit"))),
                    ),
                )),
            ],
        },
        Stmt::Return(Some(result)),
    ]
}
