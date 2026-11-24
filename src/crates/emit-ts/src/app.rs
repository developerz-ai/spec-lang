//! An app's project: the program and its checks, the runtimes of the app's libraries copied from
//! `libs/<name>/real/ts/` to `src/lib/<name>/`, and generic glue: `src/app.ts` (the manifest as data) and
//! `src/main.ts`, which hands both to the library that runs the app. The printer reads one note itself:
//! `bin`, the executable's name, for the build script.

use crate::file::TsFile;
use crate::machine::note;
use crate::module::DO_NOT_EDIT;
use crate::names::{camel, kebab, string_literal};
use crate::plan::Plan;
use crate::project::{self, script, text};
use spec_lang_diag::{Diag, Severity};
use spec_lang_lir::manifest::{Manifest, Named, NoteValue};
use spec_lang_lir::program::Body;
use spec_lang_lir::{Lir, OutFile};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn error(msg: String) -> Vec<Diag> {
    vec![Diag::new(Severity::Error, "E-UNSUPPORTED", msg)]
}

pub fn project(plan: &Plan<'_>, name: &str, program: Vec<OutFile>) -> Result<Vec<OutFile>, Vec<Diag>> {
    let lir = plan.lir;
    let Some(app) = lir.apps.iter().find(|a| a.name == name) else {
        return Err(error(format!("no app named {name}")));
    };
    let bin = bin_name(app);
    let package = kebab(&app.name);
    let mut files = project::common(&package, &format!("app {}", app.qualified));
    files.extend(program);

    let mut libraries = Vec::new();
    let mut builds = Vec::new();
    for lib in libraries_of(lir, app) {
        let dir = runtime_dir(lir, &lib).ok_or_else(|| {
            error(format!(
                "app {} uses library `{lib}`, which has no TypeScript runtime (libs/{lib}/real/ts/)",
                app.qualified
            ))
        })?;
        let mut copied = Vec::new();
        copy(&dir, &dir, &format!("src/lib/{lib}"), &mut copied)
            .map_err(|e| error(format!("library `{lib}`: {}: {e}", dir.display())))?;
        if copied.iter().any(|f: &OutFile| f.path == format!("src/lib/{lib}/build.sh")) {
            builds.push(format!("src/lib/{lib}/build.sh"));
        }
        files.extend(copied);
        libraries.push(lib);
    }
    files.push(text("src/app.ts", manifest_file(plan, app)));
    files.push(text("src/main.ts", main_file(app, &libraries)));
    files.extend(scripts(app, &bin, &builds));
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// `@bin(name)`, or the app's name.
fn bin_name(app: &Manifest) -> String {
    app.notes
        .iter()
        .find(|n| n.name == "bin")
        .and_then(|n| n.args.first())
        .and_then(|a| if let NoteValue::Text(t) = &a.value { Some(t.clone()) } else { None })
        .unwrap_or_else(|| kebab(&app.name))
}

/// Every library the app reaches: its notes and shape, its rules' triggers, the handlers, the foreign calls.
fn libraries_of(lir: &Lir, app: &Manifest) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    out.extend(app.notes.iter().map(|n| n.library.clone()));
    out.extend(app.shape.iter().cloned());
    for r in &app.rules {
        out.extend(lir.rule(*r).triggers.iter().map(|t| t.library.clone()));
    }
    out.extend(lir.handlers.iter().map(|h| h.library.clone()));
    for f in &lir.fns {
        if let Body::Extern(x) = &f.body {
            out.insert(x.library.clone());
        }
    }
    for r in &lir.rules {
        if let Body::Extern(x) = &r.body {
            out.insert(x.library.clone());
        }
    }
    out.retain(|l| !l.is_empty());
    out
}

/// `<library>/real/ts/`: where `use` found the library, or beside a library it found.
fn runtime_dir(lir: &Lir, lib: &str) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = lir.libraries.iter().filter(|l| l.name == lib).map(|l| PathBuf::from(&l.dir)).collect();
    for l in &lir.libraries {
        if let Some(parent) = Path::new(&l.dir).parent() {
            candidates.push(parent.join(lib));
        }
    }
    candidates.into_iter().map(|d| d.join("real").join("ts")).find(|d| d.is_dir())
}

/// Every file under `dir`, in name order, to `to`.
fn copy(root: &Path, dir: &Path, to: &str, out: &mut Vec<OutFile>) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?.map(|e| e.map(|e| e.path())).collect::<Result<_, _>>()?;
    entries.sort();
    for p in entries {
        if p.is_dir() {
            copy(root, &p, to, out)?;
            continue;
        }
        let rel = p.strip_prefix(root).map_err(std::io::Error::other)?;
        let rel: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
        let bytes = std::fs::read(&p)?;
        #[cfg(unix)]
        let executable = {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(&p)?.permissions().mode() & 0o111 != 0
        };
        #[cfg(not(unix))]
        let executable = false;
        out.push(OutFile { path: format!("{to}/{}", rel.join("/")), bytes, executable });
    }
    Ok(())
}

