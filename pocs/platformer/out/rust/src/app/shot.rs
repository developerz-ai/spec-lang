// Projected by hand from the shot command the game vocabulary derives from tick and frame (docs/game-vocabulary.md, src/apps/rust.slg). Stand-in for spec-lang build output; change the spec, not this file.

use super::{Io, fnv1a64, simulate};
use crate::cli::{Exit, Options};
use crate::screen::Window;
use crate::script::Script;

/// As `draw`, then the frame drawn off screen by raylib, its RGBA bytes hashed, and with `--out` written as PNG.
pub fn shot(options: &Options, script: &Script, io: &mut Io<'_>) -> Exit {
    let world = simulate(options.level, options.at, script);
    let window = match Window::hidden(io.assets) {
        Ok(window) => window,
        Err(e) => {
            io.error(e);
            return Exit::Window;
        }
    };
    let capture = match window.capture(&world) {
        Ok(capture) => capture,
        Err(e) => {
            io.error(e);
            return Exit::Window;
        }
    };
    let hash = fnv1a64(capture.bytes());
    if let Some(path) = &options.out
        && !capture.export(path)
    {
        io.error(format_args!("{}: cannot write", path.display()));
        return Exit::Window;
    }
    drop(capture);
    drop(window);
    io.print(format!("shot {hash:016x}\n").as_bytes())
}
