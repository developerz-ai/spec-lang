//! Lowering the typed front to C++ target trees: one module per concern.

pub mod board;
pub mod cli;
pub mod expr;
pub mod model;
pub mod project;
pub mod store;
pub mod tests;

use std::collections::{BTreeSet, HashMap};

use spec_lang_diag::Diag;
use spec_lang_emit_rust::front::Front;
use spec_lang_emit_rust::front::typed::T;
use spec_lang_emit_rust::provenance::Origin;
use spec_lang_ir::{DeclInfo, ModuleId, TypeDef, TypeId};
use spec_lang_syntax::Span;

use crate::ast::Ty;

/// What every lowering step reads: the typed program and the names it maps to.
#[derive(Debug)]
pub struct Gen<'f, 'p> {
    pub f: &'f Front<'p>,
    /// The app's namespace and include folder: the `@bin` name.
    pub ns: String,
    /// Each product module's short name: `todo.model` is `model`.
    pub mods: HashMap<ModuleId, String>,
    /// Strategies switched off.
    pub options: spec_lang_emit_rust::Options,
}

impl<'f, 'p> Gen<'f, 'p> {
    pub fn new(f: &'f Front<'p>, ns: String, options: spec_lang_emit_rust::Options) -> Self {
        let prog = f.prog;
        let product: Vec<&spec_lang_ir::Module> = prog
            .modules
            .iter()
            .filter(|m| {
                !m.path.is_empty()
                    && m.decls.iter().any(|d| match d {
                        spec_lang_ir::DeclRef::Type(t) => !matches!(prog.ty(*t).def, TypeDef::Rules(_)),
                        spec_lang_ir::DeclRef::Fn(_)
                        | spec_lang_ir::DeclRef::State(_)
                        | spec_lang_ir::DeclRef::Rule(_) => true,
                        _ => false,
                    })
            })
            .collect();
        let split: Vec<Vec<&str>> = product.iter().map(|m| m.name.split('.').collect()).collect();
        let mut common = 0;
        while split.iter().all(|s| s.len() > common + 1 && s.get(common) == split[0].get(common)) && !split.is_empty() {
            common += 1;
        }
        let mods = product.iter().zip(&split).map(|(m, s)| (m.id, s[common..].join("_"))).collect();
        Gen { f, ns, mods, options }
    }

    /// Whether a strategy of `spec_lang_emit_rust::STRATEGIES` may be used.
    pub fn on(&self, strategy: &str) -> bool {
        !self.options.off.contains(strategy)
    }

    pub fn module_of(&self, info: &DeclInfo) -> &str {
        self.mods.get(&info.module).map_or("", String::as_str)
    }

    /// `model::Title`.
    pub fn type_path(&self, id: TypeId) -> String {
        let d = self.f.prog.ty(id);
        format!("{}::{}", self.module_of(&d.info), d.info.name)
    }

    /// The value type of `t`.
    pub fn cty(&self, t: &T) -> Ty {
        match t {
            T::Unit => Ty::Void,
            T::Bool => Ty::name("bool"),
            T::Int => Ty::name("std::int64_t"),
            T::Byte => Ty::name("unsigned char"),
            T::Text => Ty::name("std::string"),
            T::Named(id) => Ty::Name(self.type_path(*id)),
            T::Option(x) => Ty::generic("std::optional", vec![self.cty(x)]),
            T::List(x) => Ty::generic("std::vector", vec![self.cty(x)]),
            T::Set(x) => Ty::generic("std::set", vec![self.cty(x)]),
            T::Map(k, v) => Ty::generic("std::map", vec![self.cty(k), self.cty(v)]),
            T::Tuple(xs) if xs.len() == 2 => Ty::generic("std::pair", xs.iter().map(|x| self.cty(x)).collect()),
            T::Tuple(xs) => Ty::generic("std::tuple", xs.iter().map(|x| self.cty(x)).collect()),
            T::World => Ty::name("board::World"),
            T::Event(_) => Ty::name("board::Event"),
            T::Reply | T::Step(_) | T::Trace(_) => Ty::Auto,
        }
    }

    /// Small values pass by value; text as `std::string_view`; the rest by `const&`.
    pub fn param_ty(&self, t: &T) -> Ty {
        match t {
            T::Text => Ty::name("std::string_view"),
            t if self.trivial(t) => self.cty(t),
            t => self.cty(t).cref(),
        }
    }

    /// Trivially copyable, one machine word or less.
    pub fn trivial(&self, t: &T) -> bool {
        match t {
            T::Bool | T::Int | T::Byte | T::Unit => true,
            T::Named(id) => match &self.f.prog.ty(*id).def {
                TypeDef::Sum(vs) => vs.iter().all(|v| v.fields.is_empty()),
                TypeDef::Alias(_) => self.alias_base(*id).is_some_and(|b| b == T::Int),
                _ => false,
            },
            T::Tuple(xs) => xs.iter().all(|x| self.trivial(x)),
            _ => false,
        }
    }

    pub fn alias_base(&self, id: TypeId) -> Option<T> {
        spec_lang_emit_rust::front::typer::alias_base(self.f.prog, id).map(|(b, _)| b)
    }

    /// `src/todo/board.slg:44:1`.
    pub fn at(&self, span: Span) -> String {
        self.f.at(span)
    }

    /// `todo.board.done`.
    pub fn path(&self, info: &DeclInfo) -> String {
        self.f.prog.qualified(info)
    }

    pub fn origin(&self, info: &DeclInfo, role: &str) -> Origin {
        Origin::new(self.path(info), self.at(info.span), role)
    }

    pub fn unsupported(&self, span: Span, decl: &DeclInfo, what: impl std::fmt::Display) -> Diag {
        Diag::unsupported(span, what).with_decl(self.path(decl))
    }

    /// The first line of every generated file: what it came from, and that it is generated.
    pub fn header(&self, what: &[(String, Span)]) -> String {
        let mut names = Vec::new();
        let mut files = BTreeSet::new();
        for (name, span) in what {
            let l = self.f.prog.location(*span);
            names.push(format!("{name} ({}:{})", l.path, l.line));
            files.insert(l.path);
        }
        format!("Generated by spec-lang build --target cpp from {}. Do not edit: change the spec.", names.join(", "))
    }
}

/// `TaskNo` to `task_no`.
pub fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// `add` to `Add`.
pub fn camel(name: &str) -> String {
    name.split('_')
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                Some(f) => f.to_uppercase().chain(c).collect::<String>(),
                None => String::new(),
            }
        })
        .collect()
}

/// A name that is not a C++ keyword.
pub fn ident(name: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "and",
        "auto",
        "bool",
        "break",
        "case",
        "catch",
        "char",
        "class",
        "const",
        "continue",
        "default",
        "delete",
        "do",
        "double",
        "else",
        "enum",
        "explicit",
        "export",
        "extern",
        "false",
        "float",
        "for",
        "friend",
        "goto",
        "if",
        "inline",
        "int",
        "long",
        "namespace",
        "new",
        "not",
        "operator",
        "or",
        "private",
        "protected",
        "public",
        "register",
        "return",
        "short",
        "signed",
        "sizeof",
        "static",
        "struct",
        "switch",
        "template",
        "this",
        "throw",
        "true",
        "try",
        "typedef",
        "typename",
        "union",
        "unsigned",
        "using",
        "virtual",
        "void",
        "volatile",
        "while",
    ];
    if KEYWORDS.contains(&name) { format!("{name}_") } else { name.to_string() }
}
