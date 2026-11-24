// Projected by hand from the states stage, ticks, player, taken, crawlers, lives, mode (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
//! The one running level, and the two rules that change it: `start` and `step`.

mod lose;
mod out;
mod start;
mod step;
mod summary;
mod view;

use std::collections::{BTreeMap, BTreeSet};

use crate::levels::MEADOW;
use crate::model::{Crawler, Level, Mode, Player};

pub use out::{Lines, Out, Silent};
pub use step::Over;
pub use summary::mode_text;
pub use view::View;

/// The seven states of `hop.world`. `Map` and `Set` are ordered trees: they iterate by ascending key, as the contract requires.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    pub stage: Level,
    pub ticks: i32,
    pub player: Player,
    pub taken: BTreeSet<i32>,
    pub crawlers: BTreeMap<i32, Crawler>,
    /// `lives: 0..=3`.
    pub lives: i32,
    pub mode: Mode,
}

impl Default for World {
    /// The initial values the states declare. The app never reads them (it calls `start` first); a test's `@given` starts from them.
    fn default() -> Self {
        Self {
            stage: MEADOW,
            ticks: 0,
            player: Player::new(MEADOW.start),
            taken: BTreeSet::new(),
            crawlers: BTreeMap::new(),
            lives: 3,
            mode: Mode::Playing,
        }
    }
}
