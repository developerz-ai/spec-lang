// Projected by hand from the @cases of walk_target, facing, patrol (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { vec3 } from "../../src/geom/vec3.ts";
import type { Crawler } from "../../src/model/level.ts";
import { pad } from "../../src/model/pad.ts";
import { patrol } from "../../src/physics/patrol.ts";
import { facing, walkTarget } from "../../src/physics/walk.ts";

test.each([
  [pad(), vec3(0, 0, 0)],
  [pad({ right: true }), vec3(16, 0, 0)],
  [pad({ left: true, forward: true }), vec3(-11, 0, -11)],
  [pad({ left: true, right: true }), vec3(0, 0, 0)],
])("walk_target %#", (held, want) => {
  expect(walkTarget(held)).toEqual(want);
});

test.each([
  [pad(), 90, 90],
  [pad({ right: true }), 90, 0],
  [pad({ forward: true }), 0, 90],
  [pad({ left: true, back: true }), 0, 225],
])("facing %#", (held, was, want) => {
  expect(facing(held, was)).toBe(want);
});

const crawler = (x: number, dir: number): Crawler => ({ no: 1, at: vec3(x, 0, 0), lo: -10, hi: 10, dir, speed: 4 });

test.each([
  [crawler(0, 1), crawler(4, 1)],
  [crawler(8, 1), crawler(10, -1)],
])("patrol %#", (c, want) => {
  expect(patrol(c)).toEqual(want);
});
