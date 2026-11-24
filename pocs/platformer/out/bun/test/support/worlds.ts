// Projected by hand from the declared states and the named start worlds (the @given tests) (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { expect } from "bun:test";
import { meadow } from "../../src/levels/meadow.ts";
import { ridge } from "../../src/levels/ridge.ts";
import type { Crawler, Level } from "../../src/model/level.ts";
import type { Pad } from "../../src/model/pad.ts";
import { player } from "../../src/model/player.ts";
import { rgb } from "../../src/model/rgb.ts";
import { step } from "../../src/world/step.ts";
import type { World } from "../../src/world/world.ts";

/** The initial values the states declare. Only a `@given` builds on them; the program always calls `start`. */
export const declared: World = {
  stage: meadow,
  ticks: 0,
  player: player(meadow.start),
  taken: new Set(),
  crawlers: [],
  lives: 3,
  mode: "Playing",
};

const given = (states: Partial<World>): World => ({ ...declared, ...states });

const v = (x: number, y: number, z: number) => ({ x, y, z });

const lowRoom: Level = {
  name: "box",
  start: v(0, 0, 0),
  solids: [
    { box: { lo: v(-512, -256, -512), hi: v(512, 0, 512) }, color: rgb(0, 0, 0) },
    { box: { lo: v(-512, 400, -512), hi: v(512, 600, 512) }, color: rgb(0, 0, 0) },
  ],
  movers: [],
  gems: [],
  crawlers: [],
  goal: v(100000, 0, 0),
};

const midCrawler: Crawler = { no: 1, at: v(3072, 256, 0), lo: 2752, hi: 3392, dir: 1, speed: 2 };
const falling = player(v(1280, -2040, 0), { vel: v(0, -48, 0) });

export const worlds = {
  past_the_first_ledge: given({ stage: meadow, player: player(v(1090, 0, 0), { grounded: true, coyote: 6 }) }),
  running_at_the_step: given({ stage: meadow, player: player(v(2400, 0, 0), { vel: v(16, 0, 0), grounded: true }) }),
  under_a_low_ceiling: given({ stage: lowRoom, player: player(v(0, 0, 0), { grounded: true }) }),
  riding_the_mover: given({ stage: ridge, player: player(v(1600, 0, 0), { grounded: true, riding: 1 }) }),
  above_the_mover: given({ stage: ridge, player: player(v(1600, 200, 0)) }),
  at_the_third_gem: given({ stage: meadow, player: player(v(4608, 0, 0), { grounded: true }) }),
  falling_on_the_crawler: given({
    stage: meadow,
    crawlers: [midCrawler],
    player: player(v(3072, 400, 0), { vel: v(0, -20, 0) }),
  }),
  running_at_the_crawler: given({
    stage: meadow,
    crawlers: [midCrawler],
    player: player(v(2900, 256, 0), { vel: v(16, 0, 0), grounded: true }),
  }),
  about_to_fall_out: given({ stage: meadow, player: falling }),
  about_to_fall_out_for_the_last_time: given({ stage: meadow, lives: 1, player: falling }),
  beside_the_flag: given({ stage: meadow, player: player(v(5000, 0, 0), { grounded: true }) }),
  about_to_fall_out_holding_a_gem: given({ stage: meadow, taken: new Set([1]), player: falling }),
  rising_beside_the_crawler: given({
    stage: meadow,
    crawlers: [midCrawler],
    player: player(v(2920, 330, 0), { vel: v(0, 20, 0) }),
  }),
  under_the_mover: given({ stage: ridge, player: player(v(1600, -200, 0)) }),
  falling_fast_onto_the_floor: given({ stage: meadow, player: player(v(0, 400, 0), { vel: v(0, -48, 0) }) }),
} as const satisfies Record<string, World>;

/** `world.step(pad)` that must be accepted. */
export function stepped(w: World, pad: Pad): World {
  const o = step(w, pad);
  if (!o.ok) throw new Error(`step refused: ${o.refusal}`);
  return o.world;
}

/** `for i in 1..=n { world.step(pad); }`, calling `each` with the world after every step. */
export function steps(w: World, pad: Pad, n: number, each: (after: World) => void = () => {}): World {
  let world = w;
  for (let i = 0; i < n; i++) {
    world = stepped(world, pad);
    each(world);
  }
  return world;
}

/** `world.step(pad) -> fail Over` */
export function refusedOver(w: World, pad: Pad): void {
  expect(step(w, pad)).toEqual({ ok: false, refusal: "Over" });
}
