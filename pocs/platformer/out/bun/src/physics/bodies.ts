// Projected by hand from Placed, body, crawler_box, mover_box, mover_shift (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { aroundFeet, type Box, shifted } from "../geom/box.ts";
import { triangle } from "../geom/scalar.ts";
import { scaled, type Vec3 } from "../geom/vec3.ts";
import type { Crawler, Mover } from "../model/level.ts";
import { bodyHalf, crawlerHalf } from "./constants.ts";

export type Placed = { readonly no: number; readonly box: Box };

export const body = (at: Vec3): Box => aroundFeet(at, bodyHalf);

export const crawlerBox = (c: Crawler): Box => aroundFeet(c.at, crawlerHalf);

export const moverBox = (m: Mover, t: number): Box => shifted(m.home, scaled(m.step, triangle(t, m.span)));

/**
 * A rider moves exactly as far as its mover did this tick.
 * The spec subtracts the `lo` corners of two shifted boxes; `home.lo` cancels, leaving the step times the
 * change of the triangle wave, so no box is built.
 */
export const moverShift = (m: Mover, t: number): Vec3 => scaled(m.step, triangle(t, m.span) - triangle(t - 1, m.span));
