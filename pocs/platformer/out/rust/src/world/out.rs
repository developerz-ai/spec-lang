// Projected by hand from the effect out.line (docs/game-vocabulary.md), as used by start, lose and step (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
//! Where a rule's `out.line` goes.

use std::fmt::{self, Write};

/// A sink for `out.line`. A rule sends lines only after its guards pass, so a refused rule sends none.
pub trait Out {
    fn line(&mut self, text: fmt::Arguments<'_>);
}

/// Lines kept in order, each followed by `\n`.
#[derive(Debug, Default)]
pub struct Lines {
    pub text: String,
}

impl Out for Lines {
    fn line(&mut self, text: fmt::Arguments<'_>) {
        // Writing to a `String` cannot fail.
        let _ = self.text.write_fmt(text);
        self.text.push('\n');
    }
}

/// `draw` and `shot` print nothing while they run: a line is never formatted.
/// Licensed by `out.line` having no effect but the line itself (docs/game-vocabulary.md).
#[derive(Debug, Default)]
pub struct Silent;

impl Out for Silent {
    fn line(&mut self, _text: fmt::Arguments<'_>) {}
}
