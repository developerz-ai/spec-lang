//! What every part of the printer shares: the program, its Ruby names, and the scope of the declaration
//! being printed (which module the text stands in, which locals exist and what Ruby calls them).

use crate::names::{self, Names};
use spec_lang_diag::Diag;
use spec_lang_ir::{ModuleId, TypeId};
use spec_lang_lir::{Lir, TypeDef};
use spec_lang_syntax::Span;
use std::collections::BTreeMap;

pub(crate) type R<T> = Result<T, Diag>;

#[derive(Debug)]
pub(crate) struct Gen<'a> {
    pub(crate) lir: &'a Lir,
    pub(crate) names: Names,
    /// The directory every module of the project is under, with its trailing `/`.
    root: String,
}

impl<'a> Gen<'a> {
    pub(crate) fn new(lir: &'a Lir) -> Self {
        Gen { lir, names: Names::new(lir), root: source_root(lir) }
    }

    /// A source path as a generated file names it: relative to the project, or `<library>/<file>`.
    pub(crate) fn source(&self, path: &str) -> String {
        for l in &self.lir.libraries {
            if let Some(rest) = path.strip_prefix(&l.dir) {
                return format!("{}/{}", l.name, rest.trim_start_matches('/'));
            }
        }
        path.strip_prefix(&self.root).unwrap_or(path).to_string()
    }

    /// The project's directory, as the build was given it.
    pub(crate) fn root_dir(&self) -> String {
        if self.root.is_empty() { ".".into() } else { self.root.clone() }
    }

    /// `todo/board.slg:13`.
    pub(crate) fn at(&self, span: Span) -> String {
        let l = self.lir.sources.location(span);
        format!("{}:{}", self.source(&l.path), l.line)
    }

    /// `lib/todo/board.rb` without `lib/` and `.rb`.
    pub(crate) fn module_file(&self, m: ModuleId) -> String {
        self.names.module(m).iter().map(|s| snake(s)).collect::<Vec<_>>().join("/")
    }

    pub(crate) fn type_path(&self, t: TypeId) -> Vec<String> {
        let mut path = self.names.module(self.lir.ty(t).info.module).to_vec();
        path.push(self.names.type_const(t).to_string());
        path
    }

    pub(crate) fn variant_const(&self, t: TypeId, tag: u32) -> String {
        match &self.lir.ty(t).def {
            TypeDef::Sum { variants } => variants.get(tag as usize).map_or("X".into(), |v| variant_const(&v.name)),
            _ => "X".into(),
        }
    }

    pub(crate) fn unsupported(&self, span: Span, what: impl std::fmt::Display) -> Diag {
        Diag::unsupported(span, format!("{what} (the Ruby printer)"))
    }
}

pub(crate) fn variant_const(name: &str) -> String {
    let c = names::constant(name);
    if c == "Spec" { "Spec_".into() } else { c }
}

/// `TaskNo` as a file name: `task_no`.
pub(crate) fn snake(constant: &str) -> String {
    let mut out = String::new();
    for (i, c) in constant.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(c.to_lowercase());
    }
    out
}

/// The longest directory every project module's file is under.
fn source_root(lir: &Lir) -> String {
    let in_library = |p: &str| lir.libraries.iter().any(|l| p.starts_with(&l.dir));
    let dirs: Vec<&str> = lir
        .modules
        .iter()
        .filter(|m| !m.path.is_empty() && !in_library(&m.path))
        .map(|m| m.path.rsplit_once('/').map_or("", |(d, _)| d))
        .collect();
    let Some(first) = dirs.first() else { return String::new() };
    let mut parts: Vec<&str> = first.split('/').collect();
    for d in &dirs {
        let same = parts.iter().zip(d.split('/')).take_while(|(a, b)| *a == b).count();
        parts.truncate(same);
    }
    if parts.is_empty() || (parts.len() == 1 && parts[0].is_empty()) {
        String::new()
    } else {
        format!("{}/", parts.join("/"))
    }
}

/// One local of the declaration being printed.
#[derive(Clone, Debug)]
struct Local {
    spec: String,
    ruby: String,
    /// It can hold another value later (a `var`, or a `let` in a `while` body), so a lambda that captures it
    /// must copy it.
    changes: bool,
}

/// The scope of one printed declaration.
#[derive(Debug)]
pub(crate) struct Fx<'a> {
    pub(crate) g: &'a Gen<'a>,
    /// The module whose `module ... end` the text stands in.
    pub(crate) module: Option<ModuleId>,
    /// The text is a method of that module, so its other methods are called without a receiver.
    pub(crate) bare: bool,
    /// Constant paths start at `::` (inside a sum's module, where a variant may share a type's name).
    pub(crate) rooted: bool,
    /// What every `return` value is checked against.
    pub(crate) result_check: Option<String>,
    scopes: Vec<Vec<Local>>,
    used: BTreeMap<String, u32>,
    pub(crate) whiles: u32,
    /// Parameters the prologue already checked, with the type they met.
    pub(crate) proved: Vec<(String, spec_lang_lir::DTy)>,
}

