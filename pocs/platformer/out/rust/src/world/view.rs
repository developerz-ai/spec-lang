// Projected by hand from type View, fn view (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::collections::{BTreeMap, BTreeSet};

use super::World;
use crate::model::{Crawler, Level, Mode, Player};

/// Everything the screen needs, read once per frame. Borrowed, not copied: a `fn` writes nothing, so the world cannot change
/// while a view of it is read.
#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    pub stage: &'a Level,
    pub ticks: i32,
    pub player: &'a Player,
    pub taken: &'a BTreeSet<i32>,
    /// `crawlers.values`: by crawler number.
    pub crawlers: &'a BTreeMap<i32, Crawler>,
    pub lives: i32,
    pub mode: Mode,
}

impl World {
    pub fn view(&self) -> View<'_> {
        View {
            stage: &self.stage,
            ticks: self.ticks,
            player: &self.player,
            taken: &self.taken,
            crawlers: &self.crawlers,
            lives: self.lives,
            mode: self.mode,
        }
    }
}
