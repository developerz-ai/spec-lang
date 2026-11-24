//! `spec-lang build <project> [--app <name>] --out <dir>`: check the program,
//! then hand each app to the printer its `@target` names. Tests are not run
//! here; `spec-lang test` is the gate before a build.
//! `spec-lang build <project> --target <t> --tests --out <dir>`: the whole
//! program's checks as a project whose `bin/test` runs them (decision 009).
//! Printers over the LIR are listed in [`printers`].

use crate::output::emit;
use spec_lang_diag::Diag;
use spec_lang_ir::{AppDecl, Program};
use spec_lang_lir::{Lir, OutFile, Printer};
use std::path::{Path, PathBuf};

/// One written file, whatever crate printed it.
struct File {
    path: String,
    bytes: Vec<u8>,
    executable: bool,
}

/// Every printer, by the `@target` name it serves.
const TARGETS: &[&str] = &["rust", "cpp", "bun", "ruby"];

/// The printers over the LIR, one line each.
fn printers() -> Vec<&'static dyn Printer> {
    vec![&emit_ruby::PRINTER, &emit_ts::TsPrinter, &spec_lang_emit_cpp::PRINTER, &spec_lang_emit_rust::RustPrinter]
}

fn printer_for(target: &str) -> Option<&'static dyn Printer> {
    printers().into_iter().find(|p| p.targets().contains(&target))
}

fn from_lir(files: Vec<OutFile>) -> Vec<File> {
    files.into_iter().map(|f| File { path: f.path, bytes: f.bytes, executable: f.executable }).collect()
}

fn lowered(prog: &Program) -> Result<Lir, Vec<Diag>> {
    spec_lang_lir::lower(prog)
}

fn print(prog: &Program, app: &AppDecl) -> Result<Vec<File>, Vec<Diag>> {
    // The old emitters embed one runtime, for apps of `rules: [...]`; an app with a
    // library's shape needs a printer over the LIR and that library's runtime (decision 009).
    if let (Some(library), None) = (&app.shape, printer_for(app.target.as_deref().unwrap_or(""))) {
        return Err(vec![
            Diag::error(
                "E-UNSUPPORTED",
                app.info.name_span,
                format!("app {} uses the `{library}` vocabulary, which no printer serves yet", app.info.name),
            )
            .with_decl(prog.qualified(&app.info)),
        ]);
    }
    let target = app.target.as_deref().unwrap_or("");
    if let Some(p) = printer_for(target) {
        return lowered(prog).and_then(|lir| p.app(&lir, &app.info.name)).map(from_lir);
    }
    Err(vec![
        Diag::error(
            "E-NOTE-ARG",
            app.info.name_span,
            format!("no printer for `@target({target})`: one of {}", TARGETS.join(", ")),
        )
        .with_decl(prog.qualified(&app.info)),
    ])
}

pub fn run(args: &[String]) -> u8 {
    let mut project = None;
    let mut app = None;
    let mut out = None;
    let mut target = None;
    let mut tests = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--app" => app = it.next().cloned(),
            "--target" => target = it.next().cloned(),
            "--tests" => tests = true,
            "--out" => out = it.next().map(PathBuf::from),
            s if project.is_none() && !s.starts_with("--") => project = Some(s.to_string()),
            s => {
                eprintln!("error: unexpected argument {s:?}");
                return 2;
            }
        }
    }
    let (Some(project), Some(out)) = (project, out) else {
        eprintln!(
            "usage: spec-lang build <project> [--app <name>] --out <dir>\n       spec-lang build <project> --target <t> --tests --out <dir>"
        );
        return 2;
    };
    if tests != target.is_some() {
        eprintln!("error: --tests and --target go together");
        return 2;
    }
    let prog = match spec_lang_ir::load_project(Path::new(&project)) {
        Ok(p) => p,
        Err(f) => {
            eprintln!("{}", f.render());
            return 1;
        }
    };
    let errors = spec_lang_check::check(&prog);
    if !errors.is_empty() {
        errors.iter().for_each(|d| eprint!("{}", d.human(&prog.sources)));
        return 1;
    }
    if let Some(t) = target {
        let Some(p) = printer_for(&t) else {
            eprintln!("error: no printer over the LIR for target {t:?}");
            return 2;
        };
        return match lowered(&prog).and_then(|lir| p.tests(&lir)).and_then(|fs| {
            let files = from_lir(fs);
            write(&out, &files).map(|()| files.len())
        }) {
            Ok(n) => {
                emit(&format!("{project} checks, {t}  {n} files -> {}\n", out.display()));
                0
            }
            Err(ds) => {
                ds.iter().for_each(|d| eprint!("{}", d.human(&prog.sources)));
                1
            }
        };
    }
    let apps: Vec<&AppDecl> = prog.apps.iter().filter(|a| app.as_ref().is_none_or(|n| *n == a.info.name)).collect();
    if apps.is_empty() {
        eprintln!("error: no app{} in {project}", app.map(|n| format!(" named {n}")).unwrap_or_default());
        return 1;
    }
    let single = app.is_some();
    let mut failed = false;
    for a in apps {
        let dir = if single { out.clone() } else { out.join(&a.info.name) };
        match print(&prog, a).and_then(|files| write(&dir, &files).map(|()| files.len())) {
            Ok(n) => emit(&format!("{}  {n} files -> {}\n", prog.qualified(&a.info), dir.display())),
            Err(ds) => {
                failed = true;
                ds.iter().for_each(|d| eprint!("{}", d.human(&prog.sources)));
            }
        }
    }
    u8::from(failed)
}

fn write(dir: &Path, files: &[File]) -> Result<(), Vec<Diag>> {
    let fail = |p: &Path, e: std::io::Error| vec![Diag::load(format!("{}: {e}", p.display()))];
    for f in files {
        let path = dir.join(&f.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| fail(parent, e))?;
        }
        std::fs::write(&path, &f.bytes).map_err(|e| fail(&path, e))?;
        #[cfg(unix)]
        if f.executable {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).map_err(|e| fail(&path, e))?;
        }
    }
    Ok(())
}
