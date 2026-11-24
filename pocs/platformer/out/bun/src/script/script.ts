// Projected by hand from the input script derived from Pad, the parameter of step (docs/game-vocabulary.md, docs/contract.md section 2 "The input script"). Stand-in for spec-lang build output; change the spec, not this file.

import type { Pad } from "../model/pad.ts";

/** One line of the script: the same pad for `ticks` ticks. Kept as a run, never expanded tick by tick. */
export type Hold = { readonly ticks: number; readonly pad: Pad };

export type Script = readonly Hold[];

export const MAX_HOLD = 100_000;
export const MAX_TICKS = 1_000_000;
