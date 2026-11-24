//! Which notes exist, where each may stand, and which this toolchain implements.
//! A library's notes come from its `vocab.slg` ([`crate::library`]).
//!
//! Card, Core notes: "closed set; one typed meaning; an unknown, misplaced or
//! contradictory note is an error".

use super::Lowerer;
use crate::error::IrError;
use crate::ids::ModuleId;
use spec_lang_syntax::ast;

/// Every core note of the card and the kinds it may stand on.
const CORE: &[(&str, &[&str])] = &[
    ("why", &["*"]),
    ("pre", &["fn", "rule", "task"]),
    ("post", &["fn", "rule", "task"]),
    ("guard", &["rule", "task"]),
    ("uses", &["fn", "rule", "task"]),
    ("fails", &["fn", "rule", "task"]),
    ("on", &["rule", "task"]),
    ("within", &["fn", "rule", "task"]),
    ("alloc", &["fn", "rule", "task"]),
    ("cases", &["fn", "test"]),
    ("given", &["test"]),
    ("retry", &["rule", "task"]),
    ("idempotent", &["rule", "task"]),
    ("pii", &["type", "state"]),
    ("secret", &["type", "state"]),
    ("tenant", &["type", "state", "rule"]),
    ("access", &["rule", "task", "entity"]),
    ("edge", &["flow"]),
    ("assume", &["*"]),
    ("target", &["app"]),
    ("memo", &["fn"]),
    ("mmio", &["layout"]),
    ("place", &["app", "rule", "state"]),
    ("allows", &["note"]),
];

/// Notes this toolchain gives meaning to, per kind.
const IMPLEMENTED: &[(&str, &[&str])] = &[
    ("type", &["why"]),
    ("const", &["why"]),
    ("state", &["why"]),
    ("fn", &["why", "cases", "uses"]),
    ("rule", &["why", "guard", "uses", "on"]),
    ("test", &["why", "given", "cases"]),
    ("law", &["why"]),
    ("app", &["why", "target"]),
];

/// The kinds a library's note may stand on that this toolchain keeps as data.
const LIBRARY_KINDS: &[&str] = &["app"];

/// Notes that may be written more than once.
const REPEATABLE: &[&str] = &["guard", "edge", "access", "retry"];

impl Lowerer {
    /// Check every note of a declaration of `kind`. Errors only; the meaning is
    /// read by the declaration lowerer.
    pub(crate) fn check_notes(&mut self, m: ModuleId, kind: &str, notes: &[ast::Note]) -> Result<(), IrError> {
        let mut seen: Vec<String> = Vec::new();
        for n in notes {
            let name = n.path.dotted();
            let core = CORE.iter().find(|(c, _)| *c == name).map(|(_, k)| *k);
            let library = match core {
                Some(_) => None,
                None => self.library_note(m, &name, n.span)?,
            };
            let places: Vec<&str> = match (core, &library) {
                (Some(k), _) => k.to_vec(),
                (None, Some(sig)) => sig.allows.iter().map(String::as_str).collect(),
                (None, None) => {
                    return Err(IrError::error(
                        "E-NOTE-UNKNOWN",
                        n.span,
                        format!("no note `@{name}`{}", self.note_hint(&name)),
                    ));
                }
            };
            if !places.contains(&"*") && !places.contains(&kind) {
                return Err(IrError::error("E-NOTE-PLACE", n.span, format!("`@{name}` does not belong on a `{kind}`"))
                    .with_expected(places.join(", "), kind.to_string()));
            }
            let implemented = match &library {
                Some(_) => LIBRARY_KINDS.contains(&kind),
                None => IMPLEMENTED.iter().any(|(k, ns)| *k == kind && ns.contains(&name.as_str())),
            };
            if !implemented {
                return Err(IrError::unsupported(n.span, format!("the note `@{name}` on a `{kind}`")));
            }
            if seen.contains(&name) && !REPEATABLE.contains(&name.as_str()) {
                return Err(IrError::error("E-NOTE-DUP", n.span, format!("`@{name}` appears twice")));
            }
            seen.push(name);
        }
        Ok(())
    }
}

/// The arguments of every note named `name`, in order.
pub(crate) fn note_args<'a>(
    notes: &'a [ast::Note],
    name: &str,
) -> impl Iterator<Item = (&'a ast::Note, &'a [ast::Arg])> {
    notes.iter().filter(move |n| n.path.dotted() == name).map(|n| (n, n.args.as_deref().unwrap_or(&[])))
}
