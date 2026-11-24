//! Ruby names for spec-lang names: module paths, constants, methods, locals.
//! A name Ruby cannot take (a keyword, a method every object has, a core
//! constant) gets a trailing underscore; nothing else is renamed.

use spec_lang_ir::{ModuleId, TypeId};
use spec_lang_lir::{Lir, TypeDef};
use std::collections::BTreeSet;

/// Words a Ruby local, method or keyword argument cannot be.
const KEYWORDS: &[&str] = &[
    "alias", "and", "begin", "break", "case", "class", "def", "defined", "do", "else", "elsif", "end", "ensure",
    "false", "for", "if", "in", "module", "next", "nil", "not", "or", "redo", "rescue", "retry", "return", "self",
    "super", "then", "true", "undef", "unless", "until", "when", "while", "yield",
];

/// Methods every Ruby object or module answers and the runtime relies on.
const METHODS: &[&str] = &[
    "allocate",
    "ancestors",
    "binding",
    "class",
    "clone",
    "deconstruct",
    "define_method",
    "display",
    "dup",
    "extend",
    "freeze",
    "hash",
    "include",
    "initialize",
    "inspect",
    "itself",
    "lambda",
    "members",
    "method",
    "methods",
    "name",
    "new",
    "object_id",
    "private",
    "proc",
    "public",
    "raise",
    "require",
    "send",
    "tap",
    "to_h",
    "to_s",
    "with",
];

/// Top-level constants a module of the program may not shadow: Ruby's and the runtime's.
const CONSTANTS: &[&str] = &[
    "App",
    "Array",
    "BasicObject",
    "Binding",
    "Checks",
    "Class",
    "Comparable",
    "Data",
    "Dir",
    "Encoding",
    "Enumerable",
    "Enumerator",
    "Errno",
    "Exception",
    "File",
    "Float",
    "Hash",
    "IO",
    "Integer",
    "Kernel",
    "Libs",
    "Marshal",
    "Math",
    "Method",
    "Module",
    "Numeric",
    "Object",
    "Proc",
    "Process",
    "Random",
    "Range",
    "Rational",
    "Regexp",
    "Set",
    "Signal",
    "Spec",
    "String",
    "Struct",
    "Symbol",
    "Thread",
    "Time",
    "Warning",
];

/// A local variable, parameter or keyword argument.
pub(crate) fn local(name: &str) -> String {
    if name == "self" {
        return "this".into();
    }
    let lowered = match name.chars().next() {
        Some(c) if c.is_uppercase() => format!("{}{}_", c.to_lowercase(), &name[c.len_utf8()..]),
        _ => name.to_string(),
    };
    if KEYWORDS.contains(&lowered.as_str()) { format!("{lowered}_") } else { lowered }
}

/// A module method: a `fn`, a rule, a const, a state reader.
pub(crate) fn method(name: &str) -> String {
    let n = local(name);
    if METHODS.contains(&n.as_str()) { format!("{n}_") } else { n }
}

/// A member of a record or of a variant. A value has no `name` of its own, so a field may be called that.
pub(crate) fn member(name: &str) -> String {
    if name == "name" { name.to_string() } else { method(name) }
}

/// `task_no` or `taskNo` as a constant: `TaskNo`.
pub(crate) fn constant(name: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for c in name.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    if out.is_empty() { "X".into() } else { out }
}

/// The Ruby names of every module, type and state of a program.
#[derive(Debug)]
pub(crate) struct Names {
    /// Per module: `["Todo", "Board"]`.
    modules: Vec<Vec<String>>,
    /// Per type: its constant inside its module.
    types: Vec<String>,
    /// Per state: its member of the world.
    pub(crate) world: Vec<String>,
    /// Every constant path the program defines, `Todo::Board::Task`.
    taken: BTreeSet<String>,
}

impl Names {
    pub(crate) fn new(lir: &Lir) -> Self {
        let modules: Vec<Vec<String>> = lir
            .modules
            .iter()
            .map(|m| {
                let mut path: Vec<String> = m.name.split('.').map(constant).collect();
                if let Some(first) = path.first_mut()
                    && CONSTANTS.contains(&first.as_str())
                {
                    first.push('_');
                }
                path
            })
            .collect();
        let types: Vec<String> = lir
            .types
            .iter()
            .map(|t| {
                let c = constant(&t.info.name);
                if CONSTANTS.contains(&c.as_str()) { format!("{c}_") } else { c }
            })
            .collect();
        let mut taken = BTreeSet::new();
        for m in &modules {
            for i in 1..=m.len() {
                taken.insert(m[..i].join("::"));
            }
        }
        for (t, name) in lir.types.iter().zip(&types) {
            let module = modules.get(t.info.module.index()).map(|m| m.join("::")).unwrap_or_default();
            taken.insert(format!("{module}::{name}"));
            if let TypeDef::Sum { variants } = &t.def {
                for v in variants {
                    taken.insert(format!("{module}::{name}::{}", constant(&v.name)));
                }
            }
        }
        Names { modules, types, world: world_members(lir), taken }
    }

    pub(crate) fn module(&self, m: ModuleId) -> &[String] {
        self.modules.get(m.index()).map_or(&[], Vec::as_slice)
    }

    pub(crate) fn type_const(&self, t: TypeId) -> &str {
        self.types.get(t.index()).map_or("X", String::as_str)
    }

    /// `path` as written from inside module `from` (`None`: from the top level). Ruby looks a constant up in
    /// every enclosing module first, so a path whose first part another enclosing module also defines is rooted.
    pub(crate) fn path_from(&self, from: Option<&[String]>, path: &[String], rooted: bool) -> String {
        let joined = path.join("::");
        let Some(first) = path.first() else { return joined };
        let shadowed = from.is_some_and(|scope| {
            (1..=scope.len()).any(|i| self.taken.contains(&format!("{}::{first}", scope[..i].join("::"))))
        });
        if rooted || shadowed { format!("::{joined}") } else { joined }
    }
}

/// A state's member of the world is its name; two states of one name take their module's last part first.
fn world_members(lir: &Lir) -> Vec<String> {
    let plain: Vec<String> = lir.states.iter().map(|s| method(&s.info.name)).collect();
    lir.states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if plain.iter().filter(|n| **n == plain[i]).count() == 1 {
                return plain[i].clone();
            }
            method(&s.info.qualified.replace('.', "_"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_ruby_can_take() {
        assert_eq!(local("end"), "end_");
        assert_eq!(local("self"), "this");
        assert_eq!(local("Left"), "left_");
        assert_eq!(method("hash"), "hash_");
        assert_eq!(constant("task_no"), "TaskNo");
        assert_eq!(constant("none"), "None");
    }
}
