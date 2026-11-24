// Projected by hand from hop.tests.edge (src/hop/tests/edge.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import fc from "fast-check";
import { type Box, box, touches } from "../../src/geom/box.ts";
import { type Vec3, vec3 } from "../../src/geom/vec3.ts";
import { collisionCheck } from "../../src/screen/collision.ts";
import { openRaylib } from "../../src/screen/raylib.ts";
import { library } from "../support/screen.ts";

const SUB = 1048576;
const sub = fc.integer({ min: -SUB, max: SUB });
const point: fc.Arbitrary<Vec3> = fc.tuple(sub, sub, sub).map(([x, y, z]) => vec3(x, y, z));

/** Any two boxes with coordinates drawn from Sub. */
const anyPair: fc.Arbitrary<[Box, Box]> = fc
  .tuple(point, point, point, point)
  .map(([a, b, c, d]) => [box(a, b), box(c, d)]);

const clamp = (n: number): number => Math.max(-SUB, Math.min(SUB, n));
const near = fc.integer({ min: -1, max: 1 });

/**
 * Built to touch or nearly touch: on each axis the second box starts within one sub-metre of a face of the first,
 * or shares its span, so every `<=` of the test is decided by a difference of -1, 0 or 1.
 */
const nearPair: fc.Arbitrary<[Box, Box]> = fc
  .tuple(
    point,
    point,
    fc.array(fc.tuple(fc.constantFrom("above", "below", "same"), near, sub), { minLength: 3, maxLength: 3 }),
  )
  .map(([p, r, axes]) => {
    const lo = vec3(Math.min(p.x, r.x), Math.min(p.y, r.y), Math.min(p.z, r.z));
    const hi = vec3(Math.max(p.x, r.x), Math.max(p.y, r.y), Math.max(p.z, r.z));
    const span = (aLo: number, aHi: number, i: number): [number, number] => {
      const [side, gap, far] = axes[i] as ["above" | "below" | "same", number, number];
      if (side === "above") return [clamp(aHi + gap), Math.max(clamp(aHi + gap), far)];
      if (side === "below") return [Math.min(clamp(aLo + gap), far), clamp(aLo + gap)];
      return [aLo, aHi];
    };
    const [x, y, z] = [span(lo.x, hi.x, 0), span(lo.y, hi.y, 1), span(lo.z, hi.z, 2)];
    return [box(lo, hi), box(vec3(x[0], y[0], z[0]), vec3(x[1], y[1], z[1]))];
  });

test("touch_matches_raylib", () => {
  const opened = openRaylib(library);
  if (!opened.ok) throw new Error(`raylib ${opened.path}: cannot load`);
  const checkCollisionBoxes = collisionCheck(opened.raylib);
  let touching = 0;
  const agree = ([a, b]: [Box, Box]): void => {
    const ours = touches(a, b);
    if (ours) touching++;
    expect(checkCollisionBoxes(a, b)).toBe(ours);
  };
  fc.assert(fc.property(anyPair, agree), { seed: 20261124, numRuns: 1000 });
  fc.assert(fc.property(nearPair, agree), { seed: 20261124, numRuns: 1000 });
  console.log(`touch_matches_raylib: 2000 pairs, ${touching} touching`);
  // Both answers must be exercised.
  expect(touching).toBeGreaterThan(200);
  expect(touching).toBeLessThan(1800);
});
