// Projected by hand from patrol, stomps (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Crawler } from "../model/level.ts";
import { stompLine } from "./constants.ts";

/** A crawler walks to the end of its beat and turns. */
export function patrol(c: Crawler): Crawler {
  const x = c.at.x + c.dir * c.speed;
  if (x >= c.hi) return { ...c, at: { ...c.at, x: c.hi }, dir: -1 };
  if (x <= c.lo) return { ...c, at: { ...c.at, x: c.lo }, dir: 1 };
  return { ...c, at: { ...c.at, x } };
}

/** From above and falling, on the upper half: a stomp; any other touch hurts. */
export const stomps = (feet: number, vy: number, c: Crawler): boolean => vy < 0 && feet >= c.at.y + stompLine;
