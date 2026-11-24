//! The store codec, generated from the states' types (docs/contract.md section 3):
//! a reader that takes exactly the schema and a canonical writer.
//!
//! The mapping, fixed per backend: the document is an object with `version` (1)
//! and one key per state, states sorted by name; a record is an object with its
//! fields in declaration order; a sum without data is its variant's name; a
//! `Set` is an array in ascending order, a repeat rejected; a `Map` whose values
//! sit under their own key field (`facts.keyed`) is the array of its values in
//! key order; an `Int` is an integer from 0 to 2^53, and a world holding a larger
//! one is `full`. Every law that holds of every state (`facts.invariants`) is
//! checked on a world read from outside.

use std::collections::BTreeMap;

use spec_lang_emit_rust::front::typed::*;
use spec_lang_emit_rust::front::{TState, record_fields};
use spec_lang_emit_rust::provenance::Origin;
use spec_lang_ir::TypeDef;

use super::expr::{Env, Local, R};
use super::{Gen, ident, snake};
use crate::ast::*;
use crate::format::Unit;

struct Codec<'a, 'f, 'p> {
    g: &'a Gen<'f, 'p>,
    /// Reader and writer per type, by name, in dependency order.
    funcs: Vec<(String, Func, Func, String)>,
    done: BTreeMap<String, ()>,
}

fn none() -> Stmt {
    Stmt::Return(Some(Expr::id("std::nullopt")))
}

fn r() -> Expr {
    Expr::id("r")
}

fn out() -> Expr {
    Expr::id("out")
}

fn append(text: &str) -> Stmt {
    Stmt::Expr(Expr::bin("+=", out(), Expr::str(text)))
}

fn put_char(c: char) -> Stmt {
    Stmt::Expr(out().method("push_back", vec![Expr::Id(format!("'{c}'"))]))
}

