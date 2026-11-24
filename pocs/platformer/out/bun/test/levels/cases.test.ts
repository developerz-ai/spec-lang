// Projected by hand from the @cases of q (src/hop/levels.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { q } from "../../src/levels/grid.ts";

test.each([
  [4, 256],
  [-2, -128],
])("q(%i) -> %i", (n, want) => {
  expect(q(n)).toBe(want);
});
