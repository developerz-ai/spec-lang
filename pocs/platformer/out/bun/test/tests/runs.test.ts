// Projected by hand from hop.tests.runs (src/hop/tests/runs.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { pad } from "../../src/model/pad.ts";
import { start } from "../../src/world/start.ts";
import { step } from "../../src/world/step.ts";
import type { World } from "../../src/world/world.ts";
import { meadowWin, padOf, ridgeWin } from "../support/scripts.ts";
import { stepped } from "../support/worlds.ts";

test.each([
  ["-", pad()],
  ["RJ", pad({ right: true, jump: true })],
  ["LFB", pad({ left: true, forward: true, back: true })],
])("pad_of(%s)", (keys, want) => {
  expect(padOf(keys)).toEqual(want);
});

test("the_meadow_can_be_won", () => {
  let w: World = start(1).world;
  for (const [ticks, keys] of meadowWin) {
    for (let i = 1; i <= ticks; i++) w = stepped(w, padOf(keys));
  }
  expect(w.mode).toBe("Won");
  expect(w.ticks).toBe(316);
  expect(w.taken.size).toBe(3);
  expect(w.lives).toBe(3);
});

test("the_ridge_can_be_won", () => {
  let w: World = start(2).world;
  for (const [ticks, keys] of ridgeWin) {
    for (let i = 1; i <= ticks; i++) {
      if (w.mode === "Playing") {
        const o = step(w, padOf(keys));
        if (o.ok) w = o.world;
      }
    }
  }
  expect(w.mode).toBe("Won");
  expect(w.ticks).toBe(549);
  expect(w.taken.size).toBe(3);
  expect(w.lives).toBe(3);
});
