// Projected by hand from ridge (src/hop/levels.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Level } from "../model/level.ts";
import { at, grass, q, slab, stone, wood } from "./grid.ts";

/** A moving platform over a wide gap, then two crawlers on high ground a jump can reach. */
export const ridge: Level = {
  name: "ridge",
  start: at(0, 0, 0),
  solids: [
    slab(at(-8, -4, -8), at(16, 0, 8), grass),
    slab(at(52, -4, -8), at(60, 0, 8), grass),
    slab(at(60, -4, -8), at(76, 5, 8), stone),
    slab(at(80, -4, -8), at(100, 5, 8), grass),
  ],
  movers: [
    { no: 1, home: { lo: at(20, -2, -6), hi: at(32, 0, 6) }, step: { x: 8, y: 0, z: 0 }, span: 128, color: wood },
  ],
  gems: [
    { no: 1, at: at(34, 5, 0) },
    { no: 2, at: at(68, 9, 0) },
    { no: 3, at: at(90, 9, 0) },
  ],
  crawlers: [
    { no: 1, at: at(68, 5, 0), lo: q(62), hi: q(74), dir: 1, speed: 3 },
    { no: 2, at: at(88, 5, 0), lo: q(82), hi: q(94), dir: -1, speed: 2 },
  ],
  goal: at(96, 5, 0),
};
