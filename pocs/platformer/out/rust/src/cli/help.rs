// Projected by hand from app SkyhopRust's start, tick, input, frame, dump and report (src/apps/rust.slg) and the help of contract section 2. Stand-in for spec-lang build output; change the spec, not this file.

/// The help text. The commands and their flags are fixed when the app is built, so the text is a constant.
pub const HELP: &str = "\
usage: skyhop <command> [options]

commands:
  play [--level <n>]
  run [--level <n>]
  draw [--level <n>] [--at <tick>]
  shot [--level <n>] [--at <tick>] [--out <file.png>]

play opens a window: arrows or WASD to run, space to jump, Esc to quit.
run, draw and shot read a script of controls on standard input.
";
