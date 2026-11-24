//! AST to IR. Seven passes, each over every module:
//!
//! 1. parse every file, then every library they `use` (`library`), whose
//!    contracts join the modules and whose vocabularies are registered as data;
//! 2. declare: one table entry per declaration, names into module scopes;
//! 3. resolve `use` items, and refuse import cycles;
//! 4. classify `type` bodies (alias, record, sum, rule set) and declare variants;
//! 5. lower declared types: fields, parameters, states, constants;
//! 6. lower bodies and notes;
//! 7. check what needs every body: an app field that names a rule has the
//!    trigger its library's shape asks for.
//!
//! Errors are collected per declaration so one bad declaration does not hide
//! the others; a pass that depends on a failed one is skipped.

mod apps;
mod declare;
mod decls;
mod expr;
mod library;
mod notes;
mod pattern;
mod scope;
mod stmt;
mod text;
mod ty;

use crate::error::IrError;
use crate::ids::*;
use crate::library::LibPath;
use crate::load::LoadFailure;
use crate::program::*;
use scope::Scopes;
use spec_lang_syntax::{SourceFile, SourceMap, ast, parse_file};

pub(crate) fn lower_program(sources: Vec<(String, String, String)>, libs: &LibPath) -> Result<Program, LoadFailure> {
    let mut map = SourceMap::new();
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for (name, path, text) in sources {
        let id = map.add(SourceFile::new(path.clone(), text.clone(), 1));
        match parse_file(id, &text) {
            Ok(f) => files.push((name, path, f)),
            Err(d) => errors.push(IrError::syntax(&d)),
        }
    }
    if !errors.is_empty() {
        return Err(LoadFailure { sources: map, errors });
    }
    let found = library::find_libraries(&mut files, &mut map, libs);
    let found = match found {
        Ok(f) => f,
        Err(errors) => return Err(LoadFailure { sources: map, errors }),
    };
    let mut lw = Lowerer::new(map, libs.clone());
    lw.run(files, found);
    if lw.errors.is_empty() { Ok(lw.prog) } else { Err(LoadFailure { sources: lw.prog.sources, errors: lw.errors }) }
}

/// The program being built, the module scopes, and the errors so far.
pub(crate) struct Lowerer {
    pub(crate) prog: Program,
    pub(crate) scopes: Scopes,
    pub(crate) errors: Vec<IrError>,
    /// Per module, the AST it came from (index = module id; `None` for core).
    asts: Vec<Option<ast::File>>,
    /// Per declaration, its AST, in the order declared.
    pending: Vec<(DeclRef, ast::Decl)>,
    /// Parameter names, and whether each has a default, of everything that
    /// can be called: known from the syntax, before any type is lowered.
    pub(crate) sigs: std::collections::HashMap<crate::expr::Callee, Vec<(String, bool)>>,
    /// Where `use` looked for libraries, for its errors.
    pub(crate) lib_path: LibPath,
}

impl Lowerer {
    fn new(sources: SourceMap, lib_path: LibPath) -> Self {
        let mut prog = Program {
            sources,
            modules: Vec::new(),
            types: Vec::new(),
            consts: Vec::new(),
            states: Vec::new(),
            fns: Vec::new(),
            rules: Vec::new(),
            tests: Vec::new(),
            laws: Vec::new(),
            apps: Vec::new(),
            lambdas: Vec::new(),
            option_type: TypeId(0),
            result_type: TypeId(1),
            rule_set_uses: Vec::new(),
            libraries: Vec::new(),
            handlers: Vec::new(),
        };
        declare::add_core(&mut prog);
        Lowerer {
            prog,
            scopes: Scopes::default(),
            errors: Vec::new(),
            asts: vec![None],
            pending: Vec::new(),
            sigs: Self::core_sigs(),
            lib_path,
        }
    }

    fn run(&mut self, files: Vec<(String, String, ast::File)>, libraries: Vec<library::LibSource>) {
        for (name, path, file) in files {
            if self.prog.modules.iter().any(|m| m.name == name) {
                self.errors.push(IrError::load(format!("two files make module {name}")));
                continue;
            }
            let id = ModuleId(next_id(self.prog.modules.len()));
            self.prog.modules.push(Module {
                id,
                name,
                path,
                uses: Vec::new(),
                vocabularies: Vec::new(),
                decls: Vec::new(),
                library: None,
            });
            self.asts.push(Some(file));
        }
        self.scopes.ensure(self.prog.modules.len());
        self.register_libraries(libraries);
        if !self.errors.is_empty() {
            return;
        }
        let asts = std::mem::take(&mut self.asts);
        for (i, file) in asts.iter().enumerate() {
            if let Some(f) = file {
                self.declare_module(ModuleId(next_id(i)), f);
            }
        }
        for (i, file) in asts.iter().enumerate() {
            if let Some(f) = file {
                self.resolve_uses(ModuleId(next_id(i)), f);
            }
        }
        self.check_use_cycles();
        if !self.errors.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.pending);
        for (d, decl) in &pending {
            if let DeclRef::Type(t) = d {
                self.classify_type(*t, decl);
            }
        }
        self.import_variants();
        if !self.errors.is_empty() {
            return;
        }
        for (d, decl) in &pending {
            self.lower_signature(*d, decl);
        }
        if !self.errors.is_empty() {
            return;
        }
        for (d, decl) in &pending {
            self.lower_decl_body(*d, decl);
        }
        if self.errors.is_empty() {
            self.check_app_triggers();
        }
    }

    fn core_sigs() -> std::collections::HashMap<crate::expr::Callee, Vec<(String, bool)>> {
        let mut m = std::collections::HashMap::new();
        m.insert(crate::expr::Callee::Variant(TypeId(0), SOME_TAG), vec![("value".to_string(), false)]);
        m.insert(crate::expr::Callee::Variant(TypeId(1), OK_TAG), vec![("value".to_string(), false)]);
        m.insert(crate::expr::Callee::Variant(TypeId(1), ERR_TAG), vec![("error".to_string(), false)]);
        m
    }

    pub(crate) fn err(&mut self, e: IrError) {
        self.errors.push(e);
    }
}