fn rule_info(lir: &Lir, r: spec_lang_ir::RuleId) -> String {
    format!("{}Rule", camel(&lir.rule(r).info.qualified.replace('.', "_")))
}

fn fn_info(lir: &Lir, f: spec_lang_ir::FnId) -> String {
    format!("{}Fn", camel(&lir.func(f).info.qualified.replace('.', "_")))
}

/// `src/app.ts`: the manifest as data.
fn manifest_file(plan: &Plan<'_>, app: &Manifest) -> String {
    let lir = plan.lir;
    let header = format!("Generated by spec-lang build from app {}: its manifest. {DO_NOT_EDIT}", app.qualified);
    let mut f = TsFile::new("src/app.ts", header);
    f.rt_type();
    f.type_only("src/machine.ts", "World");
    let mut used = BTreeSet::new();
    let named = |n: &Named, used: &mut BTreeSet<String>| match n {
        Named::Rule(r) => {
            let i = rule_info(lir, *r);
            used.insert(i.clone());
            format!("{{ kind: \"rule\", rule: {i} }}")
        }
        Named::Fn(x) => {
            let i = fn_info(lir, *x);
            used.insert(i.clone());
            format!("{{ kind: \"fn\", fn: {i} }}")
        }
    };
    let params: Vec<String> = app
        .params
        .iter()
        .map(|p| {
            let decls: Vec<String> = p.decls.iter().map(|d| named(d, &mut used)).collect();
            format!("{{ name: {}, decls: [{}] }}", string_literal(&p.name), decls.join(", "))
        })
        .collect();
    let rules: Vec<String> = app.rules.iter().map(|r| rule_info(lir, *r)).collect();
    used.extend(rules.iter().cloned());
    for u in &used {
        f.value("src/program.ts", u);
    }
    let opt = |s: &Option<String>| s.as_ref().map_or_else(|| "undefined".to_string(), |s| string_literal(s));
    f.blank();
    f.open("export const app: rt.App<World> = {");
    f.line(format!("name: {},", string_literal(&app.name)));
    f.line(format!("qualified: {},", string_literal(&app.qualified)));
    f.line("why: undefined,");
    f.line(format!("target: {},", opt(&app.target)));
    f.line(format!("shape: {},", opt(&app.shape)));
    let notes: Vec<String> = app.notes.iter().map(note).collect();
    f.line(format!("notes: [{}],", notes.join(", ")));
    f.open("params: [");
    for p in params {
        f.line(format!("{p},"));
    }
    f.close("],");
    f.line(format!("rules: [{}],", rules.join(", ")));
    f.close("};");
    f.finish()
}

/// `src/main.ts`: the app, the program and the libraries, handed to the library that runs the app.
fn main_file(app: &Manifest, libraries: &[String]) -> String {
    let header = format!("Generated by spec-lang build from app {}: the entry point. {DO_NOT_EDIT}", app.qualified);
    let mut f = TsFile::new("src/main.ts", header);
    f.rt();
    f.value("src/app.ts", "app");
    f.value("src/program.ts", "program");
    let names: Vec<String> = libraries.iter().map(|l| camel(l)).collect();
    for (l, n) in libraries.iter().zip(&names) {
        f.value(&format!("src/lib/{l}/index.ts"), n);
    }
    f.blank();
    f.line(format!("rt.start(app, program, [{}]);", names.join(", ")));
    f.finish()
}

fn scripts(app: &Manifest, bin: &str, builds: &[String]) -> Vec<OutFile> {
    let from = format!("# Generated by spec-lang build from app {}. {DO_NOT_EDIT}", app.qualified);
    let mut build = format!("#!/usr/bin/env bash\n{from}\n# Builds the standalone executable dist/{bin}.\nset -euo pipefail\ncd \"$(dirname \"$0\")/..\"\n");
    for b in builds {
        build.push_str(&format!("{b}\n"));
    }
    build.push_str(&format!(
        "bun build --compile --minify --bytecode --no-compile-autoload-dotenv --no-compile-autoload-bunfig src/main.ts --outfile dist/{bin}\n"
    ));
    let check = format!(
        "#!/usr/bin/env bash\n{from}\n# The whole gate: lint and format, types, the program's checks, build.\nset -euo pipefail\ncd \"$(dirname \"$0\")/..\"\nbun install --frozen-lockfile\nbunx biome check .\nbunx tsc --noEmit -p .\nbin/build\nbun test\n"
    );
    vec![script("bin/build", build), script("bin/check", check)]
}
