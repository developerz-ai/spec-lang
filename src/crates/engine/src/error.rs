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
    /// A `?hole` was reached.
    Blocked(String, spec_lang_syntax::Span),
}

pub type Halted<T> = Result<T, Halt>;

impl Halt {
    pub fn fail(name: &str) -> Halt {
        Halt::Fail(Rc::new(Failure { name: name.to_string(), data: Vec::new() }))
    }

    pub fn missing() -> Halt {
        Halt::fail("Missing")
    }
}

impl From<Diag> for Halt {
    fn from(d: Diag) -> Self {
        Halt::Fatal(Box::new(d))
    }
}