impl Codec<'_, '_, '_> {
    fn key(&self, t: &T) -> String {
        match t {
            T::Named(id) => snake(&self.g.f.prog.ty(*id).info.name),
            T::Int => "int".into(),
            T::Text => "text".into(),
            T::Bool => "bool".into(),
            T::Set(x) => format!("set_{}", self.key(x)),
            T::Map(_, v) => format!("map_{}", self.key(v)),
            _ => "value".into(),
        }
    }

    /// Makes sure a reader and writer for `t` exist; returns its name.
    fn need(&mut self, t: &T) -> R<String> {
        let name = self.key(t);
        if self.done.contains_key(&name) {
            return Ok(name);
        }
        self.done.insert(name.clone(), ());
        let g = self.g;
        let app = &g.f.app().info;
        let ty = g.cty(t);
        let v = Expr::id("v");
        let (read, write, role): (Vec<Stmt>, Vec<Stmt>, String) = match t {
            T::Named(id) => {
                let d = g.f.prog.ty(*id);
                match (&d.def, g.alias_base(*id)) {
                    (TypeDef::Sum(_), _) => (
                        vec![
                            Stmt::Var {
                                ty: Ty::Auto.konst(),
                                name: "s".into(),
                                init: Some(r().method("string", vec![])),
                            },
                            Stmt::If { cond: Expr::id("s").negate(), then: vec![none()], els: vec![] },
                            Stmt::Return(Some(Expr::call(
                                format!("{}::{}_named", g.module_of(&d.info), snake(&d.info.name)),
                                vec![Expr::id("s").deref()],
                            ))),
                        ],
                        vec![
                            Stmt::Expr(Expr::call(
                                "rt::json::put_string",
                                vec![out(), Expr::call(format!("{}::name", g.module_of(&d.info)), vec![v.clone()])],
                            )),
                            Stmt::Return(Some(Expr::Bool(true))),
                        ],
                        format!("store codec: sum {} as its variant's name", d.info.name),
                    ),
                    (_, Some(T::Int)) => (
                        int_read(Expr::call(format!("{}::make", g.type_path(*id)), vec![Expr::id("value")])),
                        vec![Stmt::Return(Some(Expr::call(
                            "rt::json::put_int",
                            vec![out(), v.method("value", vec![])],
                        )))],
                        format!("store codec: {} as an integer, then its claim", d.info.name),
                    ),
                    (_, Some(T::Text)) => (
                        vec![
                            Stmt::Var {
                                ty: Ty::Auto.konst(),
                                name: "s".into(),
                                init: Some(r().method("string", vec![])),
                            },
                            Stmt::If { cond: Expr::id("s").negate(), then: vec![none()], els: vec![] },
                            Stmt::Return(Some(Expr::call(
                                format!("{}::make", g.type_path(*id)),
                                vec![Expr::call("std::string", vec![Expr::id("s").deref()])],
                            ))),
                        ],
                        vec![
                            Stmt::Expr(Expr::call("rt::json::put_string", vec![out(), v.method("text", vec![])])),
                            Stmt::Return(Some(Expr::Bool(true))),
                        ],
                        format!("store codec: {} as a string, then its claim", d.info.name),
                    ),
                    (TypeDef::Record(fields), _) => {
                        let mut tys = Vec::new();
                        let typer = spec_lang_emit_rust::front::typer::Typer::new(g.f.prog, g.path(&d.info));
                        for f in fields {
                            tys.push((f, typer.ty(&f.ty, f.span)?));
                        }
                        let mut read = Vec::new();
                        let branches: Option<Stmt> = None;
                        let mut write = Vec::new();
                        for (i, (f, fty)) in tys.iter().enumerate() {
                            let sub = self.need(fty)?;
                            read.push(Stmt::Var {
                                ty: Ty::generic("std::optional", vec![g.cty(fty)]),
                                name: ident(&f.name),
                                init: None,
                            });
                            let _ = &branches;
                            write.push(append(&format!("{}\"{}\":", if i == 0 { "{" } else { "," }, f.name)));
                            write.push(Stmt::If {
                                cond: Expr::call(format!("put_{sub}"), vec![out(), v.clone().member(&ident(&f.name))])
                                    .negate(),
                                then: vec![Stmt::Return(Some(Expr::Bool(false)))],
                                els: vec![],
                            });
                        }
                        if tys.is_empty() {
                            write.push(append("{"));
                        }
                        write.push(put_char('}'));
                        write.push(Stmt::Return(Some(Expr::Bool(true))));
                        // One branch per key, in declaration order.
                        let mut chain: Vec<Stmt> = vec![Stmt::Return(Some(Expr::Bool(false)))];
                        for (f, fty) in tys.iter().rev() {
                            let sub = self.key(fty);
                            let fillit = Stmt::Return(Some(Expr::call(
                                "rt::json::fill",
                                vec![
                                    Expr::id(ident(&f.name)),
                                    Expr::Lambda {
                                        capture: "&",
                                        params: vec![],
                                        ret: None,
                                        body: vec![Stmt::Return(Some(Expr::call(format!("read_{sub}"), vec![r()])))],
                                    },
                                ],
                            )));
                            chain = vec![Stmt::If {
                                cond: Expr::bin("==", Expr::id("key"), Expr::str(f.name.clone())),
                                then: vec![fillit],
                                els: chain,
                            }];
                        }
                        let member = Expr::Lambda {
                            capture: "&",
                            params: vec![(Ty::name("std::string_view"), "key".into())],
                            ret: None,
                            body: chain,
                        };
                        read.push(Stmt::If {
                            cond: r().method("object", vec![member]).negate(),
                            then: vec![none()],
                            els: vec![],
                        });
                        let all =
                            tys.iter().map(|(f, _)| Expr::id(ident(&f.name))).reduce(|a, b| Expr::bin("&&", a, b));
                        if let Some(all) = all {
                            read.push(Stmt::If { cond: all.negate(), then: vec![none()], els: vec![] });
                        }
                        let build = Expr::Designated(
                            ty.clone(),
                            tys.iter()
                                .map(|(f, fty)| {
                                    let x = Expr::id(ident(&f.name)).deref();
                                    (ident(&f.name), if g.trivial(fty) { x } else { x.moved() })
                                })
                                .collect(),
                        );
                        read.push(Stmt::Return(Some(build)));
                        (
                            read,
                            write,
                            format!("store codec: record {} as an object with exactly its fields", d.info.name),
                        )
                    }
                    _ => return Err(g.unsupported(d.info.span, &d.info, "a type the store format has no form for")),
                }
            }
            T::Int => (
                int_read(Expr::id("value")),
                vec![Stmt::Return(Some(Expr::call("rt::json::put_int", vec![out(), v.clone()])))],
                "store codec: Int".into(),
            ),
            T::Set(x) => {
                let sub = self.need(x)?;
                let item = Expr::Lambda {
                    capture: "&",
                    params: vec![],
                    ret: None,
                    body: vec![
                        Stmt::Var {
                            ty: Ty::Auto,
                            name: "x".into(),
                            init: Some(Expr::call(format!("read_{sub}"), vec![r()])),
                        },
                        Stmt::Return(Some(Expr::bin(
                            "&&",
                            Expr::id("x").method("has_value", vec![]),
                            Expr::id("out").method("insert", vec![Expr::id("x").deref().moved()]).member("second"),
                        ))),
                    ],
                };
                (
                    vec![
                        Stmt::Var { ty: ty.clone(), name: "out".into(), init: None },
                        Stmt::If { cond: r().method("array", vec![item]).negate(), then: vec![none()], els: vec![] },
                        Stmt::Return(Some(Expr::id("out"))),
                    ],
                    each(&sub, v.clone()),
                    "store codec: Set as an ascending array, a repeat rejected".into(),
                )
            }
            T::Map(k, val) => {
                let st = g.f.states.iter().find(|s| &s.ty == t);
                let Some(field) = st.and_then(|s| g.f.facts.keyed.get(&s.id)) else {
                    return Err(g.unsupported(
                        app.span,
                        app,
                        "a Map whose values do not sit under their own key field: the store format has no form for it",
                    ));
                };
                let T::Named(rec) = **val else {
                    return Err(g.unsupported(app.span, app, "a Map of values that are not records"));
                };
                let fname = record_fields(g.f.prog, rec).map(|fs| fs[*field].name.clone()).unwrap_or_default();
                let _ = k;
                let sub = self.need(val)?;
                let item = Expr::Lambda {
                    capture: "&",
                    params: vec![],
                    ret: None,
                    body: vec![
                        Stmt::Var {
                            ty: Ty::Auto,
                            name: "x".into(),
                            init: Some(Expr::call(format!("read_{sub}"), vec![r()])),
                        },
                        Stmt::If {
                            cond: Expr::id("x").negate(),
                            then: vec![Stmt::Return(Some(Expr::Bool(false)))],
                            els: vec![],
                        },
                        Stmt::Var {
                            ty: Ty::Auto.konst(),
                            name: "key".into(),
                            init: Some(Expr::Arrow(Box::new(Expr::id("x")), ident(&fname))),
                        },
                        Stmt::Return(Some(
                            Expr::id("out")
                                .method("emplace", vec![Expr::id("key"), Expr::id("x").deref().moved()])
                                .member("second"),
                        )),
                    ],
                };
                (
                    vec![
                        Stmt::Var { ty: ty.clone(), name: "out".into(), init: None },
                        Stmt::If { cond: r().method("array", vec![item]).negate(), then: vec![none()], els: vec![] },
                        Stmt::Return(Some(Expr::id("out"))),
                    ],
                    each(&sub, Expr::bin("|", v.clone(), Expr::id("std::views::values"))),
                    format!(
                        "store codec: Map keyed by its values' field {fname}, as the array of its values in key order"
                    ),
                )
            }
            _ => return Err(g.unsupported(app.span, app, "a state type the store format has no form for")),
        };
        let reader = Func {
            doc: vec![],
            prefix: vec!["[[nodiscard]]"],
            ret: Ty::generic("std::optional", vec![ty.clone()]),
            name: format!("read_{name}"),
            params: vec![Param { ty: Ty::name("rt::json::Reader").mref(), name: "r".into(), default: None }],
            suffix: vec![],
            body: Some(read),
        };
        let writer = Func {
            doc: if has_int(g, t) {
                vec!["False when the store cannot carry the value: an Int above 2^53.".into()]
            } else {
                vec![]
            },
            prefix: vec!["[[nodiscard]]"],
            ret: Ty::name("bool"),
            name: format!("put_{name}"),
            params: vec![
                Param { ty: Ty::name("std::string").mref(), name: "out".into(), default: None },
                Param { ty: g.param_ty(t), name: "v".into(), default: None },
            ],
            suffix: vec![],
            body: Some(write),
        };
        self.funcs.push((name.clone(), reader, writer, role));
        Ok(name)
    }
}

