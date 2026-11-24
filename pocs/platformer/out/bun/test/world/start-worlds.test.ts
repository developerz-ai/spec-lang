// Projected by hand from the start worlds past_the_first_ledge .. falling_fast_onto_the_floor (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { overlaps } from "../../src/geom/box.ts";
import { body } from "../../src/physics/bodies.ts";
import type { World } from "../../src/world/world.ts";
import { worlds } from "../support/worlds.ts";

/** A start world has an empty body: it passes when its givens hold the claims of their types. */
function holds(w: World): void {
  expect(w.mode).toBe("Playing");
  expect(w.lives).toBeWithin(0, 4);
  expect(w.player.coyote).toBeWithin(0, 7);
  expect(w.player.yaw).toBeWithin(0, 360);
  for (const c of w.crawlers) {
    expect(c.dir).toBeWithin(-1, 2);
    expect(c.speed).toBeWithin(1, 9);
  }
  expect(w.stage.solids.some((s) => overlaps(body(w.player.at), s.box))).toBe(false);
}

test("past_the_first_ledge", () => holds(worlds.past_the_first_ledge));
test("running_at_the_step", () => holds(worlds.running_at_the_step));
test("under_a_low_ceiling", () => holds(worlds.under_a_low_ceiling));
test("riding_the_mover", () => holds(worlds.riding_the_mover));
test("above_the_mover", () => holds(worlds.above_the_mover));
test("at_the_third_gem", () => holds(worlds.at_the_third_gem));
test("falling_on_the_crawler", () => holds(worlds.falling_on_the_crawler));
test("running_at_the_crawler", () => holds(worlds.running_at_the_crawler));
test("about_to_fall_out", () => holds(worlds.about_to_fall_out));
test("about_to_fall_out_for_the_last_time", () => holds(worlds.about_to_fall_out_for_the_last_time));
test("beside_the_flag", () => holds(worlds.beside_the_flag));
test("about_to_fall_out_holding_a_gem", () => holds(worlds.about_to_fall_out_holding_a_gem));
test("rising_beside_the_crawler", () => holds(worlds.rising_beside_the_crawler));
test("under_the_mover", () => holds(worlds.under_the_mover));
test("falling_fast_onto_the_floor", () => holds(worlds.falling_fast_onto_the_floor));
