//! The whole project: every generated file, the runtime shims, the build files,
//! and `provenance.json`.

use std::collections::BTreeSet;

use spec_lang_diag::Diag;
use spec_lang_emit_rust::provenance::{FileBuf, Origin, OutFile, Span, to_json};
use spec_lang_ir::{ModuleId, UseItem};

use super::{Gen, board, cli, model, store, tests};
use crate::ast::*;
use crate::format::{self, Unit};
use crate::runtime::RUNTIME;

/// Product modules in an order where each comes after the modules it uses.
fn module_order(g: &Gen<'_, '_>) -> Vec<ModuleId> {
    let prog = g.f.prog;
    let mut out: Vec<ModuleId> = Vec::new();
    let mut pending: Vec<ModuleId> = {
        let mut v: Vec<ModuleId> = g.mods.keys().copied().collect();
        v.sort();
        v
    };
    while !pending.is_empty() {
        let before = pending.len();
        pending.retain(|m| {
            let ready = deps(g, *m).iter().all(|d| out.contains(d) || !g.mods.contains_key(d));
            if ready {
                out.push(*m);
            }
            !ready
        });
        if pending.len() == before {
            out.append(&mut pending);
        }
    }
    let _ = prog;
    out
}

fn deps(g: &Gen<'_, '_>, m: ModuleId) -> BTreeSet<ModuleId> {
    let prog = g.f.prog;
    let mut out = BTreeSet::new();
    for u in &prog.module(m).uses {
        match u {
            UseItem::Module { module, .. } => {
                out.insert(*module);
            }
            UseItem::Decl { decl, .. } => {
                let info = match decl {
                    spec_lang_ir::DeclRef::Type(t) => &prog.ty(*t).info,
                    spec_lang_ir::DeclRef::Fn(f) => &prog.func(*f).info,
                    spec_lang_ir::DeclRef::State(s) => &prog.state(*s).info,
                    spec_lang_ir::DeclRef::Rule(r) => &prog.rule(*r).info,
                    spec_lang_ir::DeclRef::Const(c) => &prog.constant(*c).info,
                    spec_lang_ir::DeclRef::Test(t) => &prog.test(*t).info,
                    spec_lang_ir::DeclRef::Law(l) => &prog.law(*l).info,
                    spec_lang_ir::DeclRef::App(a) => &prog.app(*a).info,
                };
                out.insert(info.module);
            }
            UseItem::Vocabulary { .. } => {}
        }
    }
    out.remove(&m);
    out
}

fn module_header(g: &Gen<'_, '_>, m: ModuleId) -> Result<Unit, Diag> {
    let prog = g.f.prog;
    let module = prog.module(m);
    let short = g.mods.get(&m).cloned().unwrap_or_default();
    let what: Vec<(String, spec_lang_syntax::Span)> = module
        .decls
        .iter()
        .filter_map(|d| match d {
            spec_lang_ir::DeclRef::Type(t) => Some((format!("type {}", prog.ty(*t).info.name), prog.ty(*t).info.span)),
            spec_lang_ir::DeclRef::Fn(f) => Some((format!("fn {}", prog.func(*f).info.name), prog.func(*f).info.span)),
            spec_lang_ir::DeclRef::State(s) => {
                Some((format!("state {}", prog.state(*s).info.name), prog.state(*s).info.span))
            }
            spec_lang_ir::DeclRef::Rule(r) => {
                Some((format!("rule {}", prog.rule(*r).info.name), prog.rule(*r).info.span))
            }
            _ => None,
        })
        .collect();
    let mut unit = Unit::new(
        format!("include/{}/{short}.hpp", g.ns),
        g.header(&what),
        Origin::new(module.name.clone(), module.path.clone(), "file header"),
    );
    for d in deps(g, m) {
        if let Some(s) = g.mods.get(&d) {
            unit.include(format!("{}/{s}.hpp", g.ns));
        }
    }
    unit.include("rt/core.hpp");
    unit.include("rt/text.hpp");
    unit.include("rt/unicode.hpp");
    let mut items = model::types_and_fns(g, &mut unit, m)?;
    items.extend(board::header_decls(g, &mut unit, m)?);
    unit.decls.push(Decl::Namespace(format!("{}::{short}", g.ns), items));
    Ok(unit)
}

