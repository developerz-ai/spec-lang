// Projected by hand from the run command the game vocabulary derives from tick and report (docs/game-vocabulary.md, src/apps/rust.slg). Stand-in for spec-lang build output; change the spec, not this file.

use super::Io;
use crate::cli::Exit;
use crate::levels::LevelNo;
use crate::model::Mode;
use crate::script::Script;
use crate::world::{Lines, World};

/// `start(level)`, then one `step` per tick while `Playing`, every line, then the summary.
/// Lines are gathered and written once at the end: nothing reads standard output while the script runs.
pub fn run(level: LevelNo, script: &Script, io: &mut Io<'_>) -> Exit {
    let mut out = Lines::default();
    let mut world = World::started(level, &mut out);
    for pad in script.pads() {
        if world.mode != Mode::Playing || world.step(pad, &mut out).is_err() {
            break;
        }
    }
    out.text.push_str(&world.summary());
    out.text.push('\n');
    io.print(out.text.as_bytes())
}
