// Projected by hand from effect out.line, handler out.text (docs/cli-vocabulary.md). Stand-in for spec-lang build output; change the spec, not this file.

use std::fmt::{self, Write};

/// The lines a rule sent. The effect is "after commit", so a rule only
/// collects here; whoever commits the world decides when the text leaves.
/// One buffer, so the whole reply is one write.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Out {
    text: String,
}

impl Out {
    /// `out.line(text)`.
    pub fn line(&mut self, line: impl fmt::Display) {
        // `fmt::Write` for String never fails; the Result belongs to the trait.
        let _ = writeln!(self.text, "{line}");
    }

    /// `out.text`: everything sent, each line ending in `\n`.
    pub fn text(&self) -> &str {
        &self.text
    }
}