fn app_header(g: &Gen<'_, '_>, board: &str) -> Unit {
    let app = g.f.app();
    let mut unit = Unit::new(
        format!("include/{}/app.hpp", g.ns),
        g.header(&[(format!("app {}", app.info.name), app.info.span)]),
        g.origin(&app.info, "file header"),
    );
    unit.include(format!("{}/cli.hpp", g.ns));
    unit.include(format!("{}/store.hpp", g.ns));
    unit.include(format!("{}/{board}.hpp", g.ns));
    unit.include("rt/driver.hpp");
    let ev = Ty::name(format!("{board}::Event"));
    let ev_c = ev.clone().cref();
    let world = Ty::name(format!("{board}::World"));
    let f = |prefix: Vec<&'static str>,
             ret: Ty,
             name: &str,
             params: Vec<Param>,
             suffix: Vec<&'static str>,
             body: Vec<Stmt>| {
        Member::Func(Func { doc: vec![], prefix, ret, name: name.into(), params, suffix, body: Some(body) })
    };
    let p = |ty: Ty, name: &str| Param { ty, name: name.into(), default: None };
    let members = vec![
        Member::Using { name: "Values".into(), ty: Ty::name("cli::Values") },
        Member::Using { name: "Event".into(), ty: ev.clone() },
        Member::Using { name: "World".into(), ty: world.clone() },
        Member::Field {
            ty: Ty::name("static constexpr std::string_view"),
            name: "help_text".into(),
            init: Some(Expr::id("cli::help_text")),
        },
        f(
            vec!["[[nodiscard]]", "static", "constexpr"],
            Ty::generic("std::span", vec![Ty::name("rt::cli::CommandSpec<cli::ValueType>").konst()]),
            "commands",
            vec![],
            vec!["noexcept"],
            vec![Stmt::Return(Some(Expr::id("cli::commands")))],
        ),
        f(
            vec!["[[nodiscard]]", "static"],
            ev.clone(),
            "bind",
            vec![p(Ty::name("rt::cli::Bound<cli::Value>"), "bound")],
            vec![],
            vec![Stmt::Return(Some(Expr::call("cli::bind", vec![Expr::id("bound").moved()])))],
        ),
        f(
            vec!["[[nodiscard]]", "static", "constexpr"],
            Ty::name("bool"),
            "writes",
            vec![p(ev.clone().cref(), "event")],
            vec!["noexcept"],
            vec![Stmt::Return(Some(Expr::call(format!("{board}::writes"), vec![Expr::id("event")])))],
        ),
        f(
            vec!["[[nodiscard]]", "static"],
            Ty::generic("std::optional", vec![world.clone()]),
            "decode",
            vec![p(Ty::name("std::string_view"), "bytes")],
            vec![],
            vec![Stmt::Return(Some(Expr::call("store::decode", vec![Expr::id("bytes")])))],
        ),
        f(
            vec!["[[nodiscard]]", "static"],
            Ty::generic("std::expected", vec![Ty::name("std::string"), Ty::name("rt::StoreError")]),
            "encode",
            vec![p(world.clone().cref(), "world")],
            vec![],
            vec![Stmt::Return(Some(Expr::call("store::encode", vec![Expr::id("world")])))],
        ),
        f(
            vec!["[[nodiscard]]", "static"],
            Ty::name("rt::Applied"),
            "apply",
            vec![p(ev, "event"), p(world.mref(), "world")],
            vec![],
            vec![
                Stmt::Var { ty: Ty::name("rt::Applied"), name: "applied".into(), init: None },
                Stmt::Var {
                    ty: Ty::Auto.konst(),
                    name: "done".into(),
                    init: Some(Expr::call(
                        format!("{board}::apply"),
                        vec![Expr::id("event").moved(), Expr::id("world"), Expr::id("applied").member("lines")],
                    )),
                },
                Stmt::If {
                    cond: Expr::id("done").negate(),
                    then: vec![
                        Stmt::Expr(Expr::id("applied").member("lines").method("clear", vec![])),
                        Stmt::Expr(Expr::bin(
                            "=",
                            Expr::id("applied").member("refused"),
                            Expr::call(format!("{board}::name"), vec![Expr::id("done").method("error", vec![])]),
                        )),
                    ],
                    els: vec![],
                },
                Stmt::Return(Some(Expr::id("applied"))),
            ],
        ),
    ];
    let mut members = members;
    let mut projected = Vec::new();
    for r in &g.f.rules {
        let Some(p) = board::projection(g, r) else { continue };
        let info = &g.f.prog.rule(r.id).info;
        let event = format!("{board}::{}", super::camel(&info.name));
        let mut args = vec![Expr::id("world").deref(), Expr::id("applied").member("lines")];
        for prm in &r.params {
            args.push(
                Expr::CallT(Box::new(Expr::id("std::get")), vec![Ty::name(event.clone())], vec![Expr::id("event")])
                    .member(&super::ident(&prm.name)),
            );
        }
        let decode = Expr::call(format!("store::decode_{}", super::snake(&info.name)), vec![Expr::id("bytes")]);
        projected.push(Stmt::If {
            cond: Expr::CallT(
                Box::new(Expr::id("std::holds_alternative")),
                vec![Ty::name(event.clone())],
                vec![Expr::id("event")],
            ),
            then: vec![
                Stmt::Comment(
                    unit.anchor(g.origin(info, "projected read").strategy("store.read.projection", p.licence.clone())),
                ),
                Stmt::Var { ty: Ty::Auto.konst(), name: "world".into(), init: Some(decode) },
                Stmt::If {
                    cond: Expr::id("world").negate(),
                    then: vec![Stmt::Return(Some(Expr::Brace(
                        Some(Ty::generic("std::optional", vec![Ty::name("rt::Applied")])),
                        vec![],
                    )))],
                    els: vec![],
                },
                Stmt::Var { ty: Ty::name("rt::Applied"), name: "applied".into(), init: None },
                Stmt::Expr(Expr::call(format!("{board}::{}", super::ident(&info.name)), args)),
                Stmt::Return(Some(Expr::id("applied"))),
            ],
            els: vec![],
        });
    }
    if !projected.is_empty() {
        projected.push(Stmt::Return(Some(Expr::id("std::nullopt"))));
        members.push(Member::Func(Func {
            doc: vec![
                "A read-only rule whose read set is a projection: the store read builds only what it reads.".into(),
                "nullopt: not such a rule; an empty inner value: the store is unreadable.".into(),
            ],
            prefix: vec!["[[nodiscard]]", "static"],
            ret: Ty::generic("std::optional", vec![Ty::generic("std::optional", vec![Ty::name("rt::Applied")])]),
            name: "project".into(),
            params: vec![
                Param { ty: ev_c, name: "event".into(), default: None },
                Param { ty: Ty::name("std::string_view"), name: "bytes".into(), default: None },
            ],
            suffix: vec![],
            body: Some(projected),
        }));
    }
    let mark = unit.mark(g.origin(&app.info, "app: the program the cli and store runtime run"));
    unit.decls.push(Decl::Namespace(
        format!("{}::app", g.ns),
        vec![
            mark,
            Decl::Struct {
                doc: vec![
                    "The app as the runtime driver (rt/driver.hpp) reads it: its table, codec and rules.".into(),
                    "A rule that only reads takes the shared lock and never writes; see writes().".into(),
                ],
                keyword: "struct",
                name: "App".into(),
                members,
            },
        ],
    ));
    unit
}

