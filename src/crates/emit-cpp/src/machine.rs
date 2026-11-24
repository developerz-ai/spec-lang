//! `program::World` (every state), the handlers' interfaces with their
//! pretends, and `program::Machine`: the world, the handlers, and the atomic
//! step that takes both back on a failure.

use crate::decls::{ModuleDecls, Unit, banner, foreign_name};
use crate::emit::{Fx, Gen, Ret};
use crate::names::{flat, ident, path, string_literal};
use crate::source::{Lines, Reach};
use spec_lang_lir::{Body, Extern, Handler, LTy, Param};
use std::fmt::Write;

/// `out` as `Out`.
pub fn class_name(handler: &str) -> String {
    let mut c = handler.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The handlers a program can call: those with operations.
pub fn handlers<'a>(g: &Gen<'a>) -> Vec<&'a Handler> {
    g.lir.handlers.iter().filter(|h| !h.ops.is_empty()).collect()
}

fn op_params(g: &Gen<'_>, ps: &[Param], fx: Option<&mut Fx<'_, '_>>, names: Option<&[(String, LTy)]>) -> String {
    let mut fx = fx;
    ps.iter()
        .enumerate()
        .map(|(i, p)| {
            let source = names.and_then(|n| n.get(i)).map_or(p.name.as_str(), |(n, _)| n.as_str());
            match fx.as_deref_mut() {
                Some(fx) => {
                    let name = fx.declare(source);
                    format!("{}{} {name}", fx.mark(&name), g.param_ty(&p.ty))
                }
                None => format!("{} {}", g.param_ty(&p.ty), ident(source)),
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn op_result(g: &Gen<'_>, t: &LTy) -> String {
    if *t == LTy::Unit { "void".into() } else { g.ty(t) }
}

fn handler_decl(g: &Gen<'_>, h: &Handler, out: &mut String) {
    let class = class_name(&h.name);
    let _ = writeln!(
        out,
        "// handler {}.{}: what a rule may ask of the world outside. A library's runtime performs it in an app;",
        h.library, h.name
    );
    let _ = writeln!(
        out,
        "// in tests its pretend does. Until one is installed every operation stops E-UNSUPPORTED.\nclass {class} {{\npublic:\nvirtual ~{class}() = default;"
    );
    for o in &h.ops {
        let ps = o.params.as_deref().unwrap_or(&[]);
        let _ =
            writeln!(out, "virtual {} {}({});", op_result(g, &o.result), ident(&o.name), op_params(g, ps, None, None));
    }
    let _ = writeln!(
        out,
        "// A copy of the handler as it is now: what a refused rule goes back to.\nvirtual std::unique_ptr<{class}> clone() const;\n}};\n"
    );
    if let Some(p) = &h.pretend {
        let name = ident(&p.name);
        let _ = writeln!(
            out,
            "// pretend {}: what tests run for `{}`.\nclass {name} final : public {class} {{\npublic:",
            p.name, h.name
        );
        for (o, arm) in h.ops.iter().zip(&p.arms) {
            if arm.is_some() {
                let ps = o.params.as_deref().unwrap_or(&[]);
                let _ = writeln!(
                    out,
                    "{} {}({}) override;",
                    op_result(g, &o.result),
                    ident(&o.name),
                    op_params(g, ps, None, None)
                );
            }
        }
        let _ = writeln!(out, "std::unique_ptr<{class}> clone() const override;\n\nprivate:");
        for s in &p.state {
            let _ = write!(out, "{} {}", g.dty(&s.check), ident(&s.name));
            if let Some(d) = &s.default {
                let mut fx = Fx::new(g, None, d.span);
                let _ = write!(out, " = {}", fx.expr_typed(d));
            }
            out.push_str(";\n");
        }
        out.push_str("};\n\n");
    }
}

fn handler_impl(g: &Gen<'_>, h: &Handler, out: &mut String) {
    let class = class_name(&h.name);
    for o in &h.ops {
        let ps = o.params.as_deref().unwrap_or(&[]);
        let unnamed: Vec<String> = ps.iter().map(|p| g.param_ty(&p.ty)).collect();
        let what =
            string_literal(&format!("`{}.{}`: no runtime of library `{}` is installed", h.name, o.name, h.library));
        let _ = writeln!(
            out,
            "{} {class}::{}({}) {{\nrt::stop(\"E-UNSUPPORTED\", {what});\n}}\n",
            op_result(g, &o.result),
            ident(&o.name),
            unnamed.join(", ")
        );
    }
    let _ = writeln!(
        out,
        "std::unique_ptr<{class}> {class}::clone() const {{\nreturn std::make_unique<{class}>(*this);\n}}\n"
    );
    let Some(p) = &h.pretend else { return };
    let name = ident(&p.name);
    for (o, arm) in h.ops.iter().zip(&p.arms) {
        let Some(arm) = arm else { continue };
        let ps = o.params.as_deref().unwrap_or(&[]);
        let mut fx = Fx::new(g, None, arm.body.span);
        // The pretend's state is the object's: an arm reads and changes its members.
        for s in &p.state {
            fx.bind(&s.name, &ident(&s.name));
        }
        let params = op_params(g, ps, Some(&mut fx), Some(&arm.params));
        let mut body = String::new();
        for (param, (source, _)) in ps.iter().zip(&arm.params) {
            let cpp = fx.local(source);
            if let Some(c) = fx.conform(&param.check, &cpp) {
                let _ = writeln!(
                    body,
                    "rt::claim({c}, {});",
                    string_literal(&format!("argument `{}` of `{}.{}`", param.name, h.name, o.name))
                );
            }
        }
        let ret = if o.result == LTy::Unit { Ret::Void } else { Ret::Value };
        fx.tail(&arm.body, &mut body, ret);
        let text = format!("{} {name}::{}({params}) {{\n{body}}}\n", op_result(g, &o.result), ident(&o.name));
        let _ = writeln!(out, "{}", fx.resolve(&text));
    }
    let _ = writeln!(
        out,
        "std::unique_ptr<{class}> {name}::clone() const {{\nreturn std::make_unique<{name}>(*this);\n}}\n"
    );
}

/// `include/program/machine.hpp` and `src/program/machine.cpp`.
pub fn machine(g: &Gen<'_>, lines: &Lines) -> Vec<Unit> {
    let lir = g.lir;
    let hs = handlers(g);
    let mut reach = Reach::default();
    let mut src_reach = Reach::default();
    for s in &lir.states {
        reach.dty(lir, &s.check);
        s.init.iter().for_each(|e| src_reach.expr(lir, e));
    }
    for h in &hs {
        for o in &h.ops {
            reach.params(lir, o.params.as_deref().unwrap_or(&[]));
            reach.lty(lir, &o.result);
        }
        if let Some(p) = &h.pretend {
            reach.params(lir, &p.state);
            p.arms.iter().flatten().for_each(|a| src_reach.expr(lir, &a.body));
            p.state.iter().filter_map(|s| s.default.as_ref()).for_each(|d| reach.expr(lir, d));
        }
    }
    let includes = |r: &Reach, skip: Option<&Reach>| {
        let mut s = String::new();
        for m in &r.modules {
            if !ModuleDecls::of(g, *m).is_empty() && skip.is_none_or(|k| !k.modules.contains(m)) {
                let _ = writeln!(s, "#include \"{}.hpp\"", path(&lir.modules[m.index()].name));
            }
        }
        s
    };

    let mut h = banner("the program's states, rules and handlers");
    let _ =
        writeln!(h, "#pragma once\n\n#include \"rt/prelude.hpp\"\n{}\nnamespace program {{\n", includes(&reach, None));
    h.push_str("// Every rule's number, as an event of a trace names it.\nnamespace rule {\n");
    for (i, r) in lir.rules.iter().enumerate() {
        let _ = writeln!(h, "inline constexpr int {} = {i};", flat(&r.info.qualified));
    }
    h.push_str("}  // namespace rule\n\n// Every state of the program at one moment.\nstruct World {\n");
    for (i, s) in lir.states.iter().enumerate() {
        let _ = writeln!(h, "// state {} ({})", s.info.qualified, lines.at(lir, &s.info));
        let _ = writeln!(h, "{} {};", g.dty(&s.check), g.state_field(spec_lang_ir::StateId(i as u32)));
    }
    h.push_str("friend std::strong_ordering operator<=>(const World&, const World&) = default;\nfriend bool operator==(const World&, const World&) = default;\nfriend rt::Text show(const World& value);\n};\n\n");
    for hd in &hs {
        handler_decl(g, hd, &mut h);
    }
    h.push_str("// The world and the handlers. A rule runs as one atomic step of it.\nstruct Machine {\n// The initial world: every state at its initial value, every handler at its pretend.\nMachine();\n\nWorld world;\n");
    for hd in &hs {
        let _ = writeln!(h, "std::unique_ptr<{}> {};", class_name(&hd.name), ident(&hd.name));
    }
    h.push_str("\n// Runs `step`; on any failure the world and the handlers go back to what they were, and the failure goes on.\ntemplate <class F>\nauto atomic(F&& step) -> std::invoke_result_t<F> {\nWorld saved_world = world;\n");
    for hd in &hs {
        let n = ident(&hd.name);
        let _ = writeln!(h, "auto saved_{n} = {n}->clone();");
    }
    h.push_str("try {\nreturn step();\n} catch (...) {\nworld = std::move(saved_world);\n");
    for hd in &hs {
        let n = ident(&hd.name);
        let _ = writeln!(h, "{n} = std::move(saved_{n});");
    }
    h.push_str("throw;\n}\n}\n};\n\n}  // namespace program\n");

    let mut c = banner("the program's states, rules and handlers");
    let _ = writeln!(
        c,
        "#include \"program/machine.hpp\"\n\n{}\nnamespace program {{\n",
        includes(&src_reach, Some(&reach))
    );
    c.push_str("rt::Text show(const World& value) {\nrt::Text out = \"World(\";\n");
    for (i, s) in lir.states.iter().enumerate() {
        let sep = if i == 0 { "" } else { ", " };
        let _ = writeln!(
            c,
            "out += {} + rt::str(value.{});",
            string_literal(&format!("{sep}{}: ", s.info.qualified)),
            g.state_field(spec_lang_ir::StateId(i as u32))
        );
    }
    if lir.states.is_empty() {
        c.push_str("static_cast<void>(value);\n");
    }
    c.push_str("return out + \")\";\n}\n\n");
    for hd in &hs {
        handler_impl(g, hd, &mut c);
    }
    // The initial world: each state's initial value, then each conformed to its declared type.
    let mut init = String::new();
    let mut fx = Fx::new(g, Some("ctx"), crate::emit::no_span());
    for (i, s) in lir.states.iter().enumerate() {
        fx.span = s.info.span;
        let field = g.state_field(spec_lang_ir::StateId(i as u32));
        match &s.init {
            Some(e) => {
                let _ = writeln!(init, "world.{field} = {};", fx.expr_typed(e));
            }
            None if matches!(s.ty, LTy::Map(..) | LTy::Set(_) | LTy::List(_) | LTy::Option(_)) => {}
            None => {
                let _ = writeln!(
                    init,
                    "rt::stop(\"E-TYPE-MISMATCH\", {});",
                    string_literal(&format!(
                        "state {} is not a collection or an Option and has no initial value",
                        s.info.qualified
                    ))
                );
            }
        }
    }
    for (i, s) in lir.states.iter().enumerate() {
        let field = format!("world.{}", g.state_field(spec_lang_ir::StateId(i as u32)));
        if let Some(cond) = fx.conform(&s.check, &field) {
            let _ = writeln!(
                init,
                "rt::claim({cond}, {});",
                string_literal(&format!("the initial value of {}", s.info.qualified))
            );
        }
    }
    let mut ctor = String::from("Machine::Machine() {\n");
    for hd in &hs {
        let class = hd.pretend.as_ref().map_or_else(|| class_name(&hd.name), |p| ident(&p.name));
        let _ = writeln!(ctor, "{} = std::make_unique<{class}>();", ident(&hd.name));
    }
    if fx.ctx_used {
        ctor.push_str("Machine& ctx = *this;\n");
    }
    let _ = writeln!(c, "{ctor}{}}}\n\n}}  // namespace program", fx.resolve(&init));
    vec![
        Unit { path: "include/program/machine.hpp".into(), text: h },
        Unit { path: "src/program/machine.cpp".into(), text: c },
    ]
}

/// Every foreign declaration of the program, once per symbol and signature.
fn externs<'a>(g: &Gen<'a>) -> Vec<(&'a Extern, &'a [Param], &'a LTy, &'a spec_lang_lir::Info)> {
    let lir = g.lir;
    let mut out: Vec<(&Extern, &[Param], &LTy, &spec_lang_lir::Info)> = Vec::new();
    let fns = lir.fns.iter().map(|d| (&d.body, d.params.as_slice(), &d.result, &d.info));
    let rules = lir.rules.iter().map(|d| (&d.body, d.params.as_slice(), &d.result, &d.info));
    for (body, params, result, info) in fns.chain(rules) {
        if let Body::Extern(x) = body {
            let sig = |ps: &[Param]| ps.iter().map(|p| g.ty(&p.ty)).collect::<Vec<_>>();
            let twice = out.iter().any(|(o, ps, _, _)| foreign_name(o) == foreign_name(x) && sig(ps) == sig(params));
            if !twice {
                out.push((x, params, result, info));
            }
        }
    }
    out
}

fn extern_sig(g: &Gen<'_>, x: &Extern, params: &[Param], result: &LTy, named: bool) -> String {
    let ps: Vec<String> = params
        .iter()
        .map(|p| if named { format!("{} {}", g.param_ty(&p.ty), ident(&p.name)) } else { g.param_ty(&p.ty) })
        .collect();
    format!("{} {}({})", op_result(g, result), ident(&x.symbol), ps.join(", "))
}

/// The libraries whose foreign symbols the program declares.
pub fn foreign_libraries(g: &Gen<'_>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (x, ..) in externs(g) {
        let library = if x.library.is_empty() { x.lang.clone() } else { x.library.clone() };
        if !out.contains(&library) {
            out.push(library);
        }
    }
    out
}

/// `include/program/foreign.hpp`, and its two bodies: blocked for tests, and bound to the libraries'
/// runtimes for an app (`real`).
pub fn foreign(g: &Gen<'_>, lines: &Lines, real: bool) -> Vec<Unit> {
    let lir = g.lir;
    let all = externs(g);
    if all.is_empty() {
        return Vec::new();
    }
    let mut reach = Reach::default();
    for (_, params, result, _) in &all {
        reach.params(lir, params);
        reach.lty(lir, result);
    }
    let mut h = banner("the program's foreign declarations");
    h.push_str("#pragma once\n\n#include \"rt/prelude.hpp\"\n");
    for m in &reach.modules {
        if !ModuleDecls::of(g, *m).is_empty() {
            let _ = writeln!(h, "#include \"{}.hpp\"", path(&lir.modules[m.index()].name));
        }
    }
    h.push_str("\n// What `foreign <lang> \"<library>#<symbol>\"` calls. The test build links bodies that block the check;\n// an app links the libraries' runtimes.\n");
    let mut blocked = banner("the program's foreign declarations");
    blocked.push_str("// No library is linked here: a check that reaches a foreign body is blocked.\n#include \"program/foreign.hpp\"\n\n");
    let mut bound = banner("the program's foreign declarations");
    bound.push_str("// Each foreign declaration, bound to its library's runtime.\n#include \"program/foreign.hpp\"\n\n#include \"program/data.hpp\"\n");
    for library in foreign_libraries(g) {
        let _ = writeln!(bound, "#include \"lib/{library}/foreign.hpp\"");
    }
    bound.push('\n');
    for library in foreign_libraries(g) {
        let ns = format!("foreign::{}", ident(&library));
        let _ = writeln!(h, "namespace {ns} {{");
        let _ = writeln!(blocked, "namespace {ns} {{");
        let _ = writeln!(bound, "namespace {ns} {{");
        for (x, params, result, info) in
            all.iter().filter(|(x, ..)| if x.library.is_empty() { x.lang == library } else { x.library == library })
        {
            let written = format!(
                "foreign {} \"{}{}{}\"",
                x.lang,
                x.library,
                if x.library.is_empty() { "" } else { "#" },
                x.symbol
            );
            let _ = writeln!(h, "// {} ({}) = {written}", info.qualified, lines.at(lir, info));
            let _ = writeln!(h, "{};", extern_sig(g, x, params, result, true));
            let _ = writeln!(
                blocked,
                "{} {{\nrt::foreign({}, {});\n}}",
                extern_sig(g, x, params, result, false),
                string_literal(&info.qualified),
                string_literal(&written)
            );
            let args: Vec<String> = params.iter().map(|p| ident(&p.name)).collect();
            let give = if **result == LTy::Unit { "" } else { "return " };
            let _ = writeln!(
                bound,
                "{} {{\n{give}lib::{}::{}({});\n}}",
                extern_sig(g, x, params, result, true),
                ident(&library),
                ident(&x.symbol),
                args.join(", ")
            );
        }
        let _ = writeln!(h, "}}  // namespace {ns}\n");
        let _ = writeln!(blocked, "}}  // namespace {ns}\n");
        let _ = writeln!(bound, "}}  // namespace {ns}\n");
    }
    let mut out = vec![
        Unit { path: "include/program/foreign.hpp".into(), text: h },
        Unit { path: "src/program/foreign_blocked.cpp".into(), text: blocked },
    ];
    if real {
        out.push(Unit { path: "src/program/foreign_real.cpp".into(), text: bound });
    }
    out
}
