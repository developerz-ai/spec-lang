// Projected by hand from module hop.screen (src/hop/screen.slg). Stand-in for spec-lang build output; change the spec, not this file.
//! The raylib edge. Every call into the C library is here, and nowhere else.
#![allow(unsafe_code)]

mod assets;
mod convert;
pub mod ffi;
mod keys;
mod present;
mod window;

pub use assets::{AssetError, Assets};
pub use convert::{bounding_box, color, vector};
pub use keys::read_pad;
pub use present::present;
pub use window::{Capture, Window, WindowError};

use crate::geom::Box;

/// Raylib's own box test, so the game's rule can be checked against the library's. Pure math in raylib: no window needed.
pub fn check_collision_boxes(a: Box, b: Box) -> bool {
    // SAFETY: two plain structs by value; the function reads only them.
    unsafe { ffi::CheckCollisionBoxes(bounding_box(a), bounding_box(b)) }
}
