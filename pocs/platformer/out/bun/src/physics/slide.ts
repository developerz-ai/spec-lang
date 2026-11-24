// Projected by hand from Axis, slide_x, slide_z (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { overlaps } from "../geom/box.ts";
import type { Vec3 } from "../geom/vec3.ts";
import type { Solid } from "../model/level.ts";
import { body } from "./bodies.ts";
import { bodyHalf } from "./constants.ts";

export type Axis = { readonly pos: number; readonly vel: number };

/**
 * Move on one axis, then stop flush against the first solid in the way.
 * The spec's filter, map and min/max over the hits are one pass here: min and max do not depend on order.
 */
export function slideX(at: Vec3, v: number, solids: readonly Solid[]): Axis {
  if (v === 0) return { pos: at.x, vel: 0 };
  const moved = body({ x: at.x + v, y: at.y, z: at.z });
  let hit = false;
  let nearest = v > 0 ? Number.POSITIVE_INFINITY : Number.NEGATIVE_INFINITY;
  for (const s of solids) {
    if (!overlaps(moved, s.box)) continue;
    hit = true;
    nearest = v > 0 ? Math.min(nearest, s.box.lo.x) : Math.max(nearest, s.box.hi.x);
  }
  if (!hit) return { pos: at.x + v, vel: v };
  return { pos: v > 0 ? nearest - bodyHalf.x : nearest + bodyHalf.x, vel: 0 };
}

export function slideZ(at: Vec3, v: number, solids: readonly Solid[]): Axis {
  if (v === 0) return { pos: at.z, vel: 0 };
  const moved = body({ x: at.x, y: at.y, z: at.z + v });
  let hit = false;
  let nearest = v > 0 ? Number.POSITIVE_INFINITY : Number.NEGATIVE_INFINITY;
  for (const s of solids) {
    if (!overlaps(moved, s.box)) continue;
    hit = true;
    nearest = v > 0 ? Math.min(nearest, s.box.lo.z) : Math.max(nearest, s.box.hi.z);
  }
  if (!hit) return { pos: at.z + v, vel: v };
  return { pos: v > 0 ? nearest - bodyHalf.z : nearest + bodyHalf.z, vel: 0 };
}
