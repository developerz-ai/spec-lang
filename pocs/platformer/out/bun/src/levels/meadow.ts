// Projected by hand from meadow (src/hop/levels.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Level } from "../model/level.ts";
import { at, grass, q, slab, stone } from "./grid.ts";

/** Teaches running, a gap, a step up, a crawler and the flag. */
export const meadow: Level = {
  name: "meadow",
  start: at(0, 0, 0),
  solids: [
    slab(at(-8, -4, -8), at(16, 0, 8), grass),
    slab(at(24, -4, -8), at(40, 0, 8), grass),
    slab(at(40, -4, -8), at(56, 4, 8), stone),
    slab(at(64, -4, -8), at(84, 0, 8), grass),
  ],
  movers: [],
  gems: [
    { no: 1, at: at(32, 6, 0) },
    { no: 2, at: at(60, 10, 0) },
    { no: 3, at: at(72, 2, 0) },
  ],
  crawlers: [{ no: 1, at: at(48, 4, 0), lo: q(43), hi: q(53), dir: 1, speed: 2 }],
  goal: at(80, 0, 0),
};
