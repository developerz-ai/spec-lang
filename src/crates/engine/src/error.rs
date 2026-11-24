//! Why evaluation stopped.

use crate::value::Failure;
use spec_lang_diag::Diag;
use std::rc::Rc;

/// Evaluation did not produce a value.
#[derive(Clone, Debug)]
pub enum Halt {
    /// A failure of the language (`fail E`, `Missing`, a guard): handled by
    /// `else`, reported by a rule as its refusal.
    Fail(Rc<Failure>),
    /// The engine stopped: a runtime type error, a broken claim, an overflow,
    /// an unsupported form. Never handled by `else`.
    Fatal(Box<Diag>),
    /// A test check did not hold (`assert`, `->`).
    Check(Box<Diag>),
    /// A `?hole` (`F-BLOCKED`) or a `foreign` body (`B-FOREIGN`) was
    /// reached: the check cannot run further here.
    Blocked(Box<Diag>),
}

pub type Halted<T> = Result<T, Halt>;

impl Halt {
    pub fn fail(name: &str) -> Halt {
        Halt::Fail(Rc::new(Failure { name: name.to_string(), data: Vec::new() }))
    }

    pub fn missing() -> Halt {
        Halt::fail("Missing")
    }

    /// A `?hole` was reached.
    pub fn hole(name: &str, span: spec_lang_syntax::Span) -> Halt {
        Halt::Blocked(Box::new(
            Diag::new(spec_lang_diag::Severity::Failure, "F-BLOCKED", format!("reached ?{name}")).with_span(span),
        ))
    }

    /// A call reached a `foreign` declaration `decl`: only a backend runs it.
    pub fn foreign(decl: &str, f: &spec_lang_ir::Foreign) -> Halt {
        Halt::Blocked(Box::new(
            Diag::new(
                spec_lang_diag::Severity::Blocked,
                "B-FOREIGN",
                format!("reached {decl}, which is {f}: only a backend runs it"),
            )
            .with_span(f.span)
            .with_expected("a body the interpreter runs", f.to_string())
            .with_note("card, Altitude is the form of the body: foreign evidence starts at assumed; a backend linked to the symbol checks it"),
        ))
    }
}

impl From<Diag> for Halt {
    fn from(d: Diag) -> Self {
        Halt::Fatal(Box::new(d))
    }
}
