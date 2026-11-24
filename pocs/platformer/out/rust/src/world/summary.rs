// Projected by hand from mode_text, summary (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

use super::World;
use crate::model::Mode;

pub const fn mode_text(m: Mode) -> &'static str {
    match m {
        Mode::Playing => "playing",
        Mode::Won => "won",
        Mode::Lost => "lost",
    }
}

impl World {
    /// One line that sums up a run, printed after the last tick.
    pub fn summary(&self) -> String {
        format!(
            "end {} {} gems {}/{} lives {} at {}",
            self.ticks,
            mode_text(self.mode),
            self.taken.len(),
            self.stage.gems.len(),
            self.lives,
            self.player.at
        )
    }
}
