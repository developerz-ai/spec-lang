// Projected by hand from app SkyhopBun: one parsed command to its result (src/apps/bun.slg, docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import { HELP } from "../cli/help.ts";
import { parse } from "../cli/parse.ts";
import { assetsDir, displayNamed, raylibPath } from "../screen/paths.ts";
import { draw } from "./draw.ts";
import { play } from "./play.ts";
import { type Result, usageError } from "./result.ts";
import { run } from "./run.ts";
import { shot } from "./shot.ts";

/** `@window(960, 540, "Skyhop")` and `@on(tick(60Hz))`. */
const WINDOW = { width: 960, height: 540, fps: 60 } as const;
const TITLE = "Skyhop";

export type Io = {
  readonly env: Readonly<Record<string, string | undefined>>;
  /** Standard input, read whole; asked for only by a command that takes a script, after the arguments parse. */
  readonly stdin: () => Uint8Array;
  /** Standard output during `play`, flushed as it is written. */
  readonly write: (text: string) => void;
};

export function execute(argv: readonly string[], io: Io): Result {
  const parsed = parse(argv);
  if (parsed.kind === "help") {
    return parsed.to === "stdout" ? { stdout: HELP, stderr: "", code: 0 } : { stdout: "", stderr: HELP, code: 2 };
  }
  if (parsed.kind === "usage") return usageError(parsed.message);
  const command = parsed.command;
  const screen = () => ({
    library: raylibPath(io.env),
    assets: assetsDir(io.env),
    display: displayNamed(io.env),
    title: TITLE,
  });
  switch (command.name) {
    case "run":
      return run(command.level, io.stdin());
    case "draw":
      return draw(command.level, command.at, io.stdin());
    case "shot":
      return shot(command.level, command.at, command.out, io.stdin(), screen());
    case "play":
      return play(command.level, screen(), WINDOW, io.write);
  }
}
