// Projected by hand from LevelNo, level (src/hop/levels.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Level } from "../model/level.ts";
import { meadow } from "./meadow.ts";
import { ridge } from "./ridge.ts";

/** `LevelNo = 1..=2` */
export type LevelNo = 1 | 2;

export const level = (n: LevelNo): Level => (n === 1 ? meadow : ridge);
