//! Module `hop.tests.moves`, printed by spec-lang from `src/hop/tests/moves.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.

use crate::rt::{self, Int, Opt, R, Range};
use crate::machine::Machine;

/// Constant `hop.tests.moves.right`, `src/hop/tests/moves.slg:7`.
pub fn right() -> R<crate::hop::model::Pad> {
    crate::hop::model::Pad::new(None, Some(true), None, None, None)
}

/// Constant `hop.tests.moves.jump`, `src/hop/tests/moves.slg:8`.
pub fn jump() -> R<crate::hop::model::Pad> {
    crate::hop::model::Pad::new(None, None, None, None, Some(true))
}

/// Constant `hop.tests.moves.rest`, `src/hop/tests/moves.slg:9`.
pub fn rest() -> R<crate::hop::model::Pad> {
    crate::hop::model::Pad::new(None, None, None, None, None)
}

/// Test `hop.tests.moves.standing_still_stays_put`, `src/hop/tests/moves.slg:11`.
pub fn standing_still_stays_put(m: &mut Machine) -> R<()> {
    crate::hop::world::start(m, 1)?;
    for _i in (Range {
        lo: 1,
        hi: 30,
        inclusive: true,
    })
        .list()?
    {
        crate::hop::world::step(m, rest()?)?;
    }
    rt::assert(m.world.player.at == crate::hop::geom::Vec3::new(0, 0, 0)?)?;
    rt::assert(m.world.player.grounded)?;
    rt::assert(m.world.ticks == 30)?;
    Ok(())
}

/// the first tick starts in the air, so it gains 1; then 2 a tick up to 16
///
/// Test `hop.tests.moves.running_reaches_full_speed`, `src/hop/tests/moves.slg:19`.
pub fn running_reaches_full_speed(m: &mut Machine) -> R<()> {
    crate::hop::world::start(m, 1)?;
    for _i in (Range {
        lo: 1,
        hi: 10,
        inclusive: true,
    })
        .list()?
    {
        crate::hop::world::step(m, right()?)?;
    }
    rt::assert(m.world.player.vel.x == 16)?;
    rt::assert(m.world.player.at.x == 96)?;
    rt::assert(m.world.player.yaw == 0)?;
    Ok(())
}

/// a held jump rises 38 + 36 + ... + 2 = 380 sub-metres
///
/// Test `hop.tests.moves.a_held_jump_peaks_at_380`, `src/hop/tests/moves.slg:28`.
pub fn a_held_jump_peaks_at_380(m: &mut Machine) -> R<()> {
    crate::hop::world::start(m, 1)?;
    crate::hop::world::step(m, rest()?)?;
    let mut top: Int = 0;
    for _i in (Range {
        lo: 1,
        hi: 50,
        inclusive: true,
    })
        .list()?
    {
        crate::hop::world::step(m, jump()?)?;
        top = top.max(m.world.player.at.y);
    }
    rt::assert(top == 380)?;
    rt::assert(m.world.player.grounded)?;
    Ok(())
}

/// let go after one tick: 38, then cut to 16 and fall away: 14 + 12 + ... + 2 more
///
/// Test `hop.tests.moves.a_short_hop_peaks_at_94`, `src/hop/tests/moves.slg:41`.
pub fn a_short_hop_peaks_at_94(m: &mut Machine) -> R<()> {
    crate::hop::world::start(m, 1)?;
    crate::hop::world::step(m, rest()?)?;
    crate::hop::world::step(m, jump()?)?;
    let mut top: Int = 0;
    for _i in (Range {
        lo: 1,
        hi: 30,
        inclusive: true,
    })
        .list()?
    {
        crate::hop::world::step(m, rest()?)?;
        top = top.max(m.world.player.at.y);
    }
    rt::assert(top == 94)?;
    Ok(())
}

/// holding jump through a landing does not jump again
///
/// Test `hop.tests.moves.a_jump_needs_a_fresh_press`, `src/hop/tests/moves.slg:54`.
pub fn a_jump_needs_a_fresh_press(m: &mut Machine) -> R<()> {
    crate::hop::world::start(m, 1)?;
    crate::hop::world::step(m, rest()?)?;
    for _i in (Range {
        lo: 1,
        hi: 60,
        inclusive: true,
    })
        .list()?
    {
        crate::hop::world::step(m, jump()?)?;
    }
    rt::assert(m.world.player.grounded)?;
    rt::assert(m.world.player.at.y == 0)?;
    Ok(())
}

