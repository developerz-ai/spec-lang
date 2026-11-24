// Projected by hand from Hold, pad_of, meadow_win, ridge_win (src/hop/tests/runs.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Pad } from "../../src/model/pad.ts";

export type Hold = readonly [ticks: number, keys: string];

/** One letter per Pad field: L R F B J, or - for none. */
export const padOf = (keys: string): Pad => ({
  left: keys.includes("L"),
  right: keys.includes("R"),
  forward: keys.includes("F"),
  back: keys.includes("B"),
  jump: keys.includes("J"),
});

export const meadowWin: readonly Hold[] = [
  [75, "R"],
  [20, "RJ"],
  [41, "R"],
  [20, "RJ"],
  [35, "R"],
  [20, "RJ"],
  [23, "R"],
  [20, "RJ"],
  [62, "R"],
];

export const ridgeWin: readonly Hold[] = [
  [24, "R"],
  [8, "-"],
  [8, "R"],
  [16, "RJ"],
  [16, "R"],
  [8, "-"],
  [8, "L"],
  [8, "R"],
  [8, "L"],
  [8, "R"],
  [8, "L"],
  [8, "R"],
  [8, "L"],
  [8, "R"],
  [8, "L"],
  [8, "R"],
  [16, "RJ"],
  [24, "R"],
  [16, "RJ"],
  [8, "-"],
  [8, "R"],
  [16, "-"],
  [16, "J"],
  [8, "-"],
  [16, "R"],
  [8, "J"],
  [8, "L"],
  [24, "R"],
  [16, "RJ"],
  [32, "R"],
  [8, "RJ"],
  [8, "J"],
  [8, "R"],
  [8, "-"],
  [16, "RJ"],
  [32, "R"],
  [16, "RJ"],
  [16, "R"],
  [8, "-"],
  [32, "R"],
  [16, "RJ"],
  [8, "R"],
];
