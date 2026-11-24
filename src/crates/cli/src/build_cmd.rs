//! `spec-lang build <project> [--app <name>] --out <dir>`: check the program,
//! then hand each app to the printer its `@target` names. Tests are not run
//! here; `spec-lang test` is the gate before a build.

use crate::output::emit;
use spec_lang_diag::Diag;
use spec_lang_ir::{AppDecl, Program};
use std::path::{Path, PathBuf};

/// One written file, whatever crate printed it.
struct File {
    path: String,
    bytes: Vec<u8>,
    executable: bool,
}

/// Every printer, by the `@target` name it serves.
const TARGETS: &[&str] = &["rust", "cpp", "bun", "ruby"];

fn print(prog: &Program, app: &AppDecl) -> Result<Vec<File>, Vec<Diag>> {
    let target = app.target.as_deref().unwrap_or("");
    match target {
        "rust" => spec_lang_emit_rust::emit(prog, app).map(|fs| {
            fs.into_iter().map(|f| File { path: f.path, bytes: f.bytes, executable: f.executable }).collect()
        }),
        "cpp" => spec_lang_emit_cpp::emit(prog, app).map(|fs| {
            fs.into_iter().map(|f| File { path: f.path, bytes: f.bytes, executable: f.executable }).collect()
        }),
        "bun" => emit_ts::emit(prog, app)
            .map(|fs| fs.into_iter().map(|f| File { path: f.path, bytes: f.bytes, executable: f.executable }).collect())
            .map_err(|d| vec![*d]),
        "ruby" => emit_ruby::emit(prog, app)
            .map(|fs| fs.into_iter().map(|f| File { path: f.path, bytes: f.bytes, executable: f.executable }).collect())
            .map_err(|d| vec![*d]),
        other => Err(vec![
            Diag::error(
                "E-NOTE-ARG",
                app.info.name_span,
                format!("no printer for `@target({other})`: one of {}", TARGETS.join(", ")),
            )
            .with_decl(prog.qualified(&app.info)),
        ]),
    }
}

pub fn run(args: &[String]) -> u8 {
    let mut project = None;
    let mut app = None;
    let mut out = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--app" => app = it.next().cloned(),
            "--out" => out = it.next().map(PathBuf::from),
            s if project.is_none() && !s.starts_with("--") => project = Some(s.to_string()),
            s => {
                eprintln!("error: unexpected argument {s:?}");
                return 2;
            }
        }
    }
    let (Some(project), Some(out)) = (project, out) else {
        eprintln!("usage: spec-lang build <project> [--app <name>] --out <dir>");
        return 2;
    };
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
