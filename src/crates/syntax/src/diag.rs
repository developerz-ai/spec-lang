//! Syntax diagnostics.

use crate::span::{SourceMap, Span};
use std::fmt;

/// Which stage refused the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagKind {
    Lex,
    Parse,
}

/// One syntax error at one place. Rendered as `path:line:col: message`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagKind,
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub fn lex(span: Span, message: impl Into<String>) -> Self {
        Diagnostic { kind: DiagKind::Lex, span, message: message.into() }
    }

    pub fn parse(span: Span, message: impl Into<String>) -> Self {
        Diagnostic { kind: DiagKind::Parse, span, message: message.into() }
    }

    pub fn render(&self, map: &SourceMap) -> String {
        format!("{}: {}", map.location(self.span), self)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            DiagKind::Lex => write!(f, "cannot read: {}", self.message),
            DiagKind::Parse => write!(f, "does not parse: {}", self.message),
        }
    }
}

impl std::error::Error for Diagnostic {}
