// Projected by hand from hop.tests.moves (src/hop/tests/moves.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { vec3 } from "../../src/geom/vec3.ts";
import { pad } from "../../src/model/pad.ts";
import { start } from "../../src/world/start.ts";
import { stepped, steps, worlds } from "../support/worlds.ts";

const right = pad({ right: true });
const jump = pad({ jump: true });
const rest = pad();

test("standing_still_stays_put", () => {
  const w = steps(start(1).world, rest, 30);
  expect(w.player.at).toEqual(vec3(0, 0, 0));
  expect(w.player.grounded).toBe(true);
  expect(w.ticks).toBe(30);
});

test("running_reaches_full_speed", () => {
  const w = steps(start(1).world, right, 10);
  expect(w.player.vel.x).toBe(16);
  expect(w.player.at.x).toBe(96);
  expect(w.player.yaw).toBe(0);
});

test("a_held_jump_peaks_at_380", () => {
  let top = 0;
  const w = steps(stepped(start(1).world, rest), jump, 50, (after) => {
    top = Math.max(top, after.player.at.y);
  });
  expect(top).toBe(380);
  expect(w.player.grounded).toBe(true);
});

test("a_short_hop_peaks_at_94", () => {
  let top = 0;
  steps(stepped(stepped(start(1).world, rest), jump), rest, 30, (after) => {
    top = Math.max(top, after.player.at.y);
  });
  expect(top).toBe(94);
});

test("a_jump_needs_a_fresh_press", () => {
  const w = steps(stepped(start(1).world, rest), jump, 60);
  expect(w.player.grounded).toBe(true);
  expect(w.player.at.y).toBe(0);
});

test("a_jump_just_after_the_edge_still_counts", () => {
  let w = stepped(worlds.past_the_first_ledge, rest);
  expect(w.player.grounded).toBe(false);
  w = stepped(w, jump);
  expect(w.player.vel.y).toBe(38);
});

test("a_jump_long_after_the_edge_does_not", () => {
  const w = stepped(steps(worlds.past_the_first_ledge, rest, 7), jump);
  expect(w.player.vel.y).toBeLessThan(0);
});

test("a_wall_stops_the_body_flush", () => {
  const w = steps(worlds.running_at_the_step, right, 10);
  expect(w.player.at.x).toBe(2496);
  expect(w.player.vel.x).toBe(0);
});

test("a_ceiling_stops_a_jump", () => {
  let top = 0;
  steps(worlds.under_a_low_ceiling, jump, 40, (after) => {
    top = Math.max(top, after.player.at.y);
  });
  expect(top).toBe(208);
});

test("a_rider_moves_with_the_mover", () => {
  const w = steps(worlds.riding_the_mover, rest, 10);
  expect(w.player.at).toEqual(vec3(1680, 0, 0));
  expect(w.player.riding).toBe(1);
});

test("a_mover_catches_a_falling_body", () => {
  const w = steps(worlds.above_the_mover, rest, 40);
  expect(w.player.riding).toBe(1);
  expect(w.player.at.y).toBe(0);
});

test("a_mover_never_catches_from_below", () => {
  const w = stepped(worlds.under_the_mover, rest);
  expect(w.player.at.y).toBeLessThan(-200);
  expect(w.player.riding).toBeNull();
});

test("a_fast_fall_lands", () => {
  const w = steps(worlds.falling_fast_onto_the_floor, rest, 12);
  expect(w.player.at.y).toBe(0);
  expect(w.player.grounded).toBe(true);
});
