// Projected by hand from the help of app SkyhopBun with @bin("skyhop") (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

export const HELP = `usage: skyhop <command> [options]

commands:
  play [--level <n>]
  run [--level <n>]
  draw [--level <n>] [--at <tick>]
  shot [--level <n>] [--at <tick>] [--out <file.png>]

play opens a window: arrows or WASD to run, space to jump, Esc to quit.
run, draw and shot read a script of controls on standard input.
`;
