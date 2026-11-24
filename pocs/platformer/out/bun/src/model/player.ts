// Projected by hand from Mode, Player (src/hop/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Vec3 } from "../geom/vec3.ts";

export type Mode = "Playing" | "Won" | "Lost";

/** `riding` names the mover the feet are on (`null` is `none`). `coyote` is `0..=6`, `yaw` is `0..=359`. */
export type Player = {
  readonly at: Vec3;
  readonly vel: Vec3;
  readonly grounded: boolean;
  readonly coyote: number;
  readonly jumpHeld: boolean;
  readonly riding: number | null;
  readonly yaw: number;
};

const still: Vec3 = Object.freeze({ x: 0, y: 0, z: 0 });

/** `Player(at)` with every default of the type. */
export const player = (at: Vec3, rest: Partial<Omit<Player, "at">> = {}): Player => ({
  at,
  vel: still,
  grounded: false,
  coyote: 0,
  jumpHeld: false,
  riding: null,
  yaw: 0,
  ...rest,
});
