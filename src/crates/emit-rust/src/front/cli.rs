//! The `cli` vocabulary's derived surface (pocs/todo-app/docs/cli-vocabulary.md,
//! "Derived, never written"; contract section 2): one command per rule with
//! `@on(cli(name))`, in the app's order; one argument per parameter.

use spec_lang_diag::Diag;
use spec_lang_ir::{RuleId, Trigger, TypeDef};
use spec_lang_syntax::Span;

use super::Front;
use super::typed::{K, T};

#[derive(Clone, Debug)]
pub struct Command {
    pub rule: RuleId,
    pub name: String,
    /// Parameters without a default, in declaration order.
    pub positionals: Vec<Positional>,
    /// Parameters with a default, in declaration order.
    pub flags: Vec<Flag>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Positional {
    /// Index of the rule parameter.
    pub param: usize,
    pub name: String,
    pub ty: T,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlagKind {
    /// `Bool = false`.
    Switch,
    /// `Option[T] = none`, or a plain default: one value of `T`.
    Single(T),
    /// `Set[T]` with a default: repeatable, repeats collapse.
    Set(T),
}

#[derive(Clone, Debug)]
pub struct Flag {
    pub param: usize,
    pub name: String,
    pub kind: FlagKind,
    /// Which row of the vocabulary's table derived it, for provenance.
    pub derivation: &'static str,
}

pub fn derive(front: &Front<'_>) -> Result<Vec<Command>, Diag> {
    let mut out = Vec::new();
    for rule in &front.rules {
        let decl = front.prog.rule(rule.id);
        let Some(name) = decl.triggers.iter().map(|Trigger::Cli { command }| command.clone()).next() else {
            continue;
        };
        let mut cmd = Command { rule: rule.id, name, positionals: Vec::new(), flags: Vec::new(), span: decl.info.span };
        for (i, p) in rule.params.iter().enumerate() {
            let Some(default) = &p.default else {
                cmd.positionals.push(Positional { param: i, name: p.name.clone(), ty: p.ty.clone() });
                continue;
            };
            let (kind, derivation) = match (&p.ty, &default.kind) {
                (T::Bool, K::Bool(false)) => (FlagKind::Switch, "Bool = false: a switch --name"),
                (T::Option(t), K::Variant { .. }) => {
                    (FlagKind::Single((**t).clone()), "Option[T] = none: --name <value>, at most once")
                }
                (T::Set(t), _) => (FlagKind::Set((**t).clone()), "Set[T] with a default: --name <value>, repeatable"),
                (t, _) => {
                    (FlagKind::Single(t.clone()), "a plain default: --name <value>, at most once (contract section 2)")
                }
            };
            cmd.flags.push(Flag { param: i, name: p.name.clone(), kind, derivation });
        }
        out.push(cmd);
    }
    Ok(out)
}

/// Every value type the table parses, in order of first appearance.
pub fn value_types(cmds: &[Command]) -> Vec<T> {
    let mut out: Vec<T> = Vec::new();
    for c in cmds {
        let tys = c.positionals.iter().map(|p| &p.ty).chain(c.flags.iter().filter_map(|f| match &f.kind {
            FlagKind::Switch => None,
            FlagKind::Single(t) | FlagKind::Set(t) => Some(t),
        }));
        for t in tys {
            if !out.contains(t) {
                out.push(t.clone());
            }
        }
    }
    out
}

/// How a value is shown in `--help`: a sum's variants in lower case, else `<type name in lower case>`.
pub fn placeholder(front: &Front<'_>, t: &T) -> String {
    match t {
        T::Named(id) => match &front.prog.ty(*id).def {
            TypeDef::Sum(vs) if vs.iter().all(|v| v.fields.is_empty()) => {
                vs.iter().map(|v| v.name.to_lowercase()).collect::<Vec<_>>().join("|")
            }
            _ => format!("<{}>", front.prog.ty(*id).info.name.to_lowercase()),
        },
        T::Int => "<int>".into(),
        T::Text => "<text>".into(),
        _ => "<value>".into(),
    }
}

/// The type's name as the spec writes it, for "not a ...".
pub fn value_name(front: &Front<'_>, t: &T) -> String {
    super::typer::type_name(front.prog, t)
}

/// The help text (contract section 2): one line per command.
pub fn help(front: &Front<'_>, bin: &str) -> String {
    let mut out = format!("usage: {bin} <command> [args]\n\ncommands:\n");
    for c in &front.cli {
        out.push_str("  ");
        out.push_str(&c.name);
        for p in &c.positionals {
            out.push_str(&format!(" <{}>", p.name));
        }
        for f in &c.flags {
            match &f.kind {
                FlagKind::Switch => out.push_str(&format!(" [--{}]", f.name)),
                FlagKind::Single(t) => out.push_str(&format!(" [--{} {}]", f.name, placeholder(front, t))),
                FlagKind::Set(t) => out.push_str(&format!(" [--{} {}]...", f.name, placeholder(front, t))),
            }
        }
        out.push('\n');
    }
    out
}
