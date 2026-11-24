// Projected by hand from @cases on line, and rank (src/todo/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { line } from "../../src/render/line.ts";
import { compareRank } from "../../src/render/rank.ts";
import { tags, task } from "../support/values.ts";

test.each([
  ["1 [ ] milk", task(1, "milk")],
  ["2 [x] ship !high #work", task(2, "ship", { status: "Done", priority: "High", tags: tags("work") })],
])("line -> %s", (want, t) => {
  expect(line(t)).toBe(want);
});

test("rank: most urgent first, then oldest first", () => {
  const ts = [task(4, "d"), task(1, "a", { priority: "Low" }), task(3, "c", { priority: "High" }), task(2, "b")];
  expect(ts.sort(compareRank).map((t) => Number(t.no))).toEqual([3, 2, 4, 1]);
});
