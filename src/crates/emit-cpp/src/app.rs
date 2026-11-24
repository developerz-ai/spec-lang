//! One app as a project: the program's code, its types as `rt::Data`, the manifest its libraries'
//! runtimes read, and those runtimes, copied from `libs/<name>/real/cpp/`.

use crate::decls::{ModuleDecls, Unit, banner};
use crate::emit::{Fx, Gen, Ret, no_span};
use crate::manifest::note_text;
use crate::names::{ident, path, snake, string_literal};
use crate::project::{self, file, script};
use crate::source::Lines;
use spec_lang_diag::Diag;
use spec_lang_ir::{LibPath, StateId, TypeId};
use spec_lang_lir::manifest::{Named, Note, NoteArg, NoteValue};
use spec_lang_lir::{Body, DTy, LTy, Lir, Manifest, OutFile, Param, TypeDef};
use std::fmt::Write;
use std::path::{Path, PathBuf};

/// `include/program/data.hpp` and its source: every record and sum as `rt::Data` and back.
fn data(g: &Gen<'_>) -> Vec<Unit> {
    let lir = g.lir;
    let mut h = banner("the program's types");
    h.push_str("// Every record and sum as rt::Data and back: how a library's runtime reads and makes the program's values.\n#pragma once\n\n#include \"rt/data.hpp\"\n#include \"rt/prelude.hpp\"\n");
    let mut c = banner("the program's types");
    c.push_str("#include \"program/data.hpp\"\n\n");
    for m in &lir.modules {
        let decls = ModuleDecls::of(g, m.id);
        let types: Vec<TypeId> = decls
            .types
            .iter()
            .copied()
            .filter(|t| matches!(lir.ty(*t).def, TypeDef::Record { .. } | TypeDef::Sum { .. }))
            .collect();
        if types.is_empty() {
            continue;
        }
        let _ = writeln!(h, "#include \"{}.hpp\"", path(&m.name));
        let ns = g.module_ns(m.id);
        let mut decl = format!("\nnamespace {ns} {{\n");
        let _ = writeln!(c, "namespace {ns} {{\n");
        for t in types {
            let d = lir.ty(t);
            let name = ident(&d.info.name);
            let by = if g.is_enum(t) { name.clone() } else { format!("const {name}&") };
            let _ =
                writeln!(decl, "rt::Data to_data({by} value);\nvoid from_data(const rt::Data& data, {name}& value);");
            let fields_to = |fields: &[spec_lang_lir::Field], from: &str| {
                fields.iter().map(|f| format!("to_data({from}{})", ident(&f.name))).collect::<Vec<_>>().join(", ")
            };
            let fields_from = |fields: &[spec_lang_lir::Field], to: &str| {
                let mut s = String::new();
                for (i, f) in fields.iter().enumerate() {
                    let _ = writeln!(s, "from_data(data.values().at({i}), {to}{});", ident(&f.name));
                }
                s
            };
            match &d.def {
                TypeDef::Record { fields } => {
                    let _ = writeln!(
                        c,
                        "rt::Data to_data(const {name}& value) {{\nusing rt::to_data;\nreturn rt::Data::fields(0, {{{}}});\n}}\n",
                        fields_to(fields, "value.")
                    );
                    let _ = writeln!(
                        c,
                        "void from_data(const rt::Data& data, {name}& value) {{\nusing rt::from_data;\n{}}}\n",
                        fields_from(fields, "value.")
                    );
                }
                TypeDef::Sum { .. } if g.is_enum(t) => {
                    let _ = writeln!(
                        c,
                        "rt::Data to_data({name} value) {{\nreturn rt::Data::fields(static_cast<std::size_t>(value), {{}});\n}}\n"
                    );
                    let _ = writeln!(
                        c,
                        "void from_data(const rt::Data& data, {name}& value) {{\nvalue = static_cast<{name}>(data.variant());\n}}\n"
                    );
                }
                TypeDef::Sum { variants } => {
                    let _ = writeln!(c, "rt::Data to_data(const {name}& value) {{\nusing rt::to_data;");
                    for (i, v) in variants.iter().enumerate() {
                        let unused = if v.fields.is_empty() { "[[maybe_unused]] " } else { "" };
                        let _ = writeln!(
                            c,
                            "if ({unused}const auto* v = std::get_if<{name}::{}>(&value.alt)) {{\nreturn rt::Data::fields({i}, {{{}}});\n}}",
                            ident(&v.name),
                            fields_to(&v.fields, "v->")
                        );
                    }
                    c.push_str("rt::stop(\"E-RUNTIME\", \"a sum without a variant\");\n}\n\n");
                    let _ = writeln!(
                        c,
                        "void from_data(const rt::Data& data, {name}& value) {{\nusing rt::from_data;\nswitch (data.variant()) {{"
                    );
                    for (i, v) in variants.iter().enumerate() {
                        let label = if i + 1 == variants.len() { "default".to_string() } else { format!("case {i}") };
                        let _ = writeln!(
                            c,
                            "{label}: {{\n{name}::{} v;\n{}value.alt = std::move(v);\nbreak;\n}}",
                            ident(&v.name),
                            fields_from(&v.fields, "v.")
                        );
                    }
                    c.push_str("}\n}\n\n");
                }
                _ => {}
            }
        }
        let _ = writeln!(h, "{decl}}}  // namespace {ns}");
        let _ = writeln!(c, "}}  // namespace {ns}\n");
    }
    vec![
        Unit { path: "include/program/data.hpp".into(), text: h },
        Unit { path: "src/program/data.cpp".into(), text: c },
    ]
}

