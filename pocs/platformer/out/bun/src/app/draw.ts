// Projected by hand from draw: tick = world.step, dump = render.dump (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import type { LevelNo } from "../levels/level.ts";
import { dump } from "../render/text.ts";
import { readScript } from "../script/read.ts";
import { printed, type Result, usageError } from "./result.ts";
import { simulate, stopAfter } from "./simulate.ts";

const silent = (): void => {};

/** Never opens a window or loads an asset: it does not import the screen. */
export function draw(level: LevelNo, at: bigint | null, input: Uint8Array): Result {
  const read = readScript(input);
  if (!read.ok) return usageError(read.message);
  return printed(dump(simulate(level, read.script, stopAfter(at), silent)));
}