fn main_source(g: &Gen<'_, '_>, store_path: &str) -> Unit {
    let app = g.f.app();
    let var = format!("{}_STORE", g.ns.to_uppercase());
    let mut unit = Unit::new(
        "src/main.cpp",
        g.header(&[(format!("app {}, @bin, @store", app.info.name), app.info.span)]),
        g.origin(&app.info, "file header"),
    );
    unit.include(format!("{}/app.hpp", g.ns));
    unit.include("rt/main.hpp");
    let mark = unit.mark(g.origin(&app.info, "process edge").strategy(
        format!("the store path is {store_path}, or the variable {var} when it is set and not empty"),
        "@store(file(...)) on the app; the variable is the contract's (section 3)",
    ));
    unit.decls.push(mark);
    unit.decls.push(Decl::Func(Func {
        doc: vec![],
        prefix: vec![],
        ret: Ty::name("int"),
        name: "main".into(),
        params: vec![
            Param { ty: Ty::name("int"), name: "argc".into(), default: None },
            Param { ty: Ty::name("char**"), name: "argv".into(), default: None },
        ],
        suffix: vec![],
        body: Some(vec![Stmt::Return(Some(Expr::CallT(
            Box::new(Expr::id("rt::main")),
            vec![Ty::name(format!("{}::app::App", g.ns))],
            vec![Expr::id("argc"), Expr::id("argv"), Expr::str(var), Expr::str(store_path)],
        )))]),
    }));
    unit
}

