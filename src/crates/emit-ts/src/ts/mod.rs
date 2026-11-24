//! The TypeScript side: swc tree builders, the printer, the lowering from the
//! typed tree, and one module per kind of generated file.

pub mod build;
pub mod file;
pub mod lower;
pub mod print;
pub mod product;
pub mod project;
pub mod tests;
