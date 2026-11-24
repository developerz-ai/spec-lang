//! Diagnostics for agents first: one read says where, what, and what to do.
//!
//! Every stage (parse, check, lint, test) reports a [`Diag`]. It carries a
//! stable code (registry: `codes.md` in this crate, checked by a unit test),
//! the span, the declaration it is in (`todo.board.done`) and the place
//! inside it (`guard[1]`), expected and found in spec-lang terms, and the
//! edits that would fix it: one [`Edit`] list when the fix is mechanical,
//! several [`Alternative`]s when the fix is a question.
//!
//! [`Diag::human`] renders rustc-style with a source line and carets;
//! [`Diag::json`] renders one JSON object, same content.

mod json;
mod render;

pub use json::JsonWriter;
use spec_lang_syntax::{Diagnostic, Span};

/// Error, warning, a failed check, or a blocked one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    /// A test, case, property or law that did not hold.
    Failure,
    /// A check that could not run to the end here: it reached a `foreign`
    /// body only a backend can run.
    Blocked,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Failure => "failure",
            Severity::Blocked => "blocked",
        }
    }
}

/// Replace the text of `span` with `text`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub span: Span,
    pub text: String,
}

/// One answer to a question: what it does, and its edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alternative {
    pub does: String,
    pub edits: Vec<Edit>,
}

/// One diagnostic. Build with [`Diag::error`] and friends, then the `with_*` methods.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diag {
    pub severity: Severity,
    /// Stable code from `codes.md`.
    pub code: &'static str,
    pub message: String,
    pub span: Option<Span>,
    /// The declaration, dotted: `todo.board.done`.
    pub decl: Option<String>,
    /// Where inside the declaration: `guard[1]`, `case[2]`, `body.stmt[3]`.
    pub at: Option<String>,
    pub expected: Option<String>,
    pub found: Option<String>,
    /// The mechanical fix, if there is exactly one.
    pub edits: Vec<Edit>,
    /// The possible fixes, when there are several (an `E-OPEN` question).
    pub alternatives: Vec<Alternative>,
    /// Further facts, one per line: a counterexample, a seed, a state diff.
    pub notes: Vec<String>,
}

impl Diag {
    pub fn new(severity: Severity, code: &'static str, message: impl Into<String>) -> Self {
        Diag {
            severity,
            code,
            message: message.into(),
            span: None,
            decl: None,
            at: None,
            expected: None,
            found: None,
            edits: Vec::new(),
            alternatives: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn error(code: &'static str, span: Span, message: impl Into<String>) -> Self {
        Diag::new(Severity::Error, code, message).with_span(span)
    }

    pub fn warning(code: &'static str, span: Span, message: impl Into<String>) -> Self {
        Diag::new(Severity::Warning, code, message).with_span(span)
    }

    /// A form the toolchain does not implement: never a silent wrong answer.
    pub fn unsupported(span: Span, what: impl std::fmt::Display) -> Self {
        Diag::error("E-UNSUPPORTED", span, format!("unsupported: {what}"))
    }

    /// A file that cannot be read.
    pub fn load(message: impl Into<String>) -> Self {
        Diag::new(Severity::Error, "E-LOAD", message)
    }

    /// A syntax error from the parser.
    pub fn syntax(d: &Diagnostic) -> Self {
        let code = match d.kind {
            spec_lang_syntax::diag::DiagKind::Lex => "E-PARSE-LEX",
            spec_lang_syntax::diag::DiagKind::Parse => "E-PARSE",
        };
        let mut out = Diag::error(code, d.span, d.message.clone());
        if let Some((exp, found)) = d.message.strip_prefix("expected ").and_then(|m| m.split_once(", found ")) {
            out.expected = Some(exp.to_string());
            out.found = Some(found.to_string());
        }
        out
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_decl(mut self, decl: impl Into<String>) -> Self {
        self.decl = Some(decl.into());
        self
    }

    pub fn with_at(mut self, at: impl Into<String>) -> Self {
        self.at = Some(at.into());
        self
    }

    pub fn with_expected(mut self, expected: impl Into<String>, found: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self.found = Some(found.into());
        self
    }

    pub fn with_edit(mut self, span: Span, text: impl Into<String>) -> Self {
        self.edits.push(Edit { span, text: text.into() });
        self
    }

    pub fn with_alternative(mut self, does: impl Into<String>, edits: Vec<Edit>) -> Self {
        self.alternatives.push(Alternative { does: does.into(), edits });
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Fill the declaration path if it is not set yet.
    pub fn in_decl(mut self, decl: &str) -> Self {
        if self.decl.is_none() {
            self.decl = Some(decl.to_string());
        }
        self
    }
}

impl std::fmt::Display for Diag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]: {}", self.severity.as_str(), self.code, self.message)
    }
}

impl std::error::Error for Diag {}

/// Every code in `codes.md`, for the registry test and for tools.
pub const CODES: &str = include_str!("../codes.md");

#[cfg(test)]
mod tests;
