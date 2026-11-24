// Projected by hand from shot: tick = world.step, frame = screen.present (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import type { LevelNo } from "../levels/level.ts";
import { shoot } from "../screen/offscreen.ts";
import { readScript } from "../script/read.ts";
import { view } from "../world/world.ts";
import { fnv1a64 } from "./hash.ts";
import { printed, type Result, usageError } from "./result.ts";
import { screenFailure } from "./screen-failure.ts";
import { simulate, stopAfter } from "./simulate.ts";

export type Screen = {
  readonly library: string;
  readonly assets: string;
  readonly display: boolean;
  readonly title: string;
};

const silent = (): void => {};

/** The script is read and played before any window opens. */
export function shot(level: LevelNo, at: bigint | null, out: string | null, input: Uint8Array, screen: Screen): Result {
  const read = readScript(input);
  if (!read.ok) return usageError(read.message);
  const world = simulate(level, read.script, stopAfter(at), silent);
  const taken = shoot(screen, view(world), out, fnv1a64);
  return taken.ok ? printed([`shot ${taken.value}`]) : screenFailure(taken.error);
}