impl<'a> Fx<'a> {
    /// Text inside `module`, as one of its methods.
    pub(crate) fn method(g: &'a Gen<'a>, module: ModuleId) -> Self {
        Fx {
            g,
            module: Some(module),
            bare: true,
            rooted: false,
            result_check: None,
            scopes: vec![Vec::new()],
            used: BTreeMap::new(),
            whiles: 0,
            proved: Vec::new(),
        }
    }

    /// Text inside `module` but not a method of it (a record's body).
    pub(crate) fn inside(g: &'a Gen<'a>, module: ModuleId) -> Self {
        Fx { bare: false, ..Fx::method(g, module) }
    }

    /// Text outside every module of the program (the registry, the checks).
    pub(crate) fn outside(g: &'a Gen<'a>) -> Self {
        Fx { module: None, bare: false, ..Fx::method(g, ModuleId(0)) }
    }

    pub(crate) fn const_path(&self, path: &[String]) -> String {
        let scope = self.module.map(|m| self.g.names.module(m));
        if let Some(scope) = scope
            && !self.rooted
            && path.len() > scope.len()
            && path.starts_with(scope)
        {
            return path[scope.len()..].join("::");
        }
        self.g.names.path_from(scope, path, self.rooted)
    }

    pub(crate) fn module_ref(&self, m: ModuleId) -> String {
        let scope = self.module.map(|m| self.g.names.module(m));
        self.g.names.path_from(scope, self.g.names.module(m), self.rooted)
    }

    pub(crate) fn type_ref(&self, t: TypeId) -> String {
        self.const_path(&self.g.type_path(t))
    }

    pub(crate) fn variant_ref(&self, t: TypeId, tag: u32) -> String {
        let lir = self.g.lir;
        if t == lir.option_type {
            return if tag == spec_lang_ir::SOME_TAG { "Spec::Some".into() } else { "Spec::NONE".into() };
        }
        if t == lir.result_type {
            return if tag == spec_lang_ir::OK_TAG { "Spec::Ok".into() } else { "Spec::Err".into() };
        }
        let mut path = self.g.type_path(t);
        path.push(self.g.variant_const(t, tag));
        self.const_path(&path)
    }

    /// A method of `module` named `name`, as this text calls it. `args` says whether parentheses follow.
    pub(crate) fn callee(&self, module: ModuleId, name: &str, args: bool) -> String {
        let method = names::method(name);
        if self.bare && self.module == Some(module) {
            return if !args && self.is_local(&method) { format!("self.{method}") } else { method };
        }
        format!("{}.{method}", self.module_ref(module))
    }

    pub(crate) fn push(&mut self) {
        self.scopes.push(Vec::new());
    }

    pub(crate) fn pop(&mut self) {
        self.scopes.pop();
    }

    /// A new local made by an assignment (`let`, a pattern's binding): its Ruby name, unlike every other
    /// local of this declaration, because a Ruby block does not scope an assignment.
    pub(crate) fn bind(&mut self, spec: &str, mutable: bool) -> String {
        let ruby = self.fresh(&names::local(spec));
        self.declare(spec, ruby, mutable || self.whiles > 0)
    }

    /// A parameter of a method, a block or a lambda. Ruby scopes it, so it keeps its name and hides an outer
    /// local of that name as it does in the source; only a name some other live local goes by is changed.
    pub(crate) fn bind_param(&mut self, spec: &str) -> String {
        let base = names::local(spec);
        let hides_another = self.live().any(|l| l.ruby == base && l.spec != spec);
        let ruby = if hides_another { self.fresh(&base) } else { base };
        self.declare(spec, ruby, false)
    }

    fn declare(&mut self, spec: &str, ruby: String, changes: bool) -> String {
        if let Some(top) = self.scopes.last_mut() {
            top.push(Local { spec: spec.to_string(), ruby: ruby.clone(), changes });
        }
        ruby
    }

    fn live(&self) -> impl Iterator<Item = &Local> {
        self.scopes.iter().rev().flat_map(|s| s.iter().rev())
    }

    /// Whether `name` is a Ruby local where the text stands, so a method of that name needs `self.`.
    fn is_local(&self, name: &str) -> bool {
        self.used.contains_key(name) || self.live().any(|l| l.ruby == name)
    }

    /// A name for a value the printer holds, unlike every local.
    pub(crate) fn fresh(&mut self, base: &str) -> String {
        let mut n = self.used.get(base).copied().unwrap_or(0);
        loop {
            n += 1;
            let candidate = if n == 1 { base.to_string() } else { format!("{base}{n}") };
            let taken = (n > 1 && self.used.contains_key(&candidate)) || self.live().any(|l| l.ruby == candidate);
            if !taken {
                self.used.insert(base.to_string(), n);
                self.used.entry(candidate.clone()).or_insert(1);
                return candidate;
            }
        }
    }

    pub(crate) fn lookup(&self, spec: &str) -> String {
        self.find(spec).map_or_else(|| names::local(spec), |l| l.ruby.clone())
    }

    pub(crate) fn changes(&self, spec: &str) -> bool {
        self.find(spec).is_some_and(|l| l.changes)
    }

    fn find(&self, spec: &str) -> Option<&Local> {
        self.live().find(|l| l.spec == spec)
    }
}