/// Whether a value of `t` holds an Int, which the store bounds.
fn has_int(g: &Gen<'_, '_>, t: &T) -> bool {
    match t {
        T::Int => true,
        T::Named(id) => match &g.f.prog.ty(*id).def {
            TypeDef::Alias(_) => g.alias_base(*id) == Some(T::Int),
            TypeDef::Record(fields) => {
                let typer = spec_lang_emit_rust::front::typer::Typer::new(g.f.prog, String::new());
                fields.iter().any(|f| typer.ty(&f.ty, f.span).is_ok_and(|ft| has_int(g, &ft)))
            }
            _ => false,
        },
        T::Set(x) | T::List(x) | T::Option(x) => has_int(g, x),
        T::Map(k, v) => has_int(g, k) || has_int(g, v),
        _ => false,
    }
}

fn int_read(make: Expr) -> Vec<Stmt> {
    vec![
        Stmt::Var {
            ty: Ty::Auto.konst(),
            name: "n".into(),
            init: Some(r().method(
                "integer",
                vec![Expr::Cast(Ty::name("std::uint64_t"), Box::new(Expr::id("rt::json::int_limit")))],
            )),
        },
        Stmt::If { cond: Expr::id("n").negate(), then: vec![none()], els: vec![] },
        Stmt::Var {
            ty: Ty::Auto.konst(),
            name: "value".into(),
            init: Some(Expr::Cast(Ty::name("std::int64_t"), Box::new(Expr::id("n").deref()))),
        },
        Stmt::Return(Some(make)),
    ]
}

