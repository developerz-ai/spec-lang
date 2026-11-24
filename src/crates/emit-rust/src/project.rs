//! The Cargo project: one file per module, the machine, the checks, the generators and checks of declared types,
//! the runtime, `Cargo.toml`, `bin/test`. Every Rust file is parsed and printed by `prettyplease`, so a printer
//! bug is an error here and not in rustc.

use crate::code::uses;
use crate::cx::{Cx, Mode};
use crate::names::ident;
use spec_lang_diag::Diag;
use spec_lang_ir::ModuleId;
use spec_lang_lir::{Lir, OutFile};
use std::collections::BTreeMap;

/// The runtime, copied verbatim into every project.
pub const RUNTIME: &[(&str, &str)] = &[
    ("src/rt/mod.rs", include_str!("../runtime/rt/mod.rs")),
    ("src/rt/app.rs", include_str!("../runtime/rt/app.rs")),
    ("src/rt/data.rs", include_str!("../runtime/rt/data.rs")),
    ("src/rt/fun.rs", include_str!("../runtime/rt/fun.rs")),
    ("src/rt/int.rs", include_str!("../runtime/rt/int.rs")),
    ("src/rt/list.rs", include_str!("../runtime/rt/list.rs")),
    ("src/rt/rng.rs", include_str!("../runtime/rt/rng.rs")),
    ("src/rt/runner.rs", include_str!("../runtime/rt/runner.rs")),
    ("src/rt/show.rs", include_str!("../runtime/rt/show.rs")),
    ("src/rt/text.rs", include_str!("../runtime/rt/text.rs")),
];

/// Names a file may use, and where they come from.
const IMPORTS: &[(&str, &[&str])] = &[
    ("crate::rt", &["Failure", "Fun", "Int", "Never", "Opt", "R", "Range", "Show"]),
    ("crate::rt::rng", &["Rng"]),
    ("crate::rt::runner", &["Body", "Check", "Given", "LawBody", "LawDecl", "TestDecl", "Trace", "What"]),
    ("crate::machine", &["Machine", "Pools", "World"]),
    ("crate::rt::data", &["AsData", "Data", "Shape", "Ty"]),
    (
        "crate::rt::app",
        &[
            "App",
            "AppParam",
            "Decl",
            "Extern",
            "Func",
            "Note",
            "NoteArg",
            "NoteValue",
            "Param",
            "Read",
            "Rule",
            "State",
        ],
    ),
    ("std::collections", &["BTreeMap", "BTreeSet"]),
];

/// `use` lines for the names a body uses that `own` does not define.
fn imports(body: &str, own: &str) -> String {
    let code: String = body.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
    let body = code.as_str();
    let mut out = Vec::new();
    for (path, names) in IMPORTS {
        if *path == own
            || (path.starts_with("crate::rt::app") || path.starts_with("crate::rt::data")) && own != "crate::manifest"
        {
            continue;
        }
        let defines = |n: &str| {
            ["struct", "enum", "type", "fn", "const", "static"].iter().any(|k| uses(body, &format!("{k} {n}")))
        };
        let used: Vec<&str> = names
            .iter()
            .copied()
            .filter(|n| !defines(n) && (uses(body, n) || (*n == "Show" && body.contains(".show()"))))
            .collect();
        let selfish = *path == "crate::rt" && body.contains("rt::");
        let mut list: Vec<String> = Vec::new();
        if selfish {
            list.push("self".into());
        }
        list.extend(used.iter().map(|s| s.to_string()));
        match list.len() {
            0 => {}
            1 if !selfish => out.push(format!("use {path}::{};", list[0])),
            _ => out.push(format!("use {path}::{{{}}};", list.join(", "))),
        }
    }
    out.join("\n")
}