/// Every file of the project, `provenance.json` last.
pub fn files(g: &Gen<'_, '_>) -> Result<Vec<OutFile>, Vec<Diag>> {
    let one = |d: Diag| vec![d];
    let app = g.f.app();
    let board_mod = board::state_module(g)
        .ok_or_else(|| one(Diag::unsupported(app.info.span, "an app with no state").with_decl(g.path(&app.info))))?;
    let board = g.mods.get(&board_mod).cloned().unwrap_or_default();
    let Some(spec_lang_ir::Store::File(store_path)) = app.store.clone() else {
        return Err(one(
            Diag::unsupported(app.info.span, "an app without @store(file(...))").with_decl(g.path(&app.info))
        ));
    };
    let mut units: Vec<Unit> = Vec::new();
    let mut domain_sources = Vec::new();
    for m in module_order(g) {
        units.push(module_header(g, m).map_err(one)?);
        if let Some(u) = model::source(g, m).map_err(one)? {
            domain_sources.push(u.path.clone());
            units.push(u);
        }
    }
    for r in &g.f.rules {
        if g.f.prog.rule(r.id).info.module == board_mod && board::projection(g, r).is_none() {
            let u = board::rule_source(g, r).map_err(one)?;
            domain_sources.push(u.path.clone());
            units.push(u);
        }
    }
    let applied = board::apply_source(g, board_mod).map_err(one)?;
    domain_sources.push(applied.path.clone());
    units.push(applied);
    units.push(cli::header(g, &board).map_err(one)?);
    units.push(cli::source(g, &board).map_err(one)?);
    let (sh, ss) = store::units(g, &board).map_err(one)?;
    units.push(sh);
    units.push(ss);
    units.push(app_header(g, &board));
    units.push(main_source(g, &store_path));
    let mut test_names = Vec::new();
    for (u, name) in tests::units(g, &board).map_err(one)? {
        if !name.is_empty() {
            test_names.push(name);
        }
        units.push(u);
    }
    let mut bufs: Vec<FileBuf> = Vec::new();
    for u in units {
        bufs.push(u.render().map_err(one)?);
    }
    let gen_origin = |role: &str| Origin::new(g.path(&app.info), g.at(app.info.span), role);
    let cmake = cmake(g, &domain_sources, &test_names);
    bufs.push(format::whole(
        "CMakeLists.txt",
        &cmake,
        gen_origin("build: CMake project, from @target(cpp) and @why"),
        false,
    ));
    bufs.push(format::whole(
        ".clang-format",
        &format!("# {}\n{}", g.header(&[(format!("app {}", app.info.name), app.info.span)]), format::STYLE),
        gen_origin("build: the style every file is formatted with"),
        false,
    ));
    bufs.push(format::whole(".gitignore", "build/\n", gen_origin("build: what the build writes"), false));
    let hdr = g.header(&[(format!("app {}, @bin", app.info.name), app.info.span)]);
    bufs.push(format::whole(
        "bin/build",
        &format!("#!/usr/bin/env bash\n# {hdr}\n# Builds build/{bin}.\nset -euo pipefail\ncd \"$(dirname \"$0\")/..\"\ncmake -S . -B build -DCMAKE_BUILD_TYPE=Release\ncmake --build build --parallel\n", bin = g.ns),
        gen_origin("build: bin/build"),
        true,
    ));
    bufs.push(format::whole(
        "bin/check",
        &format!("#!/usr/bin/env bash\n# {hdr}\n# The gate: format, build with every warning an error, every test.\nset -euo pipefail\ncd \"$(dirname \"$0\")/..\"\nmapfile -t sources < <(find include src tests -name '*.hpp' -o -name '*.cpp' | sort)\nclang-format --dry-run --Werror \"${{sources[@]}}\"\nbin/build\nctest --test-dir build --output-on-failure\n"),
        gen_origin("build: bin/check"),
        true,
    ));
    for (path, name, text) in RUNTIME {
        let header = format!(
            "// Generated by spec-lang build: runtime shim {name} of the cpp backend (crates/emit-cpp/runtime/{path}), emitted verbatim; do not edit.\n"
        );
        let mut buf = FileBuf::new(*path);
        let mut origin = Origin::new("runtime", "", format!("runtime shim {name}"));
        if matches!(*name, "rt.file" | "rt.store_hygiene_test") {
            origin = origin.strategy(
                "store.failure-hygiene",
                "contract section 3: a failure leaves the store as it was; the temporary file is removed on every \
                 path, and the signals that end a process are held back from the write to the rename",
            );
        }
        buf.push(&format!("{header}{text}"), origin);
        bufs.push(buf);
    }
    let mut out = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    for b in bufs {
        let (f, s) = format::finish(b);
        out.push(f);
        spans.extend(s);
    }
    out.push(OutFile {
        path: "provenance.json".into(),
        bytes: to_json("cpp", &app.info.name, spans).into_bytes(),
        executable: false,
    });
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn cmake(g: &Gen<'_, '_>, domain: &[String], tests: &[String]) -> String {
    let app = g.f.app();
    let ns = &g.ns;
    let why = app.info.why.clone().unwrap_or_default();
    let list = |xs: &[&str]| xs.iter().map(|x| format!("  {x}\n")).collect::<String>();
    let domain: Vec<&str> =
        domain.iter().map(String::as_str).chain(["src/rt/unicode.cpp", "src/rt/text.cpp"]).collect();
    format!(
        "# {hdr}\ncmake_minimum_required(VERSION 3.28)\nproject({ns} LANGUAGES CXX)\n\nset(CMAKE_CXX_STANDARD 23)\nset(CMAKE_CXX_STANDARD_REQUIRED ON)\nset(CMAKE_CXX_EXTENSIONS OFF)\nset(CMAKE_CXX_SCAN_FOR_MODULES OFF)\nif(NOT CMAKE_BUILD_TYPE)\n  set(CMAKE_BUILD_TYPE Release)\nendif()\n\nadd_compile_options(-Wall -Wextra -Wpedantic -Werror)\n\n# Release: whole-program optimisation and one static binary (@why(\"{why}\")).\ninclude(CheckIPOSupported)\ncheck_ipo_supported(RESULT ipo_ok OUTPUT ipo_message LANGUAGES CXX)\nif(ipo_ok)\n  set(CMAKE_INTERPROCEDURAL_OPTIMIZATION_RELEASE ON)\nendif()\n\n# The modules' types, fns and rules: pure, no I/O. Nothing here links the store or the cli.\nadd_library({ns}_domain STATIC\n{domain})\ntarget_include_directories({ns}_domain PUBLIC include)\n\n# @store(file(...)): codec, lock, atomic file.\nadd_library({ns}_store STATIC\n  src/store.cpp\n  src/rt/json_lexer.cpp\n  src/rt/file.cpp\n  src/rt/lock.cpp\n)\ntarget_link_libraries({ns}_store PUBLIC {ns}_domain)\n\n# The cli vocabulary: parameter table, value parsers, binding.\nadd_library({ns}_cli STATIC src/cli.cpp)\ntarget_link_libraries({ns}_cli PUBLIC {ns}_domain)\n\nadd_executable({ns} src/main.cpp)\ntarget_link_libraries({ns} PRIVATE {ns}_cli {ns}_store)\n{link}\n\n# Tests: one executable per spec test module, @cases module and law module; a small harness.\nenable_testing()\nadd_library({ns}_harness STATIC tests/rt/harness.cpp)\ntarget_include_directories({ns}_harness PUBLIC tests tests/rt)\n\nforeach(name {tests})\n  add_executable(${{name}}_test tests/${{name}}_test.cpp)\n  target_link_libraries(${{name}}_test PRIVATE {ns}_harness {ns}_domain)\n  add_test(NAME ${{name}} COMMAND ${{name}}_test)\nendforeach()\n\n# The runtime's store failure paths, over a toy app: refusal, unreadable store, unwritable directory, lock release, signals.\nadd_executable(store_hygiene_test tests/rt/store_hygiene_test.cpp)\ntarget_link_libraries(store_hygiene_test PRIVATE {ns}_harness {ns}_store)\nadd_test(NAME store_hygiene COMMAND store_hygiene_test)\n",
        hdr = g.header(&[(format!("app {}, @target(cpp), @bin", app.info.name), app.info.span)]),
        domain = list(&domain),
        tests = tests.join(" "),
        link = if g.on("link.static") {
            format!("target_link_options({ns} PRIVATE -static $<$<CONFIG:Release>:-s>)")
        } else {
            format!("target_link_options({ns} PRIVATE $<$<CONFIG:Release>:-s>)")
        },
    )
}
