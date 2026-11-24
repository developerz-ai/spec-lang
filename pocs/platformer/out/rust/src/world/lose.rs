// Projected by hand from rule lose (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

use super::{Out, World};
use crate::model::{Mode, Player};

impl World {
    /// A hit or a fall costs a life and sends the hero back to the start; the last one ends the game.
    /// Its guard `mode == Playing` always holds here: `lose` joins `step`, whose own guard checked it, and `step` changes
    /// `mode` only after its last call to `lose`.
    pub(super) fn lose(&mut self, why: &str, out: &mut impl Out) {
        out.line(format_args!("{} {why}", self.ticks));
        self.lives -= 1;
        self.player = Player::new(self.stage.start);
        if self.lives == 0 {
            self.mode = Mode::Lost;
            out.line(format_args!("{} lost", self.ticks));
        }
    }
}
