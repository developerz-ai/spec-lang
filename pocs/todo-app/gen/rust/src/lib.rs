//! A spec-lang program printed as Rust by spec-lang's Rust printer: its modules, its world and machine, its checks.
#![allow(
    unused_variables,
    unused_mut,
    unreachable_code,
    irrefutable_let_patterns,
    unused_parens,
    unused_braces,
    clippy::nursery
)]
pub mod checks;
pub mod conform;
pub mod draw;
pub mod machine;
pub mod rt;
pub mod libs;
pub mod manifest;
pub mod apps;
pub mod todo;
