// Projected by hand from rule start (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::collections::{BTreeMap, BTreeSet};

use super::{Out, World};
use crate::levels::{self, LevelNo};
use crate::model::{Mode, Player};

impl World {
    /// Every level starts the same way: three lives, nothing taken, every crawler home.
    pub fn start(&mut self, level: LevelNo, out: &mut impl Out) {
        let l = levels::level(level);
        self.stage = l;
        self.ticks = 0;
        self.player = Player::new(l.start);
        self.taken = BTreeSet::new();
        self.crawlers = l
            .crawlers
            .iter()
            .map(|c| (c.no, *c))
            .collect::<BTreeMap<_, _>>();
        self.lives = 3;
        self.mode = Mode::Playing;
        out.line(format_args!("0 level {}", l.name));
    }

    /// A world right after `start(level)`.
    pub fn started(level: LevelNo, out: &mut impl Out) -> Self {
        let mut world = Self::default();
        world.start(level, out);
        world
    }
}
