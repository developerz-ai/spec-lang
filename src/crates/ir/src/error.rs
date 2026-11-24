//! Errors from loading and resolving a program: the shared [`Diag`] shape.

pub use spec_lang_diag::Diag;

/// The IR reports with the workspace's one diagnostic type.
pub type IrError = Diag;
