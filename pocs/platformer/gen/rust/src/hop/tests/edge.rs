//! Module `hop.tests.edge`, printed by spec-lang from `src/hop/tests/edge.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.

use crate::rt::{self, R};
use crate::machine::Machine;

/// the game's touch and raylib's are one test; only a backend linked to raylib can run the right-hand side
///
/// Test `hop.tests.edge.touch_matches_raylib`, `src/hop/tests/edge.slg:7`.
pub fn touch_matches_raylib(
    _m: &mut Machine,
    a: crate::hop::geom::Box,
    b: crate::hop::geom::Box,
) -> R<()> {
    rt::assert(
        crate::hop::geom::touches(a.clone(), b.clone())?
            == crate::hop::screen::check_collision_boxes(a.clone(), b.clone())?,
    )?;
    Ok(())
}
