// Projected by hand from note @store (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

mod atomic;
pub mod codec;
mod error;
mod file;
pub mod lock;
pub mod path;

pub use error::StoreError;
pub use file::Store;