/// `[x,y,...]` over a range, each with `put_<sub>`.
fn each(sub: &str, range: Expr) -> Vec<Stmt> {
    vec![
        put_char('['),
        Stmt::Var { ty: Ty::name("bool"), name: "first".into(), init: Some(Expr::Bool(true)) },
        Stmt::For {
            ty: Ty::Auto.cref(),
            name: "x".into(),
            range,
            body: vec![
                Stmt::If { cond: Expr::id("first").negate(), then: vec![put_char(',')], els: vec![] },
                Stmt::Expr(Expr::bin("=", Expr::id("first"), Expr::Bool(false))),
                Stmt::If {
                    cond: Expr::call(format!("put_{sub}"), vec![out(), Expr::id("x")]).negate(),
                    then: vec![Stmt::Return(Some(Expr::Bool(false)))],
                    els: vec![],
                },
            ],
        },
        put_char(']'),
        Stmt::Return(Some(Expr::Bool(true))),
    ]
}

/// `include/<ns>/store.hpp` and `src/store.cpp`.
pub fn units(g: &Gen<'_, '_>, board: &str) -> R<(Unit, Unit)> {
    let app = &g.f.app().info;
    let what: Vec<(String, spec_lang_syntax::Span)> =
        g.f.states
            .iter()
            .map(|s| (format!("state {}", g.f.prog.state(s.id).info.name), g.f.prog.state(s.id).info.span))
            .collect();
    let world = Ty::name(format!("{board}::World"));
    let mut h = Unit::new(format!("include/{}/store.hpp", g.ns), g.header(&what), g.origin(app, "file header"));
    h.include(format!("{}/{board}.hpp", g.ns));
    h.include("rt/store_error.hpp");
    let decode_decl = Func {
        doc: vec![
            "The world the bytes hold, or nullopt when they are not a store (any rule of the format broken).".into(),
        ],
        prefix: vec!["[[nodiscard]]"],
        ret: Ty::generic("std::optional", vec![world.clone()]),
        name: "decode".into(),
        params: vec![Param { ty: Ty::name("std::string_view"), name: "bytes".into(), default: None }],
        suffix: vec![],
        body: None,
    };
    let encode_decl = Func {
        doc: vec!["The canonical bytes, newline included; Full when an Int is above 2^53.".into()],
        prefix: vec!["[[nodiscard]]"],
        ret: Ty::generic("std::expected", vec![Ty::name("std::string"), Ty::name("rt::StoreError")]),
        name: "encode".into(),
        params: vec![Param { ty: world.clone().cref(), name: "world".into(), default: None }],
        suffix: vec![],
        body: None,
    };
    let m1 = h.mark(g.origin(app, "store codec: interface"));
    h.decls.push(Decl::Namespace(
        format!("{}::store", g.ns),
        vec![m1, Decl::Func(decode_decl.clone()), Decl::Func(encode_decl.clone())],
    ));

    let mut s = Unit::new("src/store.cpp", g.header(&what), g.origin(app, "file header"));
    s.include(format!("{}/store.hpp", g.ns));
    s.include("rt/json_reader.hpp");
    s.include("rt/json_writer.hpp");
    s.include("rt/flat_map.hpp");
    s.include("rt/unicode.hpp");
    s.include("rt/core.hpp");
    let mut c = Codec { g, funcs: Vec::new(), done: BTreeMap::new() };
    let mut states: Vec<_> = g.f.states.iter().collect();
    states.sort_by(|a, b| g.f.prog.state(a.id).info.name.cmp(&g.f.prog.state(b.id).info.name));
    let mut subs = Vec::new();
    for st in &states {
        subs.push(c.need(&st.ty)?);
    }
    let tys: Vec<Ty> = states.iter().map(|st| g.cty(&st.ty)).collect();
    let mut decode = decode_decl;
    decode.body = Some(decode_body(g, &mut s, &states, &subs, &tys, &world)?);
    let mut projected = Vec::new();
    for r in &g.f.rules {
        let Some(p) = super::board::projection(g, r) else { continue };
        let (reader, ty, decls) = projected_reader(g, &mut c, &mut s, r, p)?;
        let view = super::board::view_name(g, r);
        let vworld = Ty::name(format!("{board}::{view}World"));
        let subs2: Vec<String> = states
            .iter()
            .zip(&subs)
            .map(|(st, sub)| if st.id == p.state { reader.clone() } else { sub.clone() })
            .collect();
        let tys2: Vec<Ty> =
            states.iter().zip(&tys).map(|(st, t)| if st.id == p.state { ty.clone() } else { t.clone() }).collect();
        let info = &g.f.prog.rule(r.id).info;
        let f = Func {
            doc: vec![format!(
                "The world as rule {} reads it, checked exactly as decode() checks the store.",
                info.name
            )],
            prefix: vec!["[[nodiscard]]"],
            ret: Ty::generic("std::optional", vec![vworld.clone()]),
            name: format!("decode_{}", snake(&info.name)),
            params: vec![Param { ty: Ty::name("std::string_view"), name: "bytes".into(), default: None }],
            suffix: vec![],
            body: Some(decode_body(g, &mut s, &states, &subs2, &tys2, &vworld)?),
        };
        let mut decl = f.clone();
        decl.body = None;
        let origin = g.origin(info, "store codec: projected read").strategy("store.read.projection", p.licence.clone());
        projected.push((origin, decls, f, decl));
    }
    // encode
    let mut write = vec![
        Stmt::Var { ty: Ty::name("std::string"), name: "out".into(), init: None },
        Stmt::Expr(out().method("reserve", vec![Expr::Int(256)])),
        append("{\"version\":1"),
    ];
    for (st, sub) in states.iter().zip(&subs) {
        let n = &g.f.prog.state(st.id).info.name;
        write.push(append(&format!(",\"{n}\":")));
        write.push(Stmt::If {
            cond: Expr::call(format!("put_{sub}"), vec![out(), Expr::id("world").member(&ident(n))]).negate(),
            then: vec![Stmt::Return(Some(Expr::call("std::unexpected", vec![Expr::id("rt::StoreError::Full")])))],
            els: vec![],
        });
    }
    write.push(append("}\n"));
    write.push(Stmt::Return(Some(Expr::id("out"))));
    let mut encode = encode_decl;
    encode.body = Some(write);
    let mut inner = Vec::new();
    for (_, reader, writer, role) in &c.funcs {
        inner.push(s.mark(Origin::new(g.path(app), g.at(app.span), role.clone())));
        inner.push(Decl::Func(reader.clone()));
        inner.push(Decl::Func(writer.clone()));
    }
    for (origin, decls, _, _) in &projected {
        inner.push(s.mark(origin.clone()));
        inner.extend(decls.iter().cloned());
    }
    let mut items = vec![Decl::Namespace(String::new(), inner)];
    items.push(s.mark(g.origin(app, "store codec: the document, with every state").strategy(
        "states sorted by name after version; schema-directed reader, no JSON tree",
        "the states and their types fix every key and value; the contract fixes the bytes",
    )));
    items.push(Decl::Func(decode));
    items.push(Decl::Func(encode));
    for (origin, _, f, decl) in projected {
        items.push(s.mark(origin.clone()));
        items.push(Decl::Func(f));
        let mark = h.mark(origin);
        if let Some(Decl::Namespace(_, hs)) = h.decls.last_mut() {
            hs.push(mark);
            hs.push(Decl::Func(decl));
        }
    }
    s.decls.push(Decl::Namespace(format!("{}::store", g.ns), items));
    Ok((h, s))
}

