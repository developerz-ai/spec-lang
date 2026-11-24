// Projected by hand from the eight laws (src/hop/laws.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import fc from "fast-check";
import { overlaps } from "../../src/geom/box.ts";
import { body } from "../../src/physics/bodies.ts";
import { jumpSpeed, maxFall, runSpeed } from "../../src/physics/constants.ts";
import type { World } from "../../src/world/world.ts";
import { replay, SEED, type Step, TRACES, trace } from "./trace.ts";

const options = { seed: SEED, numRuns: TRACES, endOnFailure: false };

const noStep = (): void => {};
const noState = (): void => {};

/** A law over `t.states`: it must hold of every world of every trace. */
function states(name: string, holds: (w: World) => boolean): void {
  test(name, () => {
    fc.assert(
      fc.property(trace, (plays) => {
        replay(
          plays,
          (w) => {
            if (!holds(w)) throw new Error(`${name} does not hold at tick ${w.ticks}`);
          },
          noStep,
        );
      }),
      options,
    );
  });
}

/** A law over `t.steps`: it must hold of every step of every trace. */
function steps(name: string, holds: (s: Step) => boolean): void {
  test(name, () => {
    fc.assert(
      fc.property(trace, (plays) => {
        replay(plays, noState, (s) => {
          if (!holds(s)) throw new Error(`${name} does not hold stepping from tick ${s.before.ticks}`);
        });
      }),
      options,
    );
  });
}

states("never_inside_a_solid", (w) => w.stage.solids.every((s) => !overlaps(body(w.player.at), s.box)));

states(
  "speed_is_bounded",
  (w) =>
    Math.abs(w.player.vel.x) <= runSpeed &&
    Math.abs(w.player.vel.z) <= runSpeed &&
    -maxFall <= w.player.vel.y &&
    w.player.vel.y <= jumpSpeed,
);

steps("taken_only_grows", (s) => [...s.before.taken].every((g) => s.after.taken.has(g)));

steps("lives_never_rise", (s) => s.after.lives <= s.before.lives);

states("lost_means_no_lives", (w) => (w.lives === 0) === (w.mode === "Lost"));

steps("over_is_final", (s) => s.before.mode === "Playing" || (!s.reply.ok && s.after === s.before));

steps("one_tick_per_step", (s) => !s.reply.ok || s.after.ticks === s.before.ticks + 1);

states("taken_are_real_gems", (w) => [...w.taken].every((n) => w.stage.gems.some((g) => g.no === n)));

test("the traces reach what the laws are about", () => {
  const seen = { steps: 0, gem: 0, stomp: 0, hurt: 0, riding: 0, lost: 0, won: 0, refused: 0 };
  fc.assert(
    fc.property(trace, (plays) => {
      replay(plays, noState, (s) => {
        seen.steps++;
        if (s.after.taken.size > s.before.taken.size) seen.gem++;
        if (s.after.crawlers.length < s.before.crawlers.length) seen.stomp++;
        if (s.after.lives < s.before.lives) seen.hurt++;
        if (s.after.player.riding !== null) seen.riding++;
        if (s.before.mode === "Playing" && s.after.mode === "Lost") seen.lost++;
        if (s.before.mode === "Playing" && s.after.mode === "Won") seen.won++;
        if (!s.reply.ok) seen.refused++;
      });
    }),
    options,
  );
  console.log(`law traces: ${JSON.stringify(seen)}`);
  for (const [what, count] of Object.entries(seen)) expect(count, what).toBeGreaterThan(0);
});