/// A blank line between items: prettyplease prints none.
fn spaced(text: &str) -> String {
    let grouped = |l: &str| l.starts_with("use ") || l.starts_with("pub mod ") || l.starts_with("//!");
    let mut out: Vec<&str> = Vec::new();
    for line in text.lines() {
        let starts = !line.is_empty() && !line.starts_with([' ', '}', ')', ']']);
        if starts && let Some(prev) = out.last().copied() {
            let ends = prev.starts_with('}')
                || prev.starts_with("//!")
                || (!prev.starts_with(' ') && prev.ends_with(';'))
                || prev.ends_with("];")
                || prev.ends_with("};");
            let together = (grouped(prev) && grouped(line) && prev.starts_with("//!") == line.starts_with("//!"))
                || (prev.starts_with("///") || prev.starts_with("#["));
            if ends && !together {
                out.push("");
            }
        }
        out.push(line);
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

/// A Rust file, parsed and printed.
fn rust(path: &str, header: &str, own: &str, body: &str) -> Result<OutFile, Diag> {
    let text = format!("{header}\n{}\n\n{body}\n", imports(body, own));
    let file = syn::parse_file(&text).map_err(|e| {
        let at = e.span().start();
        let line = text.lines().nth(at.line.saturating_sub(1)).unwrap_or("");
        let lo = at.column.saturating_sub(120);
        let snippet: String = line.chars().skip(lo).take(240).collect();
        Diag::load(format!(
            "the Rust printer wrote {path}, which does not parse: {e} at {}:{}: {snippet}",
            at.line, at.column
        ))
    })?;
    Ok(OutFile { path: path.to_string(), bytes: spaced(&prettyplease::unparse(&file)).into_bytes(), executable: false })
}

fn plain(path: &str, text: String, executable: bool) -> OutFile {
    OutFile { path: path.to_string(), bytes: text.into_bytes(), executable }
}

/// Every module of the program but `core`, by its Rust path.
fn module_tree(cx: &Cx<'_>) -> BTreeMap<Vec<String>, Option<ModuleId>> {
    let mut tree: BTreeMap<Vec<String>, Option<ModuleId>> = BTreeMap::new();
    for m in cx.lir.modules.iter().skip(1) {
        let segs = cx.names.module(m.id).to_vec();
        for i in 1..segs.len() {
            tree.entry(segs[..i].to_vec()).or_insert(None);
        }
        tree.insert(segs, Some(m.id));
    }
    tree
}

/// The program's own files: modules, machine, checks, generators, checks of declared types, `lib.rs`.
pub fn program(cx: &mut Cx<'_>, extra_mods: &[&str]) -> Result<Vec<OutFile>, Vec<Diag>> {
    let lir = cx.lir;
    let mut files = Vec::new();
    let mut errors = Vec::new();
    let mut push = |r: Result<OutFile, Diag>, errors: &mut Vec<Diag>| match r {
        Ok(f) => files.push(f),
        Err(d) => errors.push(d),
    };
    let tree = module_tree(cx);
    let mut top = Vec::new();
    for (segs, id) in &tree {
        let children: Vec<&String> =
            tree.keys().filter(|k| k.len() == segs.len() + 1 && k.starts_with(segs)).map(|k| &k[segs.len()]).collect();
        let path = if children.is_empty() {
            format!("src/{}.rs", segs.join("/"))
        } else {
            format!("src/{}/mod.rs", segs.join("/"))
        };
        if segs.len() == 1 {
            top.push(segs[0].clone());
        }
        let mods: Vec<String> = children.iter().map(|c| format!("pub mod {c};")).collect();
        let (header, body) = match id {
            Some(m) => {
                let module = &lir.modules[m.index()];
                let body = crate::decls::module_file(cx, *m);
                (
                    format!(
                        "//! Module `{}`, printed by spec-lang from `{}`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.",
                        module.name, module.path
                    ),
                    body,
                )
            }
            None => (format!("//! Modules under `{}`.", segs.join(".")), String::new()),
        };
        push(rust(&path, &header, "", &format!("{}\n\n{body}", mods.join("\n"))), &mut errors);
    }
    let machine = crate::machine::file(cx);
    push(
        rust(
            "src/machine.rs",
            "//! The world, the machine, each handler operation, events and replies of traces.",
            "crate::machine",
            &machine,
        ),
        &mut errors,
    );
    let checks = crate::checks::file(cx);
    push(
        rust("src/checks.rs", "//! Every check of the program, as `spec-lang test` names and runs it.", "", &checks),
        &mut errors,
    );
    let pools = crate::dty::pool_types(cx);
    let gens = crate::dty::gen_file(cx, &pools);
    push(
        rust(
            "src/draw.rs",
            "//! A generator per declared type a property, law or trace draws from: the engine's draws.",
            "",
            &gens,
        ),
        &mut errors,
    );
    let conforms = crate::dty::conform_file(cx);
    push(
        rust(
            "src/conform.rs",
            "//! Whether a value is of a declared type: ranges and `where` claims, field by field.",
            "",
            &conforms,
        ),
        &mut errors,
    );
    let mut mods: Vec<String> = ["checks", "conform", "draw", "machine", "rt"].iter().map(|s| s.to_string()).collect();
    mods.extend(extra_mods.iter().map(|s| s.to_string()));
    mods.extend(top);
    let lib = mods.iter().map(|m| format!("pub mod {m};")).collect::<Vec<_>>().join("\n");
    push(
        rust(
            "src/lib.rs",
            "//! A spec-lang program printed as Rust by spec-lang's Rust printer: its modules, its world and machine, its checks.",
            "",
            &lib,
        ),
        &mut errors,
    );
    errors.append(&mut cx.diags);
    if !errors.is_empty() {
        return Err(errors);
    }
    for (path, text) in RUNTIME {
        files.push(plain(path, text.to_string(), false));
    }
    Ok(files)
}

fn cargo_toml(package: &str, bins: &[(&str, &str)]) -> String {
    let mut s = format!(
        "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n[lib]\nname = \"program\"\npath = \"src/lib.rs\"\n"
    );
    for (name, path) in bins {
        s.push_str(&format!("\n[[bin]]\nname = \"{name}\"\npath = \"{path}\"\n"));
    }
    s.push_str("\n[profile.release]\ndebug = false\n\n# A printed project stands alone; it never joins an enclosing workspace.\n[workspace]\n");
    s
}

const TEST_SCRIPT: &str = "#!/usr/bin/env bash\n# Runs every check of the program: one JSON line per check. Arguments: [--seed N] [--runs N] [--traces N] [--steps N]\nset -euo pipefail\ncd \"$(dirname \"$0\")/..\"\ncargo build --release --quiet --bin checks >&2\nexec target/release/checks \"$@\"\n";

fn checks_main() -> &'static str {
    "//! Runs every check of the program: `bin/test`.\n\nfn main() -> std::process::ExitCode {\n    program::rt::runner::main()\n}\n"
}

/// The whole program's checks as a project: `bin/test`.
pub fn tests(lir: &Lir) -> Result<Vec<OutFile>, Vec<Diag>> {
    let mut cx = Cx::new(lir, Mode::Tests);
    let mut files = program(&mut cx, &[])?;
    files.push(plain("Cargo.toml", cargo_toml("spec-program", &[("checks", "src/bin/checks.rs")]), false));
    files.push(plain("src/bin/checks.rs", checks_main().to_string(), false));
    files.push(plain("bin/test", TEST_SCRIPT.to_string(), true));
    files.push(crate::provenance::file(&files));
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

const CHECK_SCRIPT: &str = "#!/usr/bin/env bash\n# The gate of this printed app: clippy with warnings denied, the runtimes' own tests, the program's checks, the release\n# build. The printer lays the\n# code out with prettyplease, not rustfmt, so `cargo fmt --check` is not part of it.\nset -euo pipefail\ncd \"$(dirname \"$0\")/..\"\ncargo clippy --release --all-targets --quiet -- -D warnings\ncargo test --release --quiet\nbin/test >/dev/null\ncargo build --release --quiet\n";

const BUILD_SCRIPT: &str = "#!/usr/bin/env bash\n# Builds the app's executable into target/release/.\nset -euo pipefail\ncd \"$(dirname \"$0\")/..\"\ncargo build --release --quiet\n";

/// One app's project: the program, its manifest, its libraries' runtimes, the executable and its checks.
pub fn app(lir: &Lir, name: &str) -> Result<Vec<OutFile>, Vec<Diag>> {
    let Some(m) = lir.apps.iter().find(|a| a.name == name) else {
        return Err(vec![Diag::load(format!("no app named {name}"))]);
    };
    let mut cx = Cx::new(lir, Mode::App);
    let libs = crate::app::libraries(&cx, m).map_err(|d| vec![d])?;
    let Some(entry) = crate::app::entry(m, &libs) else {
        return Err(vec![crate::app::unsupported(
            m,
            format!(
                "app {} names no library whose Rust runtime runs an app (`entry = true` in its runtime.conf)",
                m.name
            ),
        )]);
    };
    let exe = crate::app::executable(m, Some(entry));
    let mut files = program(&mut cx, &["libs", "manifest"])?;
    let manifest = crate::app::manifest_file(&mut cx, m);
    // The manifest may register more checks of declared types: print `conform.rs` again with them.
    let conforms = crate::dty::conform_file(&mut cx);
    let mut errors = std::mem::take(&mut cx.diags);
    for (path, header, own, body) in [
        (
            "src/manifest.rs",
            "//! The app's manifest: its notes, parameters and rules as data, with calls from plain data.",
            "crate::manifest",
            manifest,
        ),
        (
            "src/conform.rs",
            "//! Whether a value is of a declared type: ranges and `where` claims, field by field.",
            "",
            conforms,
        ),
    ] {
        match rust(path, header, own, &body) {
            Ok(f) => {
                files.retain(|x| x.path != f.path);
                files.push(f);
            }
            Err(d) => errors.push(d),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut links = Vec::new();
    for l in &libs {
        let dir = ident(&l.name);
        let link = l.conf.get("link");
        for (rel, bytes) in crate::app::runtime_files(&l.dir) {
            if rel == "runtime.conf" || rel == "README.md" {
                continue;
            }
            let path = if link == Some(&rel) { format!("build/{dir}.rs") } else { format!("src/libs/{dir}/{rel}") };
            files.push(OutFile { path, bytes, executable: false });
        }
        if link.is_some() {
            links.push(dir);
        }
    }
    files.push(plain("src/libs/mod.rs", crate::app::libs_file(m, &libs).map_err(|d| vec![d])?, false));
    files.push(plain(
        "src/main.rs",
        format!(
            "//! `{exe}`: the app `{}`, run by the `{}` library's runtime.\n\nfn main() -> std::process::ExitCode {{\n    program::libs::{}::main(&program::manifest::APP)\n}}\n",
            m.qualified,
            entry.name,
            ident(&entry.name)
        ),
        false,
    ));
    files.push(plain("src/bin/checks.rs", checks_main().to_string(), false));
    let mut toml = cargo_toml(&exe, &[(&exe, "src/main.rs"), ("checks", "src/bin/checks.rs")]);
    if !links.is_empty() {
        toml = toml.replace("publish = false\n", "publish = false\nbuild = \"build.rs\"\n");
        let mods: Vec<String> = links.iter().map(|l| format!("#[path = \"build/{l}.rs\"]\nmod {l};\n")).collect();
        let calls: Vec<String> = links.iter().map(|l| format!("    {l}::link();\n")).collect();
        files.push(plain(
            "build.rs",
            format!("//! Links the C libraries the app's runtimes call, each as its runtime says.\n\n{}\nfn main() {{\n{}}}\n", mods.concat(), calls.concat()),
            false,
        ));
    }
    files.push(plain("Cargo.toml", toml, false));
    files.push(plain("bin/test", TEST_SCRIPT.to_string(), true));
    files.push(plain("bin/check", CHECK_SCRIPT.to_string(), true));
    files.push(plain("bin/build", BUILD_SCRIPT.to_string(), true));
    files.push(crate::provenance::file(&files));
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}