/// The body of a decode: every state's reader, `version`, nothing else, then every invariant.
fn decode_body(
    g: &Gen<'_, '_>,
    s: &mut Unit,
    states: &[&TState],
    subs: &[String],
    tys: &[Ty],
    world: &Ty,
) -> R<Vec<Stmt>> {
    let mut read = vec![
        Stmt::If {
            cond: Expr::call("rt::unicode::is_valid_utf8", vec![Expr::id("bytes")]).negate(),
            then: vec![none()],
            els: vec![],
        },
        Stmt::Var {
            ty: Ty::name("rt::json::Reader"),
            name: "r".into(),
            init: Some(Expr::call("rt::json::Reader", vec![Expr::id("bytes")])),
        },
        Stmt::Var {
            ty: Ty::generic("std::optional", vec![Ty::name("std::uint64_t")]),
            name: "version".into(),
            init: None,
        },
    ];
    for (st, ty) in states.iter().zip(tys) {
        read.push(Stmt::Var {
            ty: Ty::generic("std::optional", vec![ty.clone()]),
            name: ident(&g.f.prog.state(st.id).info.name),
            init: None,
        });
    }
    let mut chain = vec![Stmt::Return(Some(Expr::Bool(false)))];
    for (st, sub) in states.iter().zip(subs).rev() {
        let n = &g.f.prog.state(st.id).info.name;
        let fillit = Stmt::Return(Some(Expr::call(
            "rt::json::fill",
            vec![
                Expr::id(ident(n)),
                Expr::Lambda {
                    capture: "&",
                    params: vec![],
                    ret: None,
                    body: vec![Stmt::Return(Some(Expr::call(format!("read_{sub}"), vec![r()])))],
                },
            ],
        )));
        chain = vec![Stmt::If {
            cond: Expr::bin("==", Expr::id("key"), Expr::str(n.clone())),
            then: vec![fillit],
            els: chain,
        }];
    }
    let version = Stmt::If {
        cond: Expr::bin("==", Expr::id("key"), Expr::str("version")),
        then: vec![Stmt::Return(Some(Expr::call(
            "rt::json::fill",
            vec![
                Expr::id("version"),
                Expr::Lambda {
                    capture: "&",
                    params: vec![],
                    ret: None,
                    body: vec![Stmt::Return(Some(r().method("integer", vec![Expr::Int(1)])))],
                },
            ],
        )))],
        els: chain,
    };
    let member = Expr::Lambda {
        capture: "&",
        params: vec![(Ty::name("std::string_view"), "key".into())],
        ret: None,
        body: vec![version],
    };
    read.push(Stmt::If {
        cond: Expr::bin("||", r().method("object", vec![member]).negate(), r().method("at_end", vec![]).negate()),
        then: vec![none()],
        els: vec![],
    });
    let mut all = Expr::bin("==", Expr::id("version"), Expr::Int(1));
    for st in states {
        all = Expr::bin("&&", all, Expr::id(ident(&g.f.prog.state(st.id).info.name)));
    }
    read.push(Stmt::If { cond: all.negate(), then: vec![none()], els: vec![] });
    // Not const: it is returned, and a const local would be copied instead of moved.
    read.push(Stmt::Var {
        ty: world.clone(),
        name: "world".into(),
        init: Some(Expr::Designated(
            world.clone(),
            g.f.states
                .iter()
                .map(|st| {
                    let n = ident(&g.f.prog.state(st.id).info.name);
                    let x = Expr::id(n.clone()).deref();
                    (n, if g.trivial(&st.ty) { x } else { x.moved() })
                })
                .collect(),
        )),
    });
    let mut decode_body = read;
    for law in &g.f.facts.invariants {
        let Some(tl) = g.f.laws.iter().find(|l| l.id == *law) else { continue };
        let info = &g.f.prog.law(*law).info;
        let K::Core(Op::All, ops) = &tl.body.kind else { continue };
        let K::Lambda { param, body } = &ops[1].kind else { continue };
        let mut env = Env::new(g, info);
        env.bind(param, Local::Value(Expr::id("world")));
        env.missing_is_false = true;
        let law = env.expr(body)?;
        let p = Expr::call(
            "rt::holds",
            vec![Expr::Lambda { capture: "&", params: vec![], ret: None, body: vec![Stmt::Return(Some(law))] }],
        );
        let o = Origin::new(g.path(info), g.at(info.span), "law checked on read").strategy(
            "a world read from outside must meet every law over states",
            format!("law {} is t.states.all(w => ...): an invariant of every world (facts.invariants)", info.name),
        );
        decode_body.push(Stmt::Comment(s.anchor(o)));
        decode_body.push(Stmt::If { cond: p.negate(), then: vec![none()], els: vec![] });
    }
    decode_body.push(Stmt::Return(Some(Expr::id("world"))));
    Ok(decode_body)
}

