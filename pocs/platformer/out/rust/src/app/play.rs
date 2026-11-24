// Projected by hand from the play command the game vocabulary derives from tick, input and frame, and @on(tick(60Hz)) on step (docs/game-vocabulary.md, src/apps/rust.slg, src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

use super::Io;
use crate::cli::Exit;
use crate::levels::LevelNo;
use crate::model::Mode;
use crate::screen::{self, Window};
use crate::world::{Lines, World};

/// A window, then one tick per frame at a fixed 60 frames per second until Esc or the close button.
pub fn play(level: LevelNo, io: &mut Io<'_>) -> Exit {
    let window = match Window::visible(io.assets) {
        Ok(window) => window,
        Err(e) => {
            io.error(e);
            return Exit::Window;
        }
    };
    let mut out = Lines::default();
    let mut world = World::started(level, &mut out);
    while !window.should_close() {
        if world.mode == Mode::Playing {
            // The guard holds: `mode` was just read as `Playing`.
            let _ = world.step(screen::read_pad(), &mut out);
        }
        if !out.text.is_empty() {
            if io.print(out.text.as_bytes()) != Exit::Success {
                return Exit::Output;
            }
            out.text.clear();
        }
        window.frame(&world);
    }
    drop(window);
    io.print(format!("{}\n", world.summary()).as_bytes())
}
