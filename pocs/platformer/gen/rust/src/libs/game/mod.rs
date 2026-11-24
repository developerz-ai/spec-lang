//! The `game` library's runtime for Rust: a fixed-tick, single-player game with a window. Reads the app's manifest
//! (`crate::rt::app::App`); draws on `crate::libs::screen`, whichever of the app's libraries provides it. Exit codes:
//! 0 success, 1 the program stopped or standard output failed, 2 usage or input, 3 the window or an asset.

mod args;
mod script;
mod session;
mod shape;

use crate::libs::screen;
use crate::rt::Halt;
use crate::rt::app::App;
use args::{Command, Invocation, Parsed};
use session::Session;
use shape::Shape;
use std::io::{Read, Write};
use std::process::ExitCode;

/// The frame `shot` draws off screen, in pixels.
const SHOT: (i32, i32) = (320, 180);

/// How a command ended early: the line for standard error (after `error: `) and the exit code.
#[derive(Debug)]
struct Refusal {
    message: String,
    code: u8,
}

impl From<Halt> for Refusal {
    fn from(h: Halt) -> Self {
        Refusal { message: h.message(), code: 1 }
    }
}

fn window(message: String) -> Refusal {
    Refusal { message, code: 3 }
}

fn print(text: &str) -> Result<(), Refusal> {
    let mut out = std::io::stdout().lock();
    out.write_all(text.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|e| Refusal { message: format!("standard output: {e}"), code: 1 })
}

fn complain(bytes: &[u8]) {
    // If standard error itself fails there is nobody left to tell; the exit code still says it.
    let _ = std::io::stderr().write_all(bytes);
}

pub fn main(app: &'static App) -> ExitCode {
    use std::os::unix::ffi::OsStringExt;
    let argv: Vec<Vec<u8>> = std::env::args_os().skip(1).map(OsStringExt::into_vec).collect();
    let ended = Shape::of(app).map_err(|message| Refusal { message, code: 1 }).and_then(|shape| {
        match args::parse(&shape, &argv) {
            Ok(Parsed::NoArguments) => {
                complain(args::help(&shape).as_bytes());
                Ok(2)
            }
            Ok(Parsed::Help) => print(&args::help(&shape)).map(|()| 0),
            Ok(Parsed::Run(invocation)) => run(&shape, invocation).map(|()| 0),
            Err(line) => {
                complain(&line);
                Ok(2)
            }
        }
    });
    match ended {
        Ok(code) => ExitCode::from(code),
        Err(r) => {
            complain(format!("error: {}\n", r.message).as_bytes());
            ExitCode::from(r.code)
        }
    }
}

/// Standard input, read whole and checked before the first tick.
fn holds(shape: &Shape) -> Result<Vec<script::Hold>, Refusal> {
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).map_err(|_| Refusal { message: "input: cannot read".into(), code: 2 })?;
    script::read(&shape.controls, &input).map_err(|message| Refusal { message, code: 2 })
}

fn run(shape: &Shape, invocation: Invocation) -> Result<(), Refusal> {
    if invocation.command == Command::Play {
        return play(shape, invocation);
    }
    let holds = holds(shape)?;
    let mut game = Session::start(shape, invocation.level)?;
    match invocation.command {
        // `start`, every tick of the script while the game goes on, every line, then the report.
        Command::Run => {
            game.play(&holds, None)?;
            print(&format!("{}{}", game.lines()?, game.report()?))
        }
        // The same, silent, up to tick `--at`; then the frame as text, or drawn off screen and hashed.
        Command::Draw => {
            game.play(&holds, invocation.at)?;
            print(&game.dump()?)
        }
        _ => {
            game.play(&holds, invocation.at)?;
            let hash = shot(shape, &mut game, invocation.out.as_deref())?;
            print(&format!("shot {hash:016x}\n"))
        }
    }
}

fn shot(shape: &Shape, game: &mut Session<'_>, out: Option<&std::path::Path>) -> Result<u64, Refusal> {
    let spec =
        screen::Spec { bin: shape.bin, title: shape.title, width: SHOT.0, height: SHOT.1, hidden: true, fps: None };
    let w = screen::open(shape.app, &spec).map_err(window)?;
    let (drawn, picture) = w.capture(SHOT.0, SHOT.1, || game.frame()).map_err(window)?;
    drawn?;
    if let Some(path) = out
        && !picture.export(path)
    {
        return Err(window(format!("{}: cannot write", path.display())));
    }
    Ok(fnv1a64(picture.bytes()))
}

/// A window, one tick per frame at the tick rate, until it is closed; then the report.
fn play(shape: &Shape, invocation: Invocation) -> Result<(), Refusal> {
    let spec = screen::Spec {
        bin: shape.bin,
        title: shape.title,
        width: shape.width,
        height: shape.height,
        hidden: false,
        fps: Some(shape.rate),
    };
    let w = screen::open(shape.app, &spec).map_err(window)?;
    let mut game = Session::start(shape, invocation.level)?;
    while !w.closing() {
        if !game.over() {
            let pad = game.input()?;
            game.tick(&pad)?;
        }
        print(&game.lines()?)?;
        w.frame(|| game.frame())?;
    }
    drop(w);
    print(&game.report()?)
}

/// FNV-1a, 64 bits.
fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3))
}

#[cfg(test)]
mod tests {
    #[test]
    fn fnv1a64_meets_its_published_values() {
        assert_eq!(super::fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(super::fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(super::fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }
}
