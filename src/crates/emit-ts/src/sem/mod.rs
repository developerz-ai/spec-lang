//! The target-neutral half of an emitter: the typed tree, the plan of an app,
//! naming, and provenance. `emit-ruby` uses it too; it lives here until the IR
//! carries expression types (see `IR-NEEDS.md`).

pub mod app;
pub mod infer;
pub mod names;
pub mod prov;
pub mod tree;
