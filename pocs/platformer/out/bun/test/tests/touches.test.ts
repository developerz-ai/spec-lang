// Projected by hand from hop.tests.touches (src/hop/tests/touches.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { meadow } from "../../src/levels/meadow.ts";
import { ridge } from "../../src/levels/ridge.ts";
import { pad } from "../../src/model/pad.ts";
import { start } from "../../src/world/start.ts";
import { step } from "../../src/world/step.ts";
import type { World } from "../../src/world/world.ts";
import { refusedOver, stepped, steps, worlds } from "../support/worlds.ts";

const rest = pad();

test("a_gem_is_taken_once", () => {
  // `out.text`: every line the test's rules printed, each followed by a newline.
  let text = "";
  const stepPrinting = (before: World): World => {
    const o = step(before, rest);
    if (!o.ok) throw new Error(`step refused: ${o.refusal}`);
    text += o.lines.map((l) => `${l}\n`).join("");
    return o.world;
  };
  let w = stepPrinting(worlds.at_the_third_gem);
  expect(w.taken).toEqual(new Set([3]));
  w = stepPrinting(w);
  expect(w.taken).toEqual(new Set([3]));
  expect(text).toBe("1 gem 3\n");
});

test("a_fall_keeps_the_gems", () => {
  const w = stepped(worlds.about_to_fall_out_holding_a_gem, rest);
  expect(w.lives).toBe(2);
  expect(w.taken).toEqual(new Set([1]));
});

test("rising_into_a_crawler_hurts", () => {
  const w = stepped(worlds.rising_beside_the_crawler, rest);
  expect(w.lives).toBe(2);
  expect(w.crawlers.some((c) => c.no === 1)).toBe(true);
});

test("a_stomp_removes_the_crawler", () => {
  const w = stepped(worlds.falling_on_the_crawler, rest);
  expect(w.crawlers.some((c) => c.no === 1)).toBe(false);
  expect(w.player.vel.y).toBe(24);
  expect(w.lives).toBe(3);
});

test("a_crawler_hurts_from_the_side", () => {
  const w = stepped(worlds.running_at_the_crawler, pad({ right: true }));
  expect(w.lives).toBe(2);
  expect(w.player.at).toEqual(meadow.start);
  expect(w.crawlers.some((c) => c.no === 1)).toBe(true);
});

test("falling_out_costs_a_life", () => {
  const w = stepped(worlds.about_to_fall_out, rest);
  expect(w.lives).toBe(2);
  expect(w.mode).toBe("Playing");
});

test("the_last_life_ends_the_game", () => {
  const w = stepped(worlds.about_to_fall_out_for_the_last_time, rest);
  expect(w.lives).toBe(0);
  expect(w.mode).toBe("Lost");
  refusedOver(w, rest);
});

test("reaching_the_flag_wins", () => {
  const w = stepped(worlds.beside_the_flag, rest);
  expect(w.mode).toBe("Won");
  refusedOver(w, rest);
});

test("starting_again_resets_everything", () => {
  // A rule returns a new world, so starting again is `start` alone; the five steps are played for the record.
  steps(start(1).world, pad({ right: true }), 5);
  const w = start(2).world;
  expect(w.taken).toEqual(new Set());
  expect(w.ticks).toBe(0);
  expect(w.crawlers.length).toBe(2);
  expect(w.player.at).toEqual(ridge.start);
});
