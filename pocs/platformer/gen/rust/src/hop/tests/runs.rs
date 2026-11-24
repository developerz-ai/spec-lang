//! Module `hop.tests.runs`, printed by spec-lang from `src/hop/tests/runs.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.

use crate::rt::{self, Int, R, Range, Show};
use crate::machine::Machine;

/// Record `hop.tests.runs.Hold`, `src/hop/tests/runs.slg:8`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hold {
    pub ticks: Int,
    pub keys: String,
}

impl Show for Hold {
    fn show(&self) -> String {
        rt::show::record(
            "Hold",
            &[("ticks", self.ticks.show()), ("keys", self.keys.show())],
        )
    }
}

impl Hold {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(ticks: Int, keys: String) -> R<Self> {
        rt::claim(crate::conform::range_1_10000(&ticks)?, "field `ticks`")?;
        Ok(Hold { ticks, keys })
    }
}

/// run right, with four held jumps: the gap, the first gem, the step past the crawler, the gap with the second gem
///
/// Constant `hop.tests.runs.meadow_win`, `src/hop/tests/runs.slg:20`.
pub fn meadow_win() -> R<Vec<Hold>> {
    Ok(
        vec![
            Hold::new(75, String::from("R"))?,
            Hold::new(20, String::from("RJ"))?,
            Hold::new(41, String::from("R"))?,
            Hold::new(20, String::from("RJ"))?,
            Hold::new(35, String::from("R"))?,
            Hold::new(20, String::from("RJ"))?,
            Hold::new(23, String::from("R"))?,
            Hold::new(20, String::from("RJ"))?,
            Hold::new(62, String::from("R"))?,
        ],
    )
}

/// found by a search over pad scripts: wait for the mover, ride it, hop the gem, then the high ground
///
/// Constant `hop.tests.runs.ridge_win`, `src/hop/tests/runs.slg:27`.
pub fn ridge_win() -> R<Vec<Hold>> {
    Ok(
        vec![
            Hold::new(24, String::from("R"))?,
            Hold::new(8, String::from("-"))?,
            Hold::new(8, String::from("R"))?,
            Hold::new(16, String::from("RJ"))?,
            Hold::new(16, String::from("R"))?,
            Hold::new(8, String::from("-"))?,
            Hold::new(8, String::from("L"))?,
            Hold::new(8, String::from("R"))?,
            Hold::new(8, String::from("L"))?,
            Hold::new(8, String::from("R"))?,
            Hold::new(8, String::from("L"))?,
            Hold::new(8, String::from("R"))?,
            Hold::new(8, String::from("L"))?,
            Hold::new(8, String::from("R"))?,
            Hold::new(8, String::from("L"))?,
            Hold::new(8, String::from("R"))?,
            Hold::new(16, String::from("RJ"))?,
            Hold::new(24, String::from("R"))?,
            Hold::new(16, String::from("RJ"))?,
            Hold::new(8, String::from("-"))?,
            Hold::new(8, String::from("R"))?,
            Hold::new(16, String::from("-"))?,
            Hold::new(16, String::from("J"))?,
            Hold::new(8, String::from("-"))?,
            Hold::new(16, String::from("R"))?,
            Hold::new(8, String::from("J"))?,
            Hold::new(8, String::from("L"))?,
            Hold::new(24, String::from("R"))?,
            Hold::new(16, String::from("RJ"))?,
            Hold::new(32, String::from("R"))?,
            Hold::new(8, String::from("RJ"))?,
            Hold::new(8, String::from("J"))?,
            Hold::new(8, String::from("R"))?,
            Hold::new(8, String::from("-"))?,
            Hold::new(16, String::from("RJ"))?,
            Hold::new(32, String::from("R"))?,
            Hold::new(16, String::from("RJ"))?,
            Hold::new(16, String::from("R"))?,
            Hold::new(8, String::from("-"))?,
            Hold::new(32, String::from("R"))?,
            Hold::new(16, String::from("RJ"))?,
            Hold::new(8, String::from("R"))?,
        ],
    )
}

/// one letter per Pad field, as the game vocabulary reads a script: L R F B J, or - for none
///
/// Fn `hop.tests.runs.pad_of`, `src/hop/tests/runs.slg:10`.
pub fn pad_of(keys: String) -> R<crate::hop::model::Pad> {
    crate::hop::model::Pad::new(
        Some(keys.contains(String::from("L").as_str())),
        Some(keys.contains(String::from("R").as_str())),
        Some(keys.contains(String::from("F").as_str())),
        Some(keys.contains(String::from("B").as_str())),
        Some(keys.contains(String::from("J").as_str())),
    )
}

/// the meadow is winnable, with every gem, without losing a life
///
/// Test `hop.tests.runs.the_meadow_can_be_won`, `src/hop/tests/runs.slg:42`.
pub fn the_meadow_can_be_won(m: &mut Machine) -> R<()> {
    crate::hop::world::start(m, 1)?;
    for h in meadow_win()? {
        for _i in (Range {
            lo: 1,
            hi: h.ticks,
            inclusive: true,
        })
            .list()?
        {
            crate::hop::world::step(m, pad_of(h.keys.clone())?)?;
        }
    }
    rt::assert(m.world.mode == crate::hop::model::Mode::Won)?;
    rt::assert(m.world.ticks == 316)?;
    rt::assert(rt::int::count(m.world.taken.len()) == 3)?;
    rt::assert(m.world.lives == 3)?;
    Ok(())
}

/// the ridge is winnable too
///
/// Test `hop.tests.runs.the_ridge_can_be_won`, `src/hop/tests/runs.slg:54`.
pub fn the_ridge_can_be_won(m: &mut Machine) -> R<()> {
    crate::hop::world::start(m, 2)?;
    for h in ridge_win()? {
        for _i in (Range {
            lo: 1,
            hi: h.ticks,
            inclusive: true,
        })
            .list()?
        {
            if m.world.mode == crate::hop::model::Mode::Playing {
                crate::hop::world::step(m, pad_of(h.keys.clone())?)?;
            }
        }
    }
    rt::assert(m.world.mode == crate::hop::model::Mode::Won)?;
    rt::assert(m.world.ticks == 549)?;
    rt::assert(rt::int::count(m.world.taken.len()) == 3)?;
    rt::assert(m.world.lives == 3)?;
    Ok(())
}
