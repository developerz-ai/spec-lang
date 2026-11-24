//! The runtimes of the libraries the app uses, copied from `libs/<name>/real/rust/`.

pub mod game;
pub mod raylib;

/// The `screen` the app's runtimes require: `raylib` provides it.
pub use raylib as screen;
