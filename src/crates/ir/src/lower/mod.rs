//! AST to IR. Six passes, each over every module:
//!
//! 1. parse every file;
//! 2. declare: one table entry per declaration, names into module scopes;
//! 3. resolve `use` items, and refuse import cycles;
//! 4. classify `type` bodies (alias, record, sum, rule set) and declare variants;
//! 5. lower declared types: fields, parameters, states, constants;
//! 6. lower bodies and notes.
//!
//! Errors are collected per declaration so one bad declaration does not hide
//! the others; a pass that depends on a failed one is skipped.

mod declare;
mod decls;
mod expr;
mod notes;
mod pattern;
mod scope;
mod stmt;
mod text;
mod ty;

use crate::error::IrError;
use crate::ids::*;
use crate::load::LoadFailure;
use crate::program::*;
use scope::Scopes;
use spec_lang_syntax::{SourceFile, SourceMap, ast, parse_file};

pub(crate) fn lower_program(sources: Vec<(String, String, String)>) -> Result<Program, LoadFailure> {
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
    let mut lw = Lowerer::new(map);
    lw.run(files);
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
}

impl Lowerer {
    fn new(sources: SourceMap) -> Self {
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
        };
        declare::add_core(&mut prog);
        Lowerer {
            prog,
            scopes: Scopes::default(),
            errors: Vec::new(),
            asts: vec![None],
            pending: Vec::new(),
            sigs: Self::core_sigs(),
        }
    }

    fn run(&mut self, files: Vec<(String, String, ast::File)>) {
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
            });
            self.asts.push(Some(file));
        }
        self.scopes.ensure(self.prog.modules.len());
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
    }

    fn core_sigs() -> std::collections::HashMap<crate::expr::Callee, Vec<(String, bool)>> {
        let mut m = std::collections::HashMap::new();
        m.insert(crate::expr::Callee::Variant(TypeId(0), SOME_TAG), vec![("value".to_string(), false)]);
        m.insert(crate::expr::Callee::Variant(TypeId(1), OK_TAG), vec![("value".to_string(), false)]);
        m.insert(crate::expr::Callee::Variant(TypeId(1), ERR_TAG), vec![("error".to_string(), false)]);
        m.insert(crate::expr::Callee::Effect(crate::expr::Effect::OutLine), vec![("text".to_string(), false)]);
        m
    }

    pub(crate) fn err(&mut self, e: IrError) {
        self.errors.push(e);
    }
}