/// For a read-only rule with a projection: a reader of the projected map that checks every field as the
/// full reader does but keeps only the key and the projected fields. Returns its name, its type and its decls.
fn projected_reader(
    g: &Gen<'_, '_>,
    c: &mut Codec<'_, '_, '_>,
    unit: &mut Unit,
    r: &spec_lang_emit_rust::front::TRule,
    p: &spec_lang_emit_rust::front::facts::Projection,
) -> R<(String, Ty, Vec<Decl>)> {
    let prog = g.f.prog;
    let info = &prog.rule(r.id).info;
    let board = g.module_of(info).to_string();
    let st = g.f.states.iter().find(|s| s.id == p.state);
    let Some(T::Map(k, v)) = st.map(|s| &s.ty) else {
        return Err(g.unsupported(info.span, info, "a projection of a state that is not a Map"));
    };
    let T::Named(rec) = **v else {
        return Err(g.unsupported(info.span, info, "a projection of values that are not records"));
    };
    let Some(&key_field) = g.f.facts.keyed.get(&p.state) else {
        return Err(g.unsupported(info.span, info, "a projection of a map that is not keyed"));
    };
    let fields = record_fields(prog, rec).unwrap_or(&[]);
    let typer = spec_lang_emit_rust::front::typer::Typer::new(prog, g.path(info));
    let view = format!("{board}::{}{}", super::board::view_name(g, r), prog.ty(rec).info.name);
    let rule = snake(&info.name);
    let mut decls = Vec::new();
    let mut slots = Vec::new();
    let mut chain = vec![Stmt::Return(Some(Expr::Bool(false)))];
    let mut branches = Vec::new();
    for (i, f) in fields.iter().enumerate() {
        let fty = typer.ty(&f.ty, f.span)?;
        let name = ident(&f.name);
        let keep = i == key_field || p.fields.contains(&i);
        let branch = if keep {
            let sub = c.need(&fty)?;
            slots.push(Stmt::Var {
                ty: Ty::generic("std::optional", vec![g.cty(&fty)]),
                name: name.clone(),
                init: None,
            });
            Expr::call(
                "rt::json::fill",
                vec![Expr::id(name.clone()), lambda(Expr::call(format!("read_{sub}"), vec![r_()]))],
            )
        } else {
            let check = check_fn(g, c, &mut decls, &fty)?;
            let seen = format!("seen_{name}");
            slots.push(Stmt::Var { ty: Ty::name("bool"), name: seen.clone(), init: Some(Expr::Bool(false)) });
            Expr::call("rt::json::once", vec![Expr::id(seen), lambda(check)])
        };
        branches.push((f.name.clone(), branch));
    }
    for (n, b) in branches.into_iter().rev() {
        chain = vec![Stmt::If {
            cond: Expr::bin("==", Expr::id("key"), Expr::str(n)),
            then: vec![Stmt::Return(Some(b))],
            els: chain,
        }];
    }
    let member = Expr::Lambda {
        capture: "&",
        params: vec![(Ty::name("std::string_view"), "key".into())],
        ret: None,
        body: chain,
    };
    let mut body = slots;
    body.push(Stmt::If { cond: r_().method("object", vec![member]).negate(), then: vec![none()], els: vec![] });
    let all = fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            if i == key_field || p.fields.contains(&i) {
                Expr::id(ident(&f.name))
            } else {
                Expr::id(format!("seen_{}", ident(&f.name)))
            }
        })
        .reduce(|a, b| Expr::bin("&&", a, b));
    if let Some(all) = all {
        body.push(Stmt::If { cond: all.negate(), then: vec![none()], els: vec![] });
    }
    let kept = Expr::Designated(
        Ty::name(view.clone()),
        p.fields.iter().map(|i| (ident(&fields[*i].name), Expr::id(ident(&fields[*i].name)).deref())).collect(),
    );
    let pair = Ty::generic("std::pair", vec![g.cty(k), Ty::name(view.clone())]);
    body.push(Stmt::Return(Some(Expr::Brace(
        Some(pair.clone()),
        vec![Expr::id(ident(&fields[key_field].name)).deref(), kept],
    ))));
    let one = format!("read_{}_for_{rule}", snake(&prog.ty(rec).info.name));
    decls.push(Decl::Func(Func {
        doc: vec![format!("One value as rule {} reads it: every field checked, only these kept.", info.name)],
        prefix: vec!["[[nodiscard]]"],
        ret: Ty::generic("std::optional", vec![pair]),
        name: one.clone(),
        params: vec![Param { ty: Ty::name("rt::json::Reader").mref(), name: "r".into(), default: None }],
        suffix: vec![],
        body: Some(body),
    }));
    let map = Ty::generic("rt::FlatMap", vec![g.cty(k), Ty::name(view)]);
    let item = Expr::Lambda {
        capture: "&",
        params: vec![],
        ret: None,
        body: vec![
            Stmt::Var { ty: Ty::Auto, name: "x".into(), init: Some(Expr::call(one, vec![r_()])) },
            Stmt::If { cond: Expr::id("x").negate(), then: vec![Stmt::Return(Some(Expr::Bool(false)))], els: vec![] },
            Stmt::Expr(Expr::id("out").method(
                "push",
                vec![
                    Expr::Arrow(Box::new(Expr::id("x")), "first".into()),
                    Expr::Arrow(Box::new(Expr::id("x")), "second".into()).moved(),
                ],
            )),
            Stmt::Return(Some(Expr::Bool(true))),
        ],
    };
    let many = format!("read_map_{}_for_{rule}", snake(&prog.ty(rec).info.name));
    decls.push(Decl::Func(Func {
        doc: vec![],
        prefix: vec!["[[nodiscard]]"],
        ret: Ty::generic("std::optional", vec![map.clone()]),
        name: many.clone(),
        params: vec![Param { ty: Ty::name("rt::json::Reader").mref(), name: "r".into(), default: None }],
        suffix: vec![],
        body: Some(vec![
            Stmt::Var { ty: map.clone(), name: "out".into(), init: None },
            Stmt::If {
                cond: Expr::bin(
                    "||",
                    r_().method("array", vec![item]).negate(),
                    Expr::id("out").method("seal", vec![]).negate(),
                ),
                then: vec![none()],
                els: vec![],
            },
            Stmt::Return(Some(Expr::id("out"))),
        ]),
    }));
    let _ = unit;
    Ok((many.trim_start_matches("read_").to_string(), map, decls))
}

