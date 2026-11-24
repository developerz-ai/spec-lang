// Projected by hand from run: tick = world.step, report = world.summary (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import type { LevelNo } from "../levels/level.ts";
import { readScript } from "../script/read.ts";
import { MAX_TICKS } from "../script/script.ts";
import { summary } from "../world/summary.ts";
import { printed, type Result, usageError } from "./result.ts";
import { simulate } from "./simulate.ts";

export function run(level: LevelNo, input: Uint8Array): Result {
  const read = readScript(input);
  if (!read.ok) return usageError(read.message);
  const lines: string[] = [];
  const world = simulate(level, read.script, MAX_TICKS, (l) => lines.push(l));
  lines.push(summary(world));
  return printed(lines);
}
