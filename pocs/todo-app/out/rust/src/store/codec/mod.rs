// Projected by hand from note @store: the bytes written and the bytes accepted (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

mod cursor;
mod decode;
mod encode;
mod unreadable;

pub use decode::{TaskView, decode, scan};
pub use encode::encode;
pub use unreadable::Unreadable;