fn r_() -> Expr {
    Expr::id("r")
}

fn lambda(value: Expr) -> Expr {
    Expr::Lambda { capture: "&", params: vec![], ret: None, body: vec![Stmt::Return(Some(value))] }
}

/// A check of one value that keeps nothing: a refined text's claim on the borrowed string, else the full
/// reader with its result dropped.
fn check_fn(g: &Gen<'_, '_>, c: &mut Codec<'_, '_, '_>, decls: &mut Vec<Decl>, t: &T) -> R<Expr> {
    if let T::Named(id) = t
        && let Some((T::Text, Some(claim))) = spec_lang_emit_rust::front::typer::alias_base(g.f.prog, *id)
    {
        let d = g.f.prog.ty(*id);
        let name = format!("check_{}", snake(&d.info.name));
        if !c.done.contains_key(&name) {
            c.done.insert(name.clone(), ());
            let mut t = spec_lang_emit_rust::front::typer::Typer::new(g.f.prog, g.path(&d.info));
            t.bind("self", T::Text);
            let typed = t.expr(&claim.expr, Some(&T::Bool))?;
            let mut env = Env::new(g, &d.info);
            env.bind("self", Local::Value(Expr::id("s").deref()));
            let check = env.expr(&typed)?;
            decls.push(Decl::Func(Func {
                doc: vec![format!("A {} checked on the borrowed text, nothing built.", d.info.name)],
                prefix: vec!["[[nodiscard]]"],
                ret: Ty::name("bool"),
                name: name.clone(),
                params: vec![Param { ty: Ty::name("rt::json::Reader").mref(), name: "r".into(), default: None }],
                suffix: vec![],
                body: Some(vec![
                    Stmt::Var { ty: Ty::Auto.konst(), name: "s".into(), init: Some(r_().method("string", vec![])) },
                    Stmt::Return(Some(Expr::bin("&&", Expr::id("s").method("has_value", vec![]), check))),
                ]),
            }));
        }
        return Ok(Expr::call(name, vec![r_()]));
    }
    if let T::Set(x) = t
        && let T::Named(id) = **x
        && let Some((T::Text, Some(claim))) = spec_lang_emit_rust::front::typer::alias_base(g.f.prog, id)
    {
        let d = g.f.prog.ty(id);
        let mut t = spec_lang_emit_rust::front::typer::Typer::new(g.f.prog, g.path(&d.info));
        t.bind("self", T::Text);
        let typed = t.expr(&claim.expr, Some(&T::Bool))?;
        let mut env = Env::new(g, &d.info);
        env.bind("self", Local::Value(Expr::id("s")));
        let check = env.expr(&typed)?;
        return Ok(r_().method(
            "string_set",
            vec![Expr::Lambda {
                capture: "&",
                params: vec![(Ty::name("std::string_view"), "s".into())],
                ret: None,
                body: vec![Stmt::Return(Some(check))],
            }],
        ));
    }
    let sub = c.need(t)?;
    Ok(Expr::call(format!("read_{sub}"), vec![r_()]).method("has_value", vec![]))
}
