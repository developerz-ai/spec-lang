//! Vocabularies the toolchain knows. A `use` that names no module of the
//! project must name one of these; it adds notes and effects, never grammar.

/// The `cli` vocabulary (`pocs/todo-app/docs/cli-vocabulary.md`): the trigger
/// `@on(cli("name"))`, the app notes `@bin` and `@store`, the effect
/// `out.line(text)` and the test handler `out.text`.
pub const CLI: &str = "cli";

/// Every known vocabulary.
pub const KNOWN: &[&str] = &[CLI];
