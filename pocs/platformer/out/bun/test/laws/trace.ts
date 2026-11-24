// Projected by hand from Trace[Play]: seeded traces of world.start and world.step (src/hop/laws.slg, docs/contract.md section 6). Stand-in for spec-lang build output; change the spec, not this file.

import fc from "fast-check";
import type { LevelNo } from "../../src/levels/level.ts";
import type { Pad } from "../../src/model/pad.ts";
import type { Outcome } from "../../src/world/outcome.ts";
import { start } from "../../src/world/start.ts";
import { step } from "../../src/world/step.ts";
import type { World } from "../../src/world/world.ts";
import { type Hold, meadowWin, padOf, ridgeWin } from "../support/scripts.ts";

export const SEED = 20261124;
export const TRACES = 200;
/**
 * The contract asks for at least 200 ticks. From `start`, no game can end in 200: the first fall takes about 120
 * ticks and three are needed to lose, and the shortest win is 316. A 200-tick trace would leave `over_is_final`
 * with nothing to check, so traces are 700 events long.
 */
export const TICKS = 700;

export type Play = { readonly rule: "start"; readonly level: LevelNo } | { readonly rule: "step"; readonly pad: Pad };

/** One `world.step` of a trace: the worlds either side and what the rule said. */
export type Step = { readonly before: World; readonly after: World; readonly reply: Outcome };

const levelNo: fc.Arbitrary<LevelNo> = fc.constantFrom(1, 2);
const oneIn = (n: number, of: number): fc.Arbitrary<boolean> => fc.integer({ min: 1, max: of }).map((k) => k <= n);

/** Leans right and jumps often, so a trace leaves the first platform, meets crawlers, falls and loses. */
const pad: fc.Arbitrary<Pad> = fc.record({
  left: oneIn(2, 10),
  right: oneIn(7, 10),
  forward: oneIn(1, 10),
  back: oneIn(1, 10),
  jump: oneIn(4, 10),
});

/** A pad held for a while, as a hand holds it; now and then the level starts again instead. */
const held: fc.Arbitrary<readonly Play[]> = fc.oneof(
  {
    weight: 40,
    arbitrary: fc
      .tuple(pad, fc.integer({ min: 1, max: 30 }))
      .map(([p, ticks]): Play[] => Array.from({ length: ticks }, () => ({ rule: "step", pad: p }))),
  },
  { weight: 1, arbitrary: levelNo.map((level): Play[] => [{ rule: "start", level }]) },
);

const expand = (script: readonly Hold[]): Play[] =>
  script.flatMap(([ticks, keys]) => Array.from({ length: ticks }, (): Play => ({ rule: "step", pad: padOf(keys) })));

const winning: Record<LevelNo, readonly Play[]> = { 1: expand(meadowWin), 2: expand(ridgeWin) };

/**
 * `start` of a random level, then random held pads, 700 events in all. Half the traces first follow the level's
 * winning script for a random number of ticks, so gems, stomps, the mover and a won game are reached too.
 */
export const trace: fc.Arbitrary<readonly Play[]> = fc
  .tuple(levelNo, fc.boolean(), fc.nat(), fc.array(held, { minLength: TICKS, maxLength: TICKS }))
  .map(([level, follow, cut, rest]): readonly Play[] => {
    const lead = follow ? winning[level].slice(0, cut % (winning[level].length + 1)) : [];
    return [{ rule: "start", level } as Play, ...lead, ...rest.flat()].slice(0, TICKS + 1);
  });

/** Plays a trace. `state` sees every world of the trace (`t.states`), `stepped` every step (`t.steps`). */
export function replay(plays: readonly Play[], state: (w: World) => void, stepped: (s: Step) => void): void {
  let world: World | undefined;
  for (const p of plays) {
    if (p.rule === "start") {
      world = start(p.level).world;
    } else {
      const before = world as World;
      const reply = step(before, p.pad);
      const after = reply.ok ? reply.world : before;
      stepped({ before, after, reply });
      world = after;
    }
    state(world);
  }
}
