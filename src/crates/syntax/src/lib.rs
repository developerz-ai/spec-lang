//! The surface syntax of spec-lang.
//!
//! This crate accepts exactly the language of the LALR(1) grammar in
//! `docs/idea/25-grammar.md`, including the behaviour of the Lark contextual
//! lexer the reference checker uses (a keyword is a keyword only where the
//! grammar can take it; see [`parser`]). It carries no types or meaning.
//!
//! Pipeline: [`lexer::lex`] turns text into [`token::Token`]s, [`parser::parse_file`]
//! turns tokens into an [`ast::File`]. Every node carries a [`span::Span`];
//! every error is a [`diag::Diagnostic`] that renders as `file:line:col`.

pub mod ast;
pub mod diag;
pub mod fence;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod token;
mod unicode;
mod unicode_tables;

pub use diag::Diagnostic;
pub use parser::{parse_expr, parse_file};
pub use span::{FileId, Location, SourceFile, SourceMap, Span};
