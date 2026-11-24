// Projected by hand from Fall, fall (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { over, overlaps } from "../geom/box.ts";
import type { Vec3 } from "../geom/vec3.ts";
import type { Solid } from "../model/level.ts";
import { body, type Placed } from "./bodies.ts";
import { bodyHalf } from "./constants.ts";

export type Fall = {
  readonly y: number;
  readonly vel: number;
  readonly grounded: boolean;
  readonly riding: number | null;
};

/**
 * Solids stop the body both ways; a mover's top only catches feet that were above it.
 * One pass per list: the lowest underside, the highest top, and the smallest number among the highest catching tops.
 */
export function fall(at: Vec3, v: number, solids: readonly Solid[], tops: readonly Placed[]): Fall {
  const moved = body({ x: at.x, y: at.y + v, z: at.z });
  let hit = false;
  let ceiling = Number.POSITIVE_INFINITY;
  let floor = Number.NEGATIVE_INFINITY;
  for (const s of solids) {
    if (!overlaps(moved, s.box)) continue;
    hit = true;
    ceiling = Math.min(ceiling, s.box.lo.y);
    floor = Math.max(floor, s.box.hi.y);
  }
  if (hit && v > 0) return { y: ceiling - bodyHalf.y, vel: 0, grounded: false, riding: null };
  if (hit) return { y: floor, vel: 0, grounded: true, riding: null };

  if (v < 0 && tops.length > 0) {
    const here = body(at);
    let top = Number.NEGATIVE_INFINITY;
    let rider: number | null = null;
    for (const p of tops) {
      const y = p.box.hi.y;
      if (!(over(here, p.box) && at.y >= y && at.y + v < y)) continue;
      if (rider === null || y > top) {
        top = y;
        rider = p.no;
      } else if (y === top && p.no < rider) {
        rider = p.no;
      }
    }
    if (rider !== null) return { y: top, vel: 0, grounded: true, riding: rider };
  }
  return { y: at.y + v, vel: v, grounded: false, riding: null };
}
