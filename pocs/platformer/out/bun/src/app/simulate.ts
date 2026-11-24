// Projected by hand from the script loop shared by run, draw and shot: start, then tick = world.step until the script, the game or --at ends (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import type { LevelNo } from "../levels/level.ts";
import { MAX_TICKS, type Script } from "../script/script.ts";
import { start } from "../world/start.ts";
import { step } from "../world/step.ts";
import type { World } from "../world/world.ts";

/**
 * `--at` as a tick to stop after. A script has at most 1,000,000 ticks, so any larger `--at`, of any length,
 * stops where 1,000,000 does: the comparison is made on the BigInt and only a small value becomes a number.
 */
export const stopAfter = (at: bigint | null): number =>
  at === null || at > BigInt(MAX_TICKS) ? MAX_TICKS : Number(at);

/** Plays the script from `start(level)`; every `out.line` goes to `line`. Stops when the game is over or at tick `until`. */
export function simulate(level: LevelNo, script: Script, until: number, line: (text: string) => void): World {
  const began = start(level);
  for (const l of began.lines) line(l);
  let world = began.world;
  for (const hold of script) {
    for (let i = 0; i < hold.ticks; i++) {
      if (world.ticks >= until) return world;
      const o = step(world, hold.pad);
      // A refused tick is the end of the run.
      if (!o.ok) return world;
      for (const l of o.lines) line(l);
      world = o.world;
    }
  }
  return world;
}
