//! Module `hop.tests.touches`, printed by spec-lang from `src/hop/tests/touches.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.

use crate::rt::{self, Int, R, Range};
use crate::machine::Machine;
use std::collections::BTreeSet;

/// Constant `hop.tests.touches.rest`, `src/hop/tests/touches.slg:8`.
pub fn rest() -> R<crate::hop::model::Pad> {
    crate::hop::model::Pad::new(None, None, None, None, None)
}

/// Test `hop.tests.touches.a_gem_is_taken_once`, `src/hop/tests/touches.slg:10`.
pub fn a_gem_is_taken_once(m: &mut Machine) -> R<()> {
    crate::hop::world::step(m, rest()?)?;
    rt::assert(m.world.taken == BTreeSet::from([3]))?;
    crate::hop::world::step(m, rest()?)?;
    rt::assert(m.world.taken == BTreeSet::from([3]))?;
    rt::assert(crate::machine::out_text(m)? == "1 gem 3\n")?;
    Ok(())
}

/// a life lost keeps the gems taken
///
/// Test `hop.tests.touches.a_fall_keeps_the_gems`, `src/hop/tests/touches.slg:19`.
pub fn a_fall_keeps_the_gems(m: &mut Machine) -> R<()> {
    crate::hop::world::step(m, rest()?)?;
    rt::assert(m.world.lives == 2)?;
    rt::assert(m.world.taken == BTreeSet::from([1]))?;
    Ok(())
}

/// only a falling body stomps; rising into a crawler hurts
///
/// Test `hop.tests.touches.rising_into_a_crawler_hurts`, `src/hop/tests/touches.slg:27`.
pub fn rising_into_a_crawler_hurts(m: &mut Machine) -> R<()> {
    crate::hop::world::step(m, rest()?)?;
    rt::assert(m.world.lives == 2)?;
    rt::assert(m.world.crawlers.contains_key(&1))?;
    Ok(())
}

/// falling onto the upper half of a crawler removes it and bounces
///
/// Test `hop.tests.touches.a_stomp_removes_the_crawler`, `src/hop/tests/touches.slg:35`.
pub fn a_stomp_removes_the_crawler(m: &mut Machine) -> R<()> {
    crate::hop::world::step(m, rest()?)?;
    rt::assert(!m.world.crawlers.contains_key(&1))?;
    rt::assert(m.world.player.vel.y == 24)?;
    rt::assert(m.world.lives == 3)?;
    Ok(())
}

/// walking into a crawler costs a life and restarts the hero
///
/// Test `hop.tests.touches.a_crawler_hurts_from_the_side`, `src/hop/tests/touches.slg:44`.
pub fn a_crawler_hurts_from_the_side(m: &mut Machine) -> R<()> {
    crate::hop::world::step(
        m,
        crate::hop::model::Pad::new(None, Some(true), None, None, None)?,
    )?;
    rt::assert(m.world.lives == 2)?;
    rt::assert(m.world.player.at == crate::hop::levels::meadow()?.start)?;
    rt::assert(m.world.crawlers.contains_key(&1))?;
    Ok(())
}

/// Test `hop.tests.touches.falling_out_costs_a_life`, `src/hop/tests/touches.slg:53`.
pub fn falling_out_costs_a_life(m: &mut Machine) -> R<()> {
    crate::hop::world::step(m, rest()?)?;
    rt::assert(m.world.lives == 2)?;
    rt::assert(m.world.mode == crate::hop::model::Mode::Playing)?;
    Ok(())
}

/// Test `hop.tests.touches.the_last_life_ends_the_game`, `src/hop/tests/touches.slg:60`.
pub fn the_last_life_ends_the_game(m: &mut Machine) -> R<()> {
    crate::hop::world::step(m, rest()?)?;
    rt::assert(m.world.lives == 0)?;
    rt::assert(m.world.mode == crate::hop::model::Mode::Lost)?;
    let tmp1 = rt::settle(rt::catch(|| crate::hop::world::step(m, rest()?)))?;
    rt::expect_fail(tmp1, "Over", vec![])?;
    Ok(())
}

/// Test `hop.tests.touches.reaching_the_flag_wins`, `src/hop/tests/touches.slg:68`.
pub fn reaching_the_flag_wins(m: &mut Machine) -> R<()> {
    crate::hop::world::step(m, rest()?)?;
    rt::assert(m.world.mode == crate::hop::model::Mode::Won)?;
    let tmp1 = rt::settle(rt::catch(|| crate::hop::world::step(m, rest()?)))?;
    rt::expect_fail(tmp1, "Over", vec![])?;
    Ok(())
}

/// Test `hop.tests.touches.starting_again_resets_everything`, `src/hop/tests/touches.slg:75`.
pub fn starting_again_resets_everything(m: &mut Machine) -> R<()> {
    crate::hop::world::start(m, 1)?;
    for _i in (Range {
        lo: 1,
        hi: 5,
        inclusive: true,
    })
        .list()?
    {
        crate::hop::world::step(
            m,
            crate::hop::model::Pad::new(None, Some(true), None, None, None)?,
        )?;
    }
    crate::hop::world::start(m, 2)?;
    rt::assert(m.world.taken == BTreeSet::<Int>::new())?;
    rt::assert(m.world.ticks == 0)?;
    rt::assert(rt::int::count(m.world.crawlers.len()) == 2)?;
    rt::assert(m.world.player.at == crate::hop::levels::ridge()?.start)?;
    Ok(())
}