/// Test `hop.tests.moves.a_jump_just_after_the_edge_still_counts`, `src/hop/tests/moves.slg:63`.
pub fn a_jump_just_after_the_edge_still_counts(m: &mut Machine) -> R<()> {
    crate::hop::world::step(m, rest()?)?;
    rt::assert(!m.world.player.grounded)?;
    crate::hop::world::step(m, jump()?)?;
    rt::assert(m.world.player.vel.y == 38)?;
    Ok(())
}

/// Test `hop.tests.moves.a_jump_long_after_the_edge_does_not`, `src/hop/tests/moves.slg:71`.
pub fn a_jump_long_after_the_edge_does_not(m: &mut Machine) -> R<()> {
    for _i in (Range {
        lo: 1,
        hi: 7,
        inclusive: true,
    })
        .list()?
    {
        crate::hop::world::step(m, rest()?)?;
    }
    crate::hop::world::step(m, jump()?)?;
    rt::assert(m.world.player.vel.y < 0)?;
    Ok(())
}

/// Test `hop.tests.moves.a_wall_stops_the_body_flush`, `src/hop/tests/moves.slg:78`.
pub fn a_wall_stops_the_body_flush(m: &mut Machine) -> R<()> {
    for _i in (Range {
        lo: 1,
        hi: 10,
        inclusive: true,
    })
        .list()?
    {
        crate::hop::world::step(m, right()?)?;
    }
    rt::assert(m.world.player.at.x == 2496)?;
    rt::assert(m.world.player.vel.x == 0)?;
    Ok(())
}

/// Test `hop.tests.moves.a_ceiling_stops_a_jump`, `src/hop/tests/moves.slg:85`.
pub fn a_ceiling_stops_a_jump(m: &mut Machine) -> R<()> {
    let mut top: Int = 0;
    for _i in (Range {
        lo: 1,
        hi: 40,
        inclusive: true,
    })
        .list()?
    {
        crate::hop::world::step(m, jump()?)?;
        top = top.max(m.world.player.at.y);
    }
    rt::assert(top == 208)?;
    Ok(())
}

/// Test `hop.tests.moves.a_rider_moves_with_the_mover`, `src/hop/tests/moves.slg:95`.
pub fn a_rider_moves_with_the_mover(m: &mut Machine) -> R<()> {
    for _i in (Range {
        lo: 1,
        hi: 10,
        inclusive: true,
    })
        .list()?
    {
        crate::hop::world::step(m, rest()?)?;
    }
    rt::assert(m.world.player.at == crate::hop::geom::Vec3::new(1680, 0, 0)?)?;
    rt::assert(m.world.player.riding == Opt::Some(1))?;
    Ok(())
}

/// Test `hop.tests.moves.a_mover_catches_a_falling_body`, `src/hop/tests/moves.slg:102`.
pub fn a_mover_catches_a_falling_body(m: &mut Machine) -> R<()> {
    for _i in (Range {
        lo: 1,
        hi: 40,
        inclusive: true,
    })
        .list()?
    {
        crate::hop::world::step(m, rest()?)?;
    }
    rt::assert(m.world.player.riding == Opt::Some(1))?;
    rt::assert(m.world.player.at.y == 0)?;
    Ok(())
}

/// a mover is one-way: a body below it keeps falling
///
/// Test `hop.tests.moves.a_mover_never_catches_from_below`, `src/hop/tests/moves.slg:109`.
pub fn a_mover_never_catches_from_below(m: &mut Machine) -> R<()> {
    crate::hop::world::step(m, rest()?)?;
    rt::assert(m.world.player.at.y < rt::int::neg(200)?)?;
    rt::assert(m.world.player.riding == Opt::None)?;
    Ok(())
}

/// the fastest fall still stops on a floor
///
/// Test `hop.tests.moves.a_fast_fall_lands`, `src/hop/tests/moves.slg:117`.
pub fn a_fast_fall_lands(m: &mut Machine) -> R<()> {
    for _i in (Range {
        lo: 1,
        hi: 12,
        inclusive: true,
    })
        .list()?
    {
        crate::hop::world::step(m, rest()?)?;
    }
    rt::assert(m.world.player.at.y == 0)?;
    rt::assert(m.world.player.grounded)?;
    Ok(())
}
