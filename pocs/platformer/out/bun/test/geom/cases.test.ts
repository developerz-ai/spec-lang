// Projected by hand from the @cases of plus, around_feet, overlaps, touches, triangle, approach, vec_text (src/hop/geom.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { aroundFeet, box, overlaps, touches } from "../../src/geom/box.ts";
import { approach, triangle } from "../../src/geom/scalar.ts";
import { plus, vec3, vecText } from "../../src/geom/vec3.ts";

test("plus", () => {
  expect(plus(vec3(1, 2, 3), vec3(10, 20, 30))).toEqual(vec3(11, 22, 33));
});

test("around_feet", () => {
  expect(aroundFeet(vec3(0, 0, 0), vec3(64, 192, 64))).toEqual(box(vec3(-64, 0, -64), vec3(64, 192, 64)));
});

const cube = box(vec3(0, 0, 0), vec3(2, 2, 2));

test.each([
  [cube, box(vec3(1, 1, 1), vec3(3, 3, 3)), true],
  [cube, box(vec3(2, 0, 0), vec3(4, 2, 2)), false],
])("overlaps %#", (a, b, want) => {
  expect(overlaps(a, b)).toBe(want);
});

test.each([
  [cube, box(vec3(2, 0, 0), vec3(4, 2, 2)), true],
  [cube, box(vec3(3, 0, 0), vec3(4, 2, 2)), false],
])("touches %#", (a, b, want) => {
  expect(touches(a, b)).toBe(want);
});

test.each([
  [0, 4, 0],
  [3, 4, 3],
  [4, 4, 4],
  [5, 4, 3],
  [8, 4, 0],
  [9, 4, 1],
])("triangle(%i, %i) -> %i", (t, n, want) => {
  expect(triangle(t, n)).toBe(want);
});

test.each([
  [0, 3, 2, 2],
  [5, 3, 2, 3],
  [5, 0, 2, 3],
  [-5, 0, 1, -4],
])("approach(%i, %i, %i) -> %i", (v, target, step, want) => {
  expect(approach(v, target, step)).toBe(want);
});

test("vec_text", () => {
  expect(vecText(vec3(1, -2, 3))).toBe("1,-2,3");
});
