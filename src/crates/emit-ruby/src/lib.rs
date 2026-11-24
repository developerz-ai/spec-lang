//! The Ruby backend of `spec-lang build`.
//!
//! [`emit`] turns one `app` of a resolved program into a complete Ruby project:
//! generated product code and tests (a typed Ruby tree, printed), the backend's
//! fixed runtime shims, a Gemfile, scripts, and `provenance.json`. Same input,
//! same bytes. The plan and the typed tree are shared with `emit-ts`
//! (`emit_ts::sem`). See `README.md` in this crate.

pub mod rb;

pub use emit_ts::out::{EmitError, OutFile};
use emit_ts::sem::app::App;
pub use emit_ts::sem::app::Strategies;
use emit_ts::sem::prov::{Entry, Tracked};
use spec_lang_ir::{AppDecl, Program};

pub const TARGET: &str = "ruby";

/// Every file of the project with its provenance entries, before `provenance.json` is computed.
pub fn emit_tracked(program: &Program, app: &AppDecl) -> Result<Vec<Tracked>, EmitError> {
    emit_tracked_with(program, app, Strategies::default())
}

/// As [`emit_tracked`], with some strategies switched off.
pub fn emit_tracked_with(program: &Program, app: &AppDecl, strategies: Strategies) -> Result<Vec<Tracked>, EmitError> {
    let plan = App::build_with(program, app, strategies)?;
    let mut generated = rb::product::Gen::new(&plan);
    generated.product()?;
    generated.tests()?;
    let info = &app.info;
    let mut files = rb::project::files(&plan);
    for (_, file) in std::mem::take(&mut generated.files) {
        let header = Entry {
            decl: program.qualified(info),
            slg: emit_ts::sem::prov::slg(program, info, info.span),
            role: "file header".into(),
            strategy: "header.provenance: every file names the declarations it was generated from".into(),
            fact: file.sources.join(", "),
        };
        files.push(file.finish(header));
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// The whole project, `provenance.json` included.
pub fn emit(program: &Program, app: &AppDecl) -> Result<Vec<OutFile>, EmitError> {
    emit_ts::out::finish(TARGET, &program.qualified(&app.info), emit_tracked(program, app)?)
}