/// The descriptions of the declared types the manifest names, numbered as they are first asked for.
struct Types<'g, 'a> {
    g: &'g Gen<'a>,
    types: Vec<DTy>,
}

impl Types<'_, '_> {
    fn of(&mut self, d: &DTy) -> String {
        let i = match self.types.iter().position(|t| t == d) {
            Some(i) => i,
            None => {
                self.types.push(d.clone());
                self.types.len() - 1
            }
        };
        format!("type_{i}")
    }

    fn print(&mut self) -> String {
        let g = self.g;
        let lir = g.lir;
        let mut bodies = String::new();
        let mut i = 0;
        while i < self.types.len() {
            let d = self.types[i].clone();
            // The shape under aliases and claims; the name is the outermost declared one.
            let mut name = String::new();
            let mut shape = d.clone();
            loop {
                shape = match &shape {
                    DTy::Where(b, _) => (**b).clone(),
                    DTy::Named(t) => match &lir.ty(*t).def {
                        TypeDef::Alias { of } => {
                            if name.is_empty() {
                                name = lir.ty(*t).info.name.clone();
                            }
                            of.clone()
                        }
                        _ => break,
                    },
                    _ => break,
                };
            }
            let mut parts: Vec<String> = Vec::new();
            let kind = match &shape {
                DTy::Unit => "Unit",
                DTy::Bool => "Bool",
                DTy::Int | DTy::Range { .. } => "Int",
                DTy::Text => "Text",
                DTy::Option(t) => {
                    parts.push(format!(".args = {{&{}}}", self.of(t)));
                    "Option"
                }
                DTy::List(t) => {
                    parts.push(format!(".args = {{&{}}}", self.of(t)));
                    "List"
                }
                DTy::Set(t) => {
                    parts.push(format!(".args = {{&{}}}", self.of(t)));
                    "Set"
                }
                DTy::Map(k, v) => {
                    parts.push(format!(".args = {{&{}, &{}}}", self.of(k), self.of(v)));
                    "Map"
                }
                DTy::Tuple(ts) => {
                    let args: Vec<String> = ts.iter().map(|t| format!("&{}", self.of(t))).collect();
                    parts.push(format!(".args = {{{}}}", args.join(", ")));
                    "Tuple"
                }
                DTy::Named(t) => {
                    if name.is_empty() {
                        name = lir.ty(*t).info.name.clone();
                    }
                    let fields = |me: &mut Self, fs: &[spec_lang_lir::Field]| {
                        fs.iter()
                            .map(|f| format!("{{{}, &{}}}", string_literal(&f.name), me.of(&f.check)))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    match &lir.ty(*t).def {
                        TypeDef::Record { fields: fs } => {
                            let fs = fields(self, fs);
                            parts.push(format!(".fields = {{{fs}}}"));
                            "Record"
                        }
                        TypeDef::Sum { variants } => {
                            let vs: Vec<String> = variants
                                .iter()
                                .map(|v| format!("{{{}, {{{}}}}}", string_literal(&v.name), fields(self, &v.fields)))
                                .collect();
                            parts.push(format!(".variants = {{{}}}", vs.join(", ")));
                            "Sum"
                        }
                        _ => "Unit",
                    }
                }
                DTy::Where(..) | DTy::Trace(_) => "Unit",
            };
            let mut all = vec![format!(".kind = rt::meta::Kind::{kind}")];
            if !name.is_empty() {
                all.push(format!(".name = {}", string_literal(&name)));
            }
            all.extend(parts);
            let mut fx = Fx::new(g, None, no_span());
            if let Some(c) = fx.conform(&d, "value") {
                all.push(format!(
                    ".conforms = [](const rt::Data& data) {{\nconst auto value = rt::from<{}>(data);\nreturn {c};\n}}",
                    g.dty(&d)
                ));
            }
            let _ = writeln!(
                bodies,
                "const rt::meta::Type& type_{i}() {{\nstatic const rt::meta::Type type{{{}}};\nreturn type;\n}}\n",
                all.join(", ")
            );
            i += 1;
        }
        let mut out = String::from("// The declared types the app's rules and states name.\n");
        for (i, d) in self.types.iter().enumerate() {
            let _ = writeln!(out, "const rt::meta::Type& type_{i}();  // {}", g.dty(d));
        }
        out + "\n" + &bodies
    }
}

fn note_value(v: &NoteValue) -> String {
    match v {
        NoteValue::Int(n) => {
            format!("{{.kind = rt::meta::NoteValue::Kind::Int, .number = {}}}", crate::emit::int_literal(*n).s)
        }
        NoteValue::Text(t) => {
            format!("{{.kind = rt::meta::NoteValue::Kind::Text, .text = {}}}", string_literal(&unescape(t)))
        }
        NoteValue::Quantity { value, unit } => format!(
            "{{.kind = rt::meta::NoteValue::Kind::Quantity, .number = {}, .text = {}}}",
            crate::emit::int_literal(*value).s,
            string_literal(unit)
        ),
        NoteValue::Variant { name, args } => {
            format!(
                "{{.kind = rt::meta::NoteValue::Kind::Variant, .text = {}, .args = {{{}}}}}",
                string_literal(name),
                note_args(args)
            )
        }
    }
}

/// A note's text is kept as written: the escapes of a spec-lang string, processed here.
fn unescape(t: &str) -> String {
    let mut out = String::new();
    let mut it = t.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

fn note_args(args: &[NoteArg]) -> String {
    args.iter()
        .map(|a| format!("{{{}, {}}}", string_literal(&a.param), note_value(&a.value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn note(n: &Note) -> String {
    format!("{{{}, {}, {{{}}}}}", string_literal(&n.library), string_literal(&n.name), note_args(&n.args))
}

/// Whether a value of the type can cross to a library as data.
fn plain(t: &LTy) -> bool {
    match t {
        LTy::Unit | LTy::Bool | LTy::Int | LTy::Text | LTy::Named(_) => true,
        LTy::Option(t) | LTy::List(t) | LTy::Set(t) => plain(t),
        LTy::Map(k, v) => plain(k) && plain(v),
        LTy::Tuple(ts) => ts.iter().all(plain),
        _ => false,
    }
}

/// What the manifest says of one rule or `fn`.
struct Decl<'a> {
    info: &'a spec_lang_lir::Info,
    cpp: String,
    rule: bool,
    writes: bool,
    ctx: bool,
    triggers: &'a [Note],
    params: &'a [Param],
    result: &'a LTy,
    check: &'a Option<DTy>,
    body: &'a Body,
}

fn callable(g: &Gen<'_>, types: &mut Types<'_, '_>, d: &Decl<'_>) -> String {
    let Decl { cpp, rule, writes, ctx, triggers, params, result, check, .. } = d;
    let (ctx, result, check, params) = (*ctx, *result, *check, *params);
    let mut ps = Vec::new();
    for p in params {
        let mut one = format!("{{{}, &{}, {}", string_literal(&p.name), types.of(&p.check), p.default.is_some());
        if let Some(d) = p.default.as_ref().filter(|d| crate::emit::closed(d) && !g.facts.needs_ctx(d)) {
            let mut fx = Fx::new(g, None, d.span);
            let value = fx.expr(d).at(2);
            let _ = write!(one, ", to_data({})", fx.resolve(&value));
        }
        ps.push(one + "}");
    }
    let mut args: Vec<String> = if ctx { vec!["ctx".into()] } else { Vec::new() };
    args.extend(params.iter().enumerate().map(|(i, p)| format!("rt::from<{}>(args.at({i}))", g.ty(&p.ty))));
    let call = format!("{cpp}({})", args.join(", "));
    let body = if *result == LTy::Unit {
        format!("{call};\nreturn rt::Data{{rt::unit}};")
    } else {
        format!("return to_data({call});")
    };
    let result_ty = check.clone().unwrap_or(DTy::Unit);
    let result_ty = if *result == LTy::Unit { DTy::Unit } else { result_ty };
    let ts: Vec<String> = triggers.iter().map(note).collect();
    let foreign = match d.body {
        Body::Extern(x) => (if x.library.is_empty() { x.lang.clone() } else { x.library.clone() }, x.symbol.clone()),
        Body::Expr(_) => (String::new(), String::new()),
    };
    format!(
        "{{.qualified = {}, .name = {}, .why = {}, .foreign_library = {}, .foreign_symbol = {}, .rule = {rule}, .writes = {writes}, .triggers = {{{}}}, .params = {{{}}}, .result = &{}, .call = []({}program::Machine& ctx, {}const std::vector<rt::Data>& args) -> rt::Data {{\n{body}\n}}}}",
        string_literal(&d.info.qualified),
        string_literal(&d.info.name),
        string_literal(d.info.why.as_deref().unwrap_or("")),
        string_literal(&foreign.0),
        string_literal(&foreign.1),
        ts.join(", "),
        ps.join(", "),
        types.of(&result_ty),
        if ctx { "" } else { "[[maybe_unused]] " },
        if params.is_empty() { "[[maybe_unused]] " } else { "" },
    )
}

/// Whether `@uses` names a write that is not a handler's: a write of a state.
fn writes(g: &Gen<'_>, uses: &[String]) -> bool {
    uses.iter().any(|u| u.ends_with(".write") && !g.lir.handlers.iter().any(|h| h.labels.contains(u)))
}

/// `include/program/manifest.hpp` and its source.
fn manifest(g: &Gen<'_>, lines: &Lines, app: &Manifest) -> Vec<Unit> {
    let lir = g.lir;
    let mut types = Types { g, types: Vec::new() };
    let what = format!("app {} ({}.slg)", app.qualified, app.module.replace('.', "/"));
    let mut h = banner(&what);
    h.push_str("#pragma once\n\n#include \"rt/meta.hpp\"\n\nnamespace program {\n\n// The app as its libraries' runtimes read it: notes, rules with their parameters and triggers, states.\nconst rt::meta::App& app();\n\n}  // namespace program\n");

    let mut rules = Vec::new();
    for r in &app.rules {
        let d = lir.rule(*r);
        if !d.params.iter().all(|p| plain(&p.ty)) || !plain(&d.result) {
            g.unsupported(
                d.info.span,
                format_args!("rule {} in an app: a parameter or its result is not plain data", d.info.qualified),
            );
            continue;
        }
        rules.push(format!(
            "// rule {} ({})\n{}",
            d.info.qualified,
            lines.at(lir, &d.info),
            callable(
                g,
                &mut types,
                &Decl {
                    info: &d.info,
                    cpp: g.rule_name(*r),
                    rule: true,
                    writes: writes(g, &d.uses),
                    ctx: true,
                    triggers: &d.triggers,
                    params: &d.params,
                    result: &d.result,
                    check: &d.result_check,
                    body: &d.body
                }
            )
        ));
    }
    let mut fns = Vec::new();
    let named: Vec<usize> = app
        .params
        .iter()
        .flat_map(|p| &p.decls)
        .filter_map(|n| if let Named::Fn(f) = n { Some(f.index()) } else { None })
        .collect();
    for (i, d) in lir.fns.iter().enumerate() {
        let wanted = named.contains(&i)
            || (d.info.is_pub
                && matches!(d.body, Body::Expr(_))
                && lir.modules[d.info.module.index()].name != app.module);
        if !wanted || !d.params.iter().all(|p| plain(&p.ty)) || !plain(&d.result) {
            continue;
        }
        let cpp = g.fn_name(spec_lang_ir::FnId(i as u32));
        fns.push(format!(
            "// fn {} ({})\n{}",
            d.info.qualified,
            lines.at(lir, &d.info),
            callable(
                g,
                &mut types,
                &Decl {
                    info: &d.info,
                    cpp,
                    rule: false,
                    writes: false,
                    ctx: g.facts.fn_ctx[i],
                    triggers: &[],
                    params: &d.params,
                    result: &d.result,
                    check: &d.result_check,
                    body: &d.body
                }
            )
        ));
    }
    let mut states = Vec::new();
    let mut get = Vec::new();
    let mut set = String::new();
    for (i, s) in lir.states.iter().enumerate() {
        let field = g.state_field(StateId(i as u32));
        states.push(format!(
            "{{{}, {}, &{}}}",
            string_literal(&s.info.qualified),
            string_literal(&s.info.name),
            types.of(&s.check)
        ));
        get.push(format!("to_data(ctx.world.{field})"));
        let _ = writeln!(set, "from_data(data.values().at({i}), ctx.world.{field});");
    }
    // A law whose one parameter is the trace says something about a world alone: it must hold on a stored world.
    let mut laws = String::new();
    for l in lir.laws.iter().filter(|l| l.params.len() == 1) {
        let mut fx = Fx::new(g, Some("fresh"), l.info.span);
        let trace = fx.declare(&l.params[0].name);
        let mut body = String::new();
        fx.tail(&l.body, &mut body, Ret::Value);
        let text = format!(
            "// law {} ({})\n[]([[maybe_unused]] Machine& ctx, [[maybe_unused]] const rt::Trace<World>& {trace}) -> bool {{\n{body}}},\n",
            l.info.qualified,
            lines.at(lir, &l.info)
        );
        laws.push_str(&fx.resolve(&text).replace("fresh.", "ctx."));
    }
    let params: Vec<String> = app
        .params
        .iter()
        .map(|p| {
            let decls: Vec<String> = p
                .decls
                .iter()
                .map(|n| match n {
                    Named::Rule(r) => string_literal(&lir.rule(*r).info.qualified),
                    Named::Fn(f) => string_literal(&lir.func(*f).info.qualified),
                })
                .collect();
            format!("{{{}, {{{}}}}}", string_literal(&p.name), decls.join(", "))
        })
        .collect();
    let notes: Vec<String> = app.notes.iter().map(note).collect();

    let mut c = banner(&what);
    c.push_str(
        "#include \"program/manifest.hpp\"\n\n#include \"program/data.hpp\"\n#include \"program/machine.hpp\"\n",
    );
    for m in &lir.modules {
        if !ModuleDecls::of(g, m.id).is_empty() {
            let _ = writeln!(c, "#include \"{}.hpp\"", path(&m.name));
        }
    }
    let types_text = types.print();
    let _ = writeln!(
        c,
        "\nnamespace program {{\n\nnamespace {{\n\nusing rt::from_data;\nusing rt::to_data;\n\n{types_text}"
    );
    let _ = writeln!(
        c,
        "// The laws about one world alone: each must hold on a world a library hands back to the program.\nconst std::vector<bool (*)(Machine&, const rt::Trace<World>&)> laws = {{\n{laws}}};\n\n}}  // namespace\n"
    );
    for n in &app.notes {
        let _ = writeln!(c, "// @{}", note_text(n));
    }
    let _ = writeln!(
        c,
        "const rt::meta::App& app() {{\nstatic const rt::meta::App app{{\n.name = {},\n.qualified = {},\n.module = {},\n.target = {},\n.shape = {},\n.notes = {{{}}},\n.params = {{{}}},\n.rules = {{\n{}}},\n.fns = {{\n{}}},\n.states = {{{}}},\n.world = [](const Machine& ctx) {{ return rt::Data::fields(0, {{{}}}); }},\n.set_world = []([[maybe_unused]] Machine& ctx, [[maybe_unused]] const rt::Data& data) {{\n{set}}},\n.lawful = [](const Machine& ctx) {{\nconst rt::Trace<World> still{{{{}}, {{ctx.world}}}};\ntry {{\nfor (const auto law : laws) {{\nMachine fresh;\nif (!law(fresh, still)) {{\nreturn false;\n}}\n}}\n}} catch (const rt::Failure&) {{\nreturn false;\n}} catch (const rt::Stop&) {{\nreturn false;\n}}\nreturn true;\n}},\n}};\nreturn app;\n}}\n\n}}  // namespace program",
        string_literal(&app.name),
        string_literal(&app.qualified),
        string_literal(&app.module),
        string_literal(app.target.as_deref().unwrap_or("")),
        string_literal(app.shape.as_deref().unwrap_or("")),
        notes.join(", "),
        params.join(", "),
        rules.iter().map(|r| format!("{r},\n")).collect::<String>(),
        fns.iter().map(|r| format!("{r},\n")).collect::<String>(),
        states.join(", "),
        get.join(", "),
    );
    vec![
        Unit { path: "include/program/manifest.hpp".into(), text: h },
        Unit { path: "src/program/manifest.cpp".into(), text: c },
    ]
}

/// The libraries an app needs a runtime of: its shape's, its notes', its rules' triggers', the handlers'
/// its program calls, and those its foreign declarations name. The first is the one that runs the app.
fn libraries(g: &Gen<'_>, app: &Manifest) -> Vec<String> {
    let lir = g.lir;
    let mut out: Vec<String> = Vec::new();
    let mut add = |l: &str| {
        if !l.is_empty() && !out.iter().any(|o| o == l) {
            out.push(l.to_string());
        }
    };
    app.shape.iter().for_each(|s| add(s));
    app.notes.iter().for_each(|n| add(&n.library));
    for r in &app.rules {
        lir.rule(*r).triggers.iter().for_each(|t| add(&t.library));
    }
    crate::machine::handlers(g).iter().for_each(|h| add(&h.library));
    crate::machine::foreign_libraries(g).iter().for_each(|l| add(l));
    out
}

fn copy_tree(root: &Path, dir: &Path, into: &str, out: &mut Vec<OutFile>) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?.map(|e| e.map(|e| e.path())).collect::<Result<_, _>>()?;
    entries.sort();
    for p in entries {
        if p.is_dir() {
            copy_tree(root, &p, into, out)?;
        } else if let Ok(rel) = p.strip_prefix(root) {
            let bytes = std::fs::read(&p)?;
            #[cfg(unix)]
            let executable = {
                use std::os::unix::fs::PermissionsExt;
                std::fs::metadata(&p)?.permissions().mode() & 0o111 != 0
            };
            #[cfg(not(unix))]
            let executable = false;
            out.push(OutFile { path: format!("{into}/{}", rel.display()), bytes, executable });
        }
    }
    Ok(())
}

/// Notes as CMake variables, for the libraries' build files: `APP_NOTE_cli_bin_name`.
fn cmake_notes(app: &Manifest) -> String {
    fn value(prefix: &str, v: &NoteValue, out: &mut String) {
        match v {
            NoteValue::Int(n) => {
                let _ = writeln!(out, "set({prefix} \"{n}\")");
            }
            NoteValue::Text(t) => {
                let _ = writeln!(
                    out,
                    "set({prefix} \"{}\")",
                    t.replace('\\', "\\\\").replace('"', "\\\"").replace('$', "\\$")
                );
            }
            NoteValue::Quantity { value, unit } => {
                let _ = writeln!(out, "set({prefix} \"{value}{unit}\")");
            }
            NoteValue::Variant { name, args } => {
                let _ = writeln!(out, "set({prefix} \"{name}\")");
                args.iter().for_each(|a| value(&format!("{prefix}_{}", a.param), &a.value, out));
            }
        }
    }
    let mut out = String::new();
    for n in &app.notes {
        for a in &n.args {
            value(&format!("APP_NOTE_{}_{}_{}", n.library, n.name, a.param), &a.value, &mut out);
        }
    }
    out
}

const BUILD_SCRIPT: &str = "#!/usr/bin/env bash
# @BANNER@
# Builds the app's executable and its tests into build/.
set -euo pipefail
cd \"$(dirname \"$0\")/..\"
generator=()
command -v ninja >/dev/null && generator=(-G Ninja)
cmake -S . -B build \"${generator[@]}\" -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel
";

const CHECK_SCRIPT: &str = "#!/usr/bin/env bash
# @BANNER@
# The gate: format, build with every warning an error, every test.
set -euo pipefail
cd \"$(dirname \"$0\")/..\"
mapfile -t sources < <(find include src tests lib -name '*.hpp' -o -name '*.cpp' | sort)
clang-format --dry-run --Werror \"${sources[@]}\"
bin/build
# A test that opens a window needs a display: a virtual one is started when there is none.
if [ -z \"${DISPLAY:-}\" ] && command -v xvfb-run >/dev/null; then
  xvfb-run -a ctest --test-dir build --output-on-failure
else
  ctest --test-dir build --output-on-failure
fi
";

pub fn app(lir: &Lir, name: &str) -> Result<Vec<OutFile>, Vec<Diag>> {
    let Some(app) = lir.apps.iter().find(|a| a.name == name) else {
        return Err(vec![Diag::load(format!("no app {name}"))]);
    };
    let g = Gen::new(lir);
    let lines = Lines::read(lir);
    let libs = libraries(&g, app);
    // Each library's runtime, found where `use` found the library.
    let project = lir.modules.iter().find(|m| m.name == app.module).map(|m| PathBuf::from(&m.path)).unwrap_or_default();
    let search = LibPath::for_project(&project);
    let mut copied = Vec::new();
    let mut errors = Vec::new();
    for l in &libs {
        let dir = search.find(l).map(|d| d.join("real").join("cpp"));
        match dir.filter(|d| d.is_dir()) {
            Some(d) => {
                if let Err(e) = copy_tree(&d, &d, &format!("lib/{l}"), &mut copied) {
                    errors.push(Diag::load(format!("library `{l}`: {e}")));
                }
            }
            None => errors.push(
                Diag::new(
                    spec_lang_diag::Severity::Error,
                    "E-UNSUPPORTED",
                    format!("app {}: library `{l}` has no runtime for cpp (libs/{l}/real/cpp)", app.name),
                )
                .with_decl(app.qualified.clone()),
            ),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    copied.retain(|f| !f.path.ends_with("/README.md") && !f.path.ends_with("/.clang-format"));

    let mut printed = crate::program(&g, &lines, true);
    printed.units.extend(data(&g));
    printed.units.extend(manifest(&g, &lines, app));
    let what = format!("app {} ({}.slg)", app.qualified, app.module.replace('.', "/"));
    let driver = libs.first().cloned().unwrap_or_default();
    let mut main = banner(&what);
    let _ = writeln!(
        main,
        "// The `{driver}` library runs the app: it reads the manifest and does the rest.\n#include \"lib/{driver}/main.hpp\"\n#include \"program/manifest.hpp\"\n\nint main(int argc, char** argv) {{\nreturn lib::{}::main(program::app(), argc, argv);\n}}",
        ident(&driver)
    );
    printed.units.push(Unit { path: "src/main.cpp".into(), text: main });
    printed.plain.extend(
        [
            ("include/rt/data.hpp", include_str!("../runtime/include/rt/data.hpp")),
            ("include/rt/meta.hpp", include_str!("../runtime/include/rt/meta.hpp")),
            ("src/rt/data.cpp", include_str!("../runtime/src/rt/data.cpp")),
        ]
        .iter()
        .map(|(p, t)| project::runtime_file(p, t)),
    );
    let has_foreign = !crate::machine::foreign_libraries(&g).is_empty();
    crate::finish(&g, printed, |files| {
        let program_sources: String = project::sources(files, "src/")
            .lines()
            .filter(|l| !l.ends_with("src/main.cpp") && !l.contains("src/program/foreign_"))
            .map(|l| format!("{l}\n"))
            .collect();
        let mut cmake = format!(
            "# {}\n{}",
            banner(&what).trim_start_matches("// ").trim_end(),
            project::CMAKE_HEAD.replace("@NAME@", &snake(&app.name))
        );
        let _ = writeln!(
            cmake,
            "\n# The app's notes, for its libraries' build files.\nset(APP_NAME \"{}\")\n{}",
            app.name,
            cmake_notes(app)
        );
        let _ = writeln!(
            cmake,
            "# The program: its modules, the machine, the manifest, the runtime prelude. No library, no I/O.\nadd_library(program STATIC\n{program_sources})\ntarget_include_directories(program PUBLIC include ${{CMAKE_CURRENT_SOURCE_DIR}})\n"
        );
        cmake.push_str("# The program with its foreign declarations: bound to the libraries' runtimes for the app, and blocked\n# for tests, where a check that reaches a foreign body is blocked, never run.\nadd_library(program_real INTERFACE)\ntarget_link_libraries(program_real INTERFACE program)\nadd_library(program_blocked INTERFACE)\ntarget_link_libraries(program_blocked INTERFACE program)\n");
        if has_foreign {
            cmake.push_str("target_sources(program_real INTERFACE src/program/foreign_real.cpp)\ntarget_sources(program_blocked INTERFACE src/program/foreign_blocked.cpp)\n");
        }
        cmake.push_str("\n# The app: `main` hands the manifest to the library that runs it.\nadd_executable(app src/main.cpp)\ntarget_link_libraries(app PRIVATE program_real)\n\n");
        let _ = writeln!(
            cmake,
            "# The program's own checks, as `spec-lang test` runs them.\nenable_testing()\nadd_executable(checks\n{})\ntarget_include_directories(checks PRIVATE tests)\ntarget_link_libraries(checks PRIVATE program_blocked)\nadd_test(NAME checks COMMAND checks)\n",
            project::sources(files, "tests/")
        );
        cmake.push_str("# The libraries' runtimes, each as its own build file says.\n");
        for l in &libs {
            let _ = writeln!(cmake, "include(lib/{l}/runtime.cmake)");
        }
        let line = banner(&what);
        let line = line.trim_start_matches("// ").trim_end();
        let mut out = vec![
            file("CMakeLists.txt", &cmake),
            script("bin/build", &BUILD_SCRIPT.replace("@BANNER@", line)),
            script("bin/check", &CHECK_SCRIPT.replace("@BANNER@", line)),
        ];
        out.extend(copied);
        out
    })
}
