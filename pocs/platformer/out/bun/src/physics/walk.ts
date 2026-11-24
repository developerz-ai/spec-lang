// Projected by hand from axis, walk_target, facing (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Vec3 } from "../geom/vec3.ts";
import type { Pad } from "../model/pad.ts";
import { diagonalSpeed, runSpeed } from "./constants.ts";

const axis = (neg: boolean, pos: boolean): number => (pos ? 1 : 0) - (neg ? 1 : 0);

/** x runs right, z runs toward the camera, so forward is minus z. */
export function walkTarget(pad: Pad): Vec3 {
  const dx = axis(pad.left, pad.right);
  const dz = axis(pad.forward, pad.back);
  const speed = dx !== 0 && dz !== 0 ? diagonalSpeed : runSpeed;
  return { x: dx * speed, y: 0, z: dz * speed };
}

/** The hero faces where the pad points, and keeps facing there when it lets go. */
export function facing(pad: Pad, was: number): number {
  const dx = axis(pad.left, pad.right);
  const dz = axis(pad.forward, pad.back);
  if (dx === 1) return dz === -1 ? 45 : dz === 1 ? 315 : 0;
  if (dx === -1) return dz === -1 ? 135 : dz === 1 ? 225 : 180;
  return dz === -1 ? 90 : dz === 1 ? 270 : was;
}
