// Projected by hand from the draw command the game vocabulary derives from tick and dump (docs/game-vocabulary.md, src/apps/rust.slg). Stand-in for spec-lang build output; change the spec, not this file.

use super::Io;
use crate::cli::Exit;
use crate::levels::LevelNo;
use crate::model::Mode;
use crate::render;
use crate::script::Script;
use crate::world::{Silent, World};

/// `start(level)` then steps, silent, stopping after tick `at` (tick 0 is right after `start`), at the end of the script,
/// or when the game ends: whichever comes first.
pub fn simulate(level: LevelNo, at: Option<u64>, script: &Script) -> World {
    let mut world = World::started(level, &mut Silent);
    let stop = at.unwrap_or(u64::MAX);
    for pad in script.pads() {
        let done = u64::try_from(world.ticks).unwrap_or(0) >= stop;
        if done || world.mode != Mode::Playing || world.step(pad, &mut Silent).is_err() {
            break;
        }
    }
    world
}

/// The frame of the stopping tick, as text.
pub fn draw(level: LevelNo, at: Option<u64>, script: &Script, io: &mut Io<'_>) -> Exit {
    let world = simulate(level, at, script);
    let mut text = String::new();
    for line in render::dump(&world) {
        text.push_str(&line);
        text.push('\n');
    }
    io.print(text.as_bytes())
}
