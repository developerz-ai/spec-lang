// Projected by hand from check_collision_boxes (src/hop/screen.slg, docs/contract.md section 5). Stand-in for spec-lang build output; change the spec, not this file.

import type { Box } from "../geom/box.ts";
import type { Raylib } from "./raylib.ts";
import { putBoundingBox } from "./structs.ts";

/** raylib's own box test. It needs no window. */
export function collisionCheck(rl: Raylib): (a: Box, b: Box) => boolean {
  const check = rl.fn("CheckCollisionBoxes");
  const first = new Float32Array(6);
  const second = new Float32Array(6);
  return (a, b) => {
    putBoundingBox(first, a);
    putBoundingBox(second, b);
    return rl.shim.skyhop_CheckCollisionBoxes(check, first, second);
  };
}
