//! The TypeScript-on-Bun backend of `spec-lang build`.
//!
//! [`emit`] turns one `app` of a resolved program into a complete Bun project:
//! generated product code and tests (swc trees, printed), the backend's fixed
//! runtime shims, configuration, scripts, and `provenance.json`. Same input,
//! same bytes. See `README.md` in this crate.

pub mod out;
pub mod sem;
pub mod ts;

pub use out::{EmitError, OutFile};
use sem::app::App;
pub use sem::app::Strategies;
use sem::prov::{Entry, Tracked};
use spec_lang_ir::{AppDecl, Program};

pub const TARGET: &str = "bun";

/// Every file of the project with its provenance entries, before `provenance.json` is computed.
pub fn emit_tracked(program: &Program, app: &AppDecl) -> Result<Vec<Tracked>, EmitError> {
    emit_tracked_with(program, app, Strategies::default())
}

/// As [`emit_tracked`], with some strategies switched off.
pub fn emit_tracked_with(program: &Program, app: &AppDecl, strategies: Strategies) -> Result<Vec<Tracked>, EmitError> {
    let plan = App::build_with(program, app, strategies)?;
    let mut generated = ts::product::Gen::new(&plan);
    generated.product()?;
    generated.tests()?;
    let info = &app.info;
    let mut files = ts::project::files(&plan);
    for (_, file) in std::mem::take(&mut generated.files) {
        let header = Entry {
            decl: program.qualified(info),
            slg: sem::prov::slg(program, info, info.span),
            role: "file header".into(),
            strategy: "header.provenance: every file names the declarations it was generated from".into(),
            fact: file.sources.join(", "),
        };
        files.push(file.finish(header).map_err(out::internal)?);
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// The whole project, `provenance.json` included.
pub fn emit(program: &Program, app: &AppDecl) -> Result<Vec<OutFile>, EmitError> {
    out::finish(TARGET, &program.qualified(&app.info), emit_tracked(program, app)?)
}
